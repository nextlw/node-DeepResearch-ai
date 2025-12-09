//! # Paytour Tools
//!
//! Este módulo implementa a integração com a API Paytour para
//! gerenciamento de passeios turísticos.
//!
//! ## Funcionalidades
//!
//! - Listar passeios disponíveis
//! - Detalhar informações de um passeio
//! - Verificar disponibilidade por data
//! - Obter horários disponíveis

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;

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

/// Configuração do cliente Paytour.
#[derive(Debug, Clone)]
pub struct PaytourConfig {
    /// URL base da API.
    pub base_url: String,
    
    /// Chave de API (se aplicável).
    pub api_key: Option<String>,
    
    /// Token de autenticação (se aplicável).
    pub auth_token: Option<String>,
    
    /// Timeout em segundos.
    pub timeout_secs: u64,
}

impl Default for PaytourConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.paytour.com.br".to_string(),
            api_key: None,
            auth_token: None,
            timeout_secs: 30,
        }
    }
}

/// Filtros para busca de passeios.
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

/// Representação de um passeio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passeio {
    /// ID único do passeio.
    pub id: u64,
    
    /// Nome do passeio.
    pub nome: String,
    
    /// Descrição do passeio.
    #[serde(default)]
    pub descricao: String,
    
    /// Preço base.
    #[serde(default)]
    pub preco: f64,
    
    /// Duração em horas.
    #[serde(default)]
    pub duracao_horas: Option<f64>,
    
    /// URL da imagem principal.
    #[serde(default)]
    pub imagem_url: Option<String>,
    
    /// Cidade do passeio.
    #[serde(default)]
    pub cidade: Option<String>,
    
    /// Categoria do passeio.
    #[serde(default)]
    pub categoria: Option<String>,
    
    /// Avaliação média (0-5).
    #[serde(default)]
    pub avaliacao: Option<f32>,
    
    /// Número de avaliações.
    #[serde(default)]
    pub num_avaliacoes: Option<u32>,
    
    /// Se está ativo.
    #[serde(default = "default_true")]
    pub ativo: bool,
}

fn default_true() -> bool {
    true
}

/// Detalhes completos de um passeio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasseioDetalhes {
    /// Informações básicas do passeio.
    #[serde(flatten)]
    pub passeio: Passeio,
    
    /// O que está incluído.
    #[serde(default)]
    pub incluso: Vec<String>,
    
    /// O que não está incluído.
    #[serde(default)]
    pub nao_incluso: Vec<String>,
    
    /// Políticas de cancelamento.
    #[serde(default)]
    pub politica_cancelamento: Option<String>,
    
    /// Ponto de encontro.
    #[serde(default)]
    pub ponto_encontro: Option<String>,
    
    /// Coordenadas (lat, lng).
    #[serde(default)]
    pub coordenadas: Option<(f64, f64)>,
    
    /// Galeria de imagens.
    #[serde(default)]
    pub galeria: Vec<String>,
    
    /// FAQs.
    #[serde(default)]
    pub faqs: Vec<FAQ>,
}

/// Pergunta frequente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FAQ {
    /// Pergunta.
    pub pergunta: String,
    
    /// Resposta.
    pub resposta: String,
}

/// Disponibilidade para uma data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disponibilidade {
    /// Data (YYYY-MM-DD).
    pub data: String,
    
    /// Se está disponível.
    pub disponivel: bool,
    
    /// Vagas disponíveis.
    #[serde(default)]
    pub vagas: Option<u32>,
    
    /// Preço para esta data (pode variar).
    #[serde(default)]
    pub preco: Option<f64>,
}

/// Horário disponível.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Horario {
    /// ID do horário.
    pub id: String,
    
    /// Hora de início (HH:MM).
    pub hora_inicio: String,
    
    /// Hora de fim (HH:MM).
    #[serde(default)]
    pub hora_fim: Option<String>,
    
    /// Vagas disponíveis.
    #[serde(default)]
    pub vagas: Option<u32>,
    
    /// Preço para este horário.
    #[serde(default)]
    pub preco: Option<f64>,
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
    /// Cliente HTTP.
    client: Client,
    
    /// Configuração.
    config: PaytourConfig,
}

impl PaytourTools {
    /// Cria um novo cliente Paytour.
    ///
    /// Tenta carregar configuração do ambiente.
    pub async fn new() -> Result<Self, PaytourError> {
        let config = Self::load_config_from_env()?;
        Self::with_config(config).await
    }
    
    /// Cria um novo cliente Paytour com configuração customizada.
    pub async fn with_config(config: PaytourConfig) -> Result<Self, PaytourError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| PaytourError::NetworkError(e.to_string()))?;
        
        Ok(Self { client, config })
    }
    
    /// Carrega configuração do ambiente.
    fn load_config_from_env() -> Result<PaytourConfig, PaytourError> {
        let base_url = std::env::var("PAYTOUR_BASE_URL")
            .unwrap_or_else(|_| "https://api.paytour.com.br".to_string());
        
        let api_key = std::env::var("PAYTOUR_API_KEY").ok();
        let auth_token = std::env::var("PAYTOUR_AUTH_TOKEN").ok();
        
        Ok(PaytourConfig {
            base_url,
            api_key,
            auth_token,
            timeout_secs: 30,
        })
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
        let url = format!("{}/api/v1/passeios", self.config.base_url);
        
        let mut request = self.client.get(&url);
        
        // Adiciona headers de autenticação
        request = self.add_auth_headers(request);
        
        // Adiciona query params se houver filtros
        if let Some(f) = filtros {
            let params = serde_json::to_value(&f)
                .map_err(|e| PaytourError::ApiError(e.to_string()))?;
            
            if let serde_json::Value::Object(map) = params {
                for (key, value) in map {
                    if let serde_json::Value::String(s) = value {
                        request = request.query(&[(key, s)]);
                    } else if let serde_json::Value::Number(n) = value {
                        request = request.query(&[(key, n.to_string())]);
                    }
                }
            }
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| PaytourError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    /// Obtém detalhes de um passeio específico.
    ///
    /// # Argumentos
    /// * `id` - ID do passeio.
    ///
    /// # Retorna
    /// JSON com os detalhes do passeio.
    pub async fn detalhar_passeio(&self, id: u64) -> Result<serde_json::Value, PaytourError> {
        let url = format!("{}/api/v1/passeios/{}", self.config.base_url, id);
        
        let request = self.add_auth_headers(self.client.get(&url));
        
        let response = request
            .send()
            .await
            .map_err(|e| PaytourError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
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
        let url = format!(
            "{}/api/v1/passeios/{}/disponibilidade",
            self.config.base_url, id
        );
        
        let request = self
            .add_auth_headers(self.client.get(&url))
            .query(&[("mes", mes.to_string()), ("ano", ano.to_string())]);
        
        let response = request
            .send()
            .await
            .map_err(|e| PaytourError::NetworkError(e.to_string()))?;
        
        let json = self.handle_response(response).await?;
        
        // Extrai datas disponíveis do JSON
        let datas = json
            .get("datas")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let disponivel = v.get("disponivel")?.as_bool()?;
                        if disponivel {
                            v.get("data")?.as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        
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
        let url = format!(
            "{}/api/v1/passeios/{}/horarios",
            self.config.base_url, id
        );
        
        let request = self
            .add_auth_headers(self.client.get(&url))
            .query(&[("data", dia)]);
        
        let response = request
            .send()
            .await
            .map_err(|e| PaytourError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    /// Adiciona headers de autenticação ao request.
    fn add_auth_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut req = request;
        
        if let Some(ref api_key) = self.config.api_key {
            req = req.header("X-API-Key", api_key);
        }
        
        if let Some(ref token) = self.config.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        
        req
    }
    
    /// Processa a resposta HTTP.
    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<serde_json::Value, PaytourError> {
        let status = response.status();
        
        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| PaytourError::ApiError(format!("Failed to parse response: {}", e)))
        } else if status.as_u16() == 401 {
            Err(PaytourError::AuthError("Invalid credentials".into()))
        } else if status.as_u16() == 404 {
            Err(PaytourError::NotFound("Resource not found".into()))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(PaytourError::ApiError(format!(
                "API error ({}): {}",
                status, error_text
            )))
        }
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
    fn test_config_default() {
        let config = PaytourConfig::default();
        assert!(config.base_url.contains("paytour"));
        assert!(config.api_key.is_none());
    }
    
    #[test]
    fn test_passeio_query_serialization() {
        let query = PasseioQuery {
            cidade_id: Some(1),
            categoria_id: None,
            data_inicio: Some("2024-01-01".into()),
            ..Default::default()
        };
        
        let json = serde_json::to_value(&query).unwrap();
        assert!(json.get("cidade_id").is_some());
        assert!(json.get("categoria_id").is_none()); // skip_serializing_if
    }
    
    #[test]
    fn test_passeio_deserialization() {
        let json = r#"{
            "id": 123,
            "nome": "Passeio de Barco",
            "preco": 150.0
        }"#;
        
        let passeio: Passeio = serde_json::from_str(json).unwrap();
        assert_eq!(passeio.id, 123);
        assert_eq!(passeio.nome, "Passeio de Barco");
        assert_eq!(passeio.preco, 150.0);
        assert!(passeio.ativo); // default
    }
}
