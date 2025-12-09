//! # Digisac Tools
//!
//! Este módulo implementa a integração com a API Digisac para
//! gerenciamento de mensagens via WhatsApp e outros canais.
//!
//! Utiliza o crate `digisac` do crates.io para comunicação com a API.
//!
//! ## Funcionalidades
//!
//! - Enviar mensagens
//! - Listar webhooks
//! - Criar webhooks
//! - Gerenciar contatos

use async_trait::async_trait;
use digisac::{
    client::{DigisacApi, DigisacClient, SendMessageRequest as DigisacSendMessageRequest, WebhookCreateRequest},
    config::EnvManager,
    error::DigisacError as CrateDigisacError,
};
use serde_json;

use super::paytour_tools::AgentTool;

/// Erro que pode ocorrer nas operações Digisac.
#[derive(Debug, thiserror::Error)]
pub enum DigisacError {
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

    /// Erro de validação.
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<CrateDigisacError> for DigisacError {
    fn from(err: CrateDigisacError) -> Self {
        match err {
            CrateDigisacError::ApiError { status, message } => {
                if status == 401 {
                    DigisacError::AuthError(message)
                } else if status == 404 {
                    DigisacError::NotFound(message)
                } else if status == 422 {
                    DigisacError::ValidationError(message)
                } else {
                    DigisacError::ApiError(format!("API error ({}): {}", status, message))
                }
            }
            CrateDigisacError::NetworkError(e) => DigisacError::NetworkError(e.to_string()),
            CrateDigisacError::ConfigError(msg) => DigisacError::ConfigError(msg),
            CrateDigisacError::Unknown(msg) => DigisacError::ApiError(msg),
            CrateDigisacError::AuthError => DigisacError::AuthError("Authentication failed".into()),
            CrateDigisacError::ParseError(e) => DigisacError::ApiError(format!("Parse error: {}", e)),
        }
    }
}

/// Cliente para integração com Digisac.
///
/// # Exemplo
/// ```rust,ignore
/// use deep_research::integrations::DigisacTools;
///
/// let tools = DigisacTools::new().await?;
///
/// // Enviar mensagem
/// let response = tools.enviar_mensagem(
///     "service_123",
///     "contact_456",
///     "Olá, como posso ajudar?",
/// ).await?;
///
/// // Listar webhooks
/// let webhooks = tools.listar_webhooks().await?;
/// ```
pub struct DigisacTools {
    /// Cliente Digisac do crate.
    client: DigisacClient,
}

impl DigisacTools {
    /// Cria um novo cliente Digisac.
    ///
    /// Tenta carregar configuração do ambiente usando `EnvManager` do crate.
    pub async fn new() -> Result<Self, DigisacError> {
        let env = EnvManager::load().map_err(|e| DigisacError::ConfigError(e.to_string()))?;

        let token = env.access_token.ok_or_else(|| {
            DigisacError::ConfigError("DIGISAC_ACCESS_TOKEN não encontrado".into())
        })?;

        let client = DigisacClient::new(env.api_base_url, token);

        Ok(Self { client })
    }

    /// Cria um novo cliente Digisac com configuração customizada.
    pub async fn with_config(base_url: String, token: String) -> Result<Self, DigisacError> {
        let client = DigisacClient::new(base_url, token);
        Ok(Self { client })
    }

    /// Envia uma mensagem de texto.
    ///
    /// # Argumentos
    /// * `service_id` - ID do serviço (canal).
    /// * `contact_id` - ID do contato.
    /// * `texto` - Texto da mensagem.
    ///
    /// # Retorna
    /// JSON com a resposta da API.
    pub async fn enviar_mensagem(
        &self,
        service_id: &str,
        contact_id: &str,
        texto: &str,
    ) -> Result<serde_json::Value, DigisacError> {
        // Validações
        if texto.is_empty() {
            return Err(DigisacError::ValidationError("Message text cannot be empty".into()));
        }
        if texto.len() > 4096 {
            return Err(DigisacError::ValidationError("Message text too long (max 4096 chars)".into()));
        }

        let request = DigisacSendMessageRequest::builder()
            .service_id(service_id.to_string())
            .contact_id(contact_id.to_string())
            .text(texto.to_string())
            .build();

        let response = self.client.send_message(&request).await?;

        Ok(serde_json::to_value(response).map_err(|e| {
            DigisacError::ApiError(format!("Failed to serialize response: {}", e))
        })?)
    }

    /// Envia uma mensagem com mídia.
    ///
    /// # Argumentos
    /// * `service_id` - ID do serviço (canal).
    /// * `contact_id` - ID do contato.
    /// * `media_url` - URL da mídia.
    /// * `caption` - Legenda opcional.
    pub async fn enviar_midia(
        &self,
        service_id: &str,
        contact_id: &str,
        media_url: &str,
        caption: Option<&str>,
    ) -> Result<serde_json::Value, DigisacError> {
        let request = DigisacSendMessageRequest::builder()
            .service_id(service_id.to_string())
            .contact_id(contact_id.to_string())
            .text(caption.unwrap_or("").to_string())
            .media_url(media_url.to_string())
            .build();

        let response = self.client.send_message(&request).await?;

        Ok(serde_json::to_value(response).map_err(|e| {
            DigisacError::ApiError(format!("Failed to serialize response: {}", e))
        })?)
    }

    /// Lista webhooks configurados.
    ///
    /// # Retorna
    /// JSON com a lista de webhooks.
    pub async fn listar_webhooks(&self) -> Result<serde_json::Value, DigisacError> {
        let response = self.client.list_webhooks().await?;

        Ok(serde_json::to_value(response).map_err(|e| {
            DigisacError::ApiError(format!("Failed to serialize response: {}", e))
        })?)
    }

    /// Cria um novo webhook.
    ///
    /// # Argumentos
    /// * `url` - URL de destino do webhook.
    /// * `eventos` - Lista de eventos a assinar.
    ///
    /// # Retorna
    /// JSON com os dados do webhook criado.
    pub async fn criar_webhook(
        &self,
        url: &str,
        eventos: Vec<String>,
    ) -> Result<serde_json::Value, DigisacError> {
        // Validações
        if !url.starts_with("https://") {
            return Err(DigisacError::ValidationError(
                "Webhook URL must use HTTPS".into()
            ));
        }
        if eventos.is_empty() {
            return Err(DigisacError::ValidationError(
                "At least one event must be specified".into()
            ));
        }

        let request = WebhookCreateRequest::builder()
            .url(url.to_string())
            .events(eventos)
            .active(true)
            .build();

        let webhook = self.client.create_webhook(&request).await?;

        Ok(serde_json::to_value(webhook).map_err(|e| {
            DigisacError::ApiError(format!("Failed to serialize response: {}", e))
        })?)
    }

    /// Deleta um webhook.
    ///
    /// # Argumentos
    /// * `webhook_id` - ID do webhook a ser deletado.
    pub async fn deletar_webhook(&self, webhook_id: &str) -> Result<(), DigisacError> {
        self.client.delete_webhook(webhook_id).await?;
        Ok(())
    }

    /// Lista contatos.
    ///
    /// # Argumentos
    /// * `limit` - Limite de resultados.
    /// * `offset` - Offset para paginação.
    ///
    /// Nota: A API do crate digisac não suporta diretamente listar contatos.
    /// Este método retorna um erro informando que a funcionalidade não está disponível.
    pub async fn listar_contatos(
        &self,
        _limit: Option<u32>,
        _offset: Option<u32>,
    ) -> Result<serde_json::Value, DigisacError> {
        Err(DigisacError::ApiError(
            "Listar contatos não está disponível na API atual do crate digisac".into()
        ))
    }

    /// Busca contato por telefone.
    ///
    /// # Argumentos
    /// * `phone` - Número de telefone (com DDI).
    ///
    /// Nota: A API do crate digisac não suporta diretamente buscar contatos.
    /// Este método retorna um erro informando que a funcionalidade não está disponível.
    pub async fn buscar_contato_por_telefone(
        &self,
        _phone: &str,
    ) -> Result<serde_json::Value, DigisacError> {
        Err(DigisacError::ApiError(
            "Buscar contato por telefone não está disponível na API atual do crate digisac".into()
        ))
    }
}

#[async_trait]
impl AgentTool for DigisacTools {
    fn name(&self) -> &'static str {
        "digisac"
    }

    fn description(&self) -> &'static str {
        "Ferramenta para enviar mensagens via WhatsApp e gerenciar webhooks usando Digisac. \
        Permite enviar mensagens de texto e mídia, listar e criar webhooks."
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let action = params.get("action")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'action' parameter")?;

        match action {
            "enviar_mensagem" => {
                let service_id = params.get("service_id")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'service_id' parameter")?;
                let contact_id = params.get("contact_id")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'contact_id' parameter")?;
                let texto = params.get("texto")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'texto' parameter")?;

                self.enviar_mensagem(service_id, contact_id, texto)
                    .await
                    .map_err(|e| e.to_string())
            }
            "listar_webhooks" => {
                self.listar_webhooks()
                    .await
                    .map_err(|e| e.to_string())
            }
            "criar_webhook" => {
                let url = params.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'url' parameter")?;
                let eventos: Vec<String> = params.get("eventos")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                self.criar_webhook(url, eventos)
                    .await
                    .map_err(|e| e.to_string())
            }
            "listar_contatos" => {
                let limit = params.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);
                let offset = params.get("offset").and_then(|v| v.as_u64()).map(|n| n as u32);

                self.listar_contatos(limit, offset)
                    .await
                    .map_err(|e| e.to_string())
            }
            "buscar_contato" => {
                let phone = params.get("phone")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'phone' parameter")?;

                self.buscar_contato_por_telefone(phone)
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
    fn test_error_conversion() {
        let crate_error = CrateDigisacError::ConfigError("test".into());
        let our_error: DigisacError = crate_error.into();
        assert!(matches!(our_error, DigisacError::ConfigError(_)));
    }
}
