//! # Paytour Tools
//!
//! Este módulo implementa a integração com a API Paytour para
//! gerenciamento de passeios turísticos.
//!
//! Utiliza o crate `paytour` do crates.io para comunicação com a API.
//!
//! ## Funcionalidades
//!
//! - Listar passeios disponíveis
//! - Detalhar informações de um passeio
//! - Verificar disponibilidade por data
//! - Obter horários disponíveis

use async_trait::async_trait;
use paytour::{
    auth::PaytourAuthenticator,
    client::{PaytourClient, PasseioQuery as CratePasseioQuery},
    config::EnvManager,
    error::PaytourError as CratePaytourError,
};
use serde::{Deserialize, Serialize};
use serde_json;

/// Erro que pode ocorrer nas operações Paytour.
#[derive(Debug, thiserror::Error)]
pub enum PaytourError {
    /// Erro de autenticação.
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Erro de rede.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Erro de API.
    #[error("API error: {0}")]
    ApiError(String),

    /// Recurso não encontrado.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Erro de configuração.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<CratePaytourError> for PaytourError {
    fn from(err: CratePaytourError) -> Self {
        match err {
            CratePaytourError::ApiError { status, message } => {
                if status == 401 {
                    PaytourError::AuthError(message)
                } else if status == 404 {
                    PaytourError::NotFound(message)
                } else {
                    PaytourError::ApiError(format!("API error ({}): {}", status, message))
                }
            }
            CratePaytourError::Http(e) => PaytourError::NetworkError(e.to_string()),
            CratePaytourError::ConfigError(msg) => PaytourError::ConfigError(msg),
            CratePaytourError::Serialization(e) => PaytourError::ApiError(format!("Serialization error: {}", e)),
            CratePaytourError::Io(e) => PaytourError::ApiError(format!("IO error: {}", e)),
        }
    }
}

/// Filtros para busca de passeios.
/// Mantido para compatibilidade com a interface existente.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PasseioQuery {
    /// ID da cidade.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cidade_id: Option<u64>,

    /// ID da categoria.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categoria_id: Option<u64>,

    /// Data de início (YYYY-MM-DD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_inicio: Option<String>,

    /// Data de fim (YYYY-MM-DD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_fim: Option<String>,

    /// Preço mínimo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preco_min: Option<f64>,

    /// Preço máximo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preco_max: Option<f64>,

    /// Termo de busca.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub busca: Option<String>,

    /// Página (paginação).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagina: Option<u32>,

    /// Limite por página.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limite: Option<u32>,
}

impl From<PasseioQuery> for CratePasseioQuery {
    fn from(query: PasseioQuery) -> Self {
        CratePasseioQuery {
            pagina: query.pagina,
            quantidade: query.limite,
            busca: query.busca,
            nome: None, // Não há mapeamento direto
            destino_id: query.cidade_id.map(|id| id.to_string()), // Assumindo que cidade_id pode ser usado como destino_id
            data_de: query.data_inicio,
            data_ate: query.data_fim,
            minimal_response: None,
        }
    }
}

/// Cliente para integração com Paytour.
///
/// # Exemplo
/// ```rust,ignore
/// use deep_research::integrations::PaytourTools;
///
/// let tools = PaytourTools::new().await?;
///
/// // Listar passeios
/// let passeios = tools.listar_passeios(None).await?;
///
/// // Detalhar passeio
/// let detalhes = tools.detalhar_passeio(123).await?;
///
/// // Verificar disponibilidade
/// let datas = tools.verificar_disponibilidade(123, 12, 2024).await?;
/// ```
pub struct PaytourTools {
    /// Cliente Paytour do crate.
    client: PaytourClient,
}

impl PaytourTools {
    /// Cria um novo cliente Paytour.
    ///
    /// Tenta carregar configuração do ambiente e autenticar usando `EnvManager` e `PaytourAuthenticator` do crate.
    pub async fn new() -> Result<Self, PaytourError> {
        let env = EnvManager::load()?;
        let authenticator = PaytourAuthenticator::new(&env);

        // Verifica se o token está expirado ou não existe
        let token = if EnvManager::is_token_expired() || EnvManager::get_cached_access_token().is_none() {
            // Autentica usando a estratégia configurada
            let token_response = authenticator.authenticate(env.auth_strategy()).await?;
            token_response.access_token
        } else {
            // Usa o token em cache
            EnvManager::get_cached_access_token().ok_or_else(|| {
                PaytourError::ConfigError("Token de acesso não encontrado".into())
            })?
        };

        let client = PaytourClient::new(env.api_base_url, token);

        Ok(Self { client })
    }

    /// Cria um novo cliente Paytour com configuração customizada.
    pub async fn with_config(base_url: String, token: String) -> Result<Self, PaytourError> {
        let client = PaytourClient::new(base_url, token);
        Ok(Self { client })
    }

    /// Lista passeios disponíveis.
    ///
    /// # Argumentos
    /// * `filtros` - Filtros opcionais para a busca.
    ///
    /// # Retorna
    /// JSON com a lista de passeios.
    pub async fn listar_passeios(
        &self,
        filtros: Option<PasseioQuery>,
    ) -> Result<serde_json::Value, PaytourError> {
        let query = filtros.map(Into::into).unwrap_or_default();
        let response = self.client.list_passeios(&query).await?;

        Ok(serde_json::to_value(response).map_err(|e| {
            PaytourError::ApiError(format!("Failed to serialize response: {}", e))
        })?)
    }

    /// Obtém detalhes de um passeio específico.
    ///
    /// # Argumentos
    /// * `id` - ID do passeio.
    ///
    /// # Retorna
    /// JSON com os detalhes do passeio.
    pub async fn detalhar_passeio(&self, id: u64) -> Result<serde_json::Value, PaytourError> {
        let detalhes = self.client.get_passeio_detail(id, None).await?;

        Ok(serde_json::to_value(detalhes).map_err(|e| {
            PaytourError::ApiError(format!("Failed to serialize response: {}", e))
        })?)
    }

    /// Verifica disponibilidade de um passeio para um mês específico.
    ///
    /// # Argumentos
    /// * `id` - ID do passeio.
    /// * `mes` - Mês (1-12).
    /// * `ano` - Ano (ex: 2024).
    ///
    /// # Retorna
    /// Lista de datas disponíveis no formato YYYY-MM-DD.
    pub async fn verificar_disponibilidade(
        &self,
        id: u64,
        mes: u8,
        ano: u32,
    ) -> Result<Vec<String>, PaytourError> {
        let datas = self.client.get_dias_disponiveis(id, mes, ano).await?;
        Ok(datas)
    }

    /// Obtém horários disponíveis para um passeio em uma data específica.
    ///
    /// # Argumentos
    /// * `id` - ID do passeio.
    /// * `dia` - Data no formato YYYY-MM-DD.
    ///
    /// # Retorna
    /// JSON com os horários disponíveis.
    pub async fn obter_horarios(
        &self,
        id: u64,
        dia: &str,
    ) -> Result<serde_json::Value, PaytourError> {
        let horarios = self.client.get_horarios_disponiveis(id, dia).await?;

        Ok(serde_json::to_value(horarios).map_err(|e| {
            PaytourError::ApiError(format!("Failed to serialize response: {}", e))
        })?)
    }
}

/// Trait para ferramentas do agente.
///
/// Define a interface que as ferramentas devem implementar
/// para serem usadas pelo agente de pesquisa.
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Nome da ferramenta.
    fn name(&self) -> &'static str;

    /// Descrição da ferramenta.
    fn description(&self) -> &'static str;

    /// Executa a ferramenta com os parâmetros fornecidos.
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, String>;
}

#[async_trait]
impl AgentTool for PaytourTools {
    fn name(&self) -> &'static str {
        "paytour"
    }

    fn description(&self) -> &'static str {
        "Ferramenta para consultar passeios turísticos via Paytour. \
        Permite listar passeios, ver detalhes, verificar disponibilidade e horários."
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let action = params.get("action")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'action' parameter")?;

        match action {
            "listar" => {
                let filtros: Option<PasseioQuery> = params.get("filtros")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                self.listar_passeios(filtros)
                    .await
                    .map_err(|e| e.to_string())
            }
            "detalhar" => {
                let id = params.get("id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing 'id' parameter")?;

                self.detalhar_passeio(id)
                    .await
                    .map_err(|e| e.to_string())
            }
            "disponibilidade" => {
                let id = params.get("id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing 'id' parameter")?;
                let mes = params.get("mes")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing 'mes' parameter")? as u8;
                let ano = params.get("ano")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing 'ano' parameter")? as u32;

                let datas = self.verificar_disponibilidade(id, mes, ano)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::json!({ "datas_disponiveis": datas }))
            }
            "horarios" => {
                let id = params.get("id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing 'id' parameter")?;
                let dia = params.get("dia")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'dia' parameter")?;

                self.obter_horarios(id, dia)
                    .await
                    .map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown action: {}", action)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passeio_query_conversion() {
        let query = PasseioQuery {
            cidade_id: Some(1),
            categoria_id: None,
            data_inicio: Some("2024-01-01".into()),
            ..Default::default()
        };

        let crate_query: CratePasseioQuery = query.into();
        assert_eq!(crate_query.destino_id, Some("1".to_string()));
        assert_eq!(crate_query.data_de, Some("2024-01-01".to_string()));
    }

    #[test]
    fn test_error_conversion() {
        let crate_error = CratePaytourError::ConfigError("test".into());
        let our_error: PaytourError = crate_error.into();
        assert!(matches!(our_error, PaytourError::ConfigError(_)));
    }
}
