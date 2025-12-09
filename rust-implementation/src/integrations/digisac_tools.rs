//! # Digisac Tools
//!
//! Este módulo implementa a integração com a API Digisac para
//! gerenciamento de mensagens via WhatsApp e outros canais.
//!
//! ## Funcionalidades
//!
//! - Enviar mensagens
//! - Listar webhooks
//! - Criar webhooks
//! - Gerenciar contatos

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;

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

/// Configuração do cliente Digisac.
#[derive(Debug, Clone)]
pub struct DigisacConfig {
    /// URL base da API.
    pub base_url: String,
    
    /// Token de API.
    pub api_token: Option<String>,
    
    /// ID da conta.
    pub account_id: Option<String>,
    
    /// Timeout em segundos.
    pub timeout_secs: u64,
}

impl Default for DigisacConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.digisac.me/v1".to_string(),
            api_token: None,
            account_id: None,
            timeout_secs: 30,
        }
    }
}

/// Requisição para enviar mensagem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// ID do serviço (canal de comunicação).
    pub service_id: String,
    
    /// ID do contato.
    pub contact_id: String,
    
    /// Texto da mensagem.
    pub text: String,
    
    /// Tipo de mensagem.
    #[serde(default = "default_message_type")]
    pub message_type: String,
    
    /// Mídia anexada (opcional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<MediaAttachment>,
}

fn default_message_type() -> String {
    "text".to_string()
}

/// Anexo de mídia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    /// Tipo de mídia (image, audio, video, document).
    pub media_type: String,
    
    /// URL da mídia.
    pub url: String,
    
    /// Nome do arquivo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    
    /// Legenda.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

/// Resposta de envio de mensagem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    /// ID da mensagem.
    pub id: String,
    
    /// Status da mensagem.
    pub status: String,
    
    /// Timestamp de criação.
    #[serde(default)]
    pub created_at: Option<String>,
    
    /// ID do contato.
    #[serde(default)]
    pub contact_id: Option<String>,
}

/// Webhook configurado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    /// ID do webhook.
    pub id: String,
    
    /// URL de destino.
    pub url: String,
    
    /// Eventos assinados.
    pub events: Vec<String>,
    
    /// Se está ativo.
    #[serde(default = "default_true")]
    pub active: bool,
    
    /// Data de criação.
    #[serde(default)]
    pub created_at: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Requisição para criar webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    /// URL de destino.
    pub url: String,
    
    /// Eventos a assinar.
    pub events: Vec<String>,
    
    /// Se deve estar ativo.
    #[serde(default = "default_true")]
    pub active: bool,
}

/// Eventos disponíveis para webhook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookEvent {
    /// Nova mensagem recebida.
    MessageReceived,
    /// Mensagem enviada.
    MessageSent,
    /// Status da mensagem atualizado.
    MessageStatusUpdate,
    /// Novo contato.
    ContactCreated,
    /// Contato atualizado.
    ContactUpdated,
    /// Ticket criado.
    TicketCreated,
    /// Ticket fechado.
    TicketClosed,
}

impl WebhookEvent {
    /// Retorna o nome do evento como string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MessageReceived => "message.received",
            Self::MessageSent => "message.sent",
            Self::MessageStatusUpdate => "message.status",
            Self::ContactCreated => "contact.created",
            Self::ContactUpdated => "contact.updated",
            Self::TicketCreated => "ticket.created",
            Self::TicketClosed => "ticket.closed",
        }
    }
    
    /// Cria evento a partir de string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "message.received" => Some(Self::MessageReceived),
            "message.sent" => Some(Self::MessageSent),
            "message.status" => Some(Self::MessageStatusUpdate),
            "contact.created" => Some(Self::ContactCreated),
            "contact.updated" => Some(Self::ContactUpdated),
            "ticket.created" => Some(Self::TicketCreated),
            "ticket.closed" => Some(Self::TicketClosed),
            _ => None,
        }
    }
}

/// Contato do Digisac.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// ID do contato.
    pub id: String,
    
    /// Nome do contato.
    #[serde(default)]
    pub name: String,
    
    /// Número de telefone.
    #[serde(default)]
    pub phone: Option<String>,
    
    /// Email.
    #[serde(default)]
    pub email: Option<String>,
    
    /// Tags/etiquetas.
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// Campos customizados.
    #[serde(default)]
    pub custom_fields: HashMap<String, String>,
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
    /// Cliente HTTP.
    client: Client,
    
    /// Configuração.
    config: DigisacConfig,
}

impl DigisacTools {
    /// Cria um novo cliente Digisac.
    ///
    /// Tenta carregar configuração do ambiente.
    pub async fn new() -> Result<Self, DigisacError> {
        let config = Self::load_config_from_env()?;
        Self::with_config(config).await
    }
    
    /// Cria um novo cliente Digisac com configuração customizada.
    pub async fn with_config(config: DigisacConfig) -> Result<Self, DigisacError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        Ok(Self { client, config })
    }
    
    /// Carrega configuração do ambiente.
    fn load_config_from_env() -> Result<DigisacConfig, DigisacError> {
        let base_url = std::env::var("DIGISAC_BASE_URL")
            .unwrap_or_else(|_| "https://api.digisac.me/v1".to_string());
        
        let api_token = std::env::var("DIGISAC_API_TOKEN").ok();
        let account_id = std::env::var("DIGISAC_ACCOUNT_ID").ok();
        
        Ok(DigisacConfig {
            base_url,
            api_token,
            account_id,
            timeout_secs: 30,
        })
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
        
        let url = format!("{}/messages", self.config.base_url);
        
        let body = SendMessageRequest {
            service_id: service_id.to_string(),
            contact_id: contact_id.to_string(),
            text: texto.to_string(),
            message_type: "text".to_string(),
            media: None,
        };
        
        let request = self
            .add_auth_headers(self.client.post(&url))
            .json(&body);
        
        let response = request
            .send()
            .await
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    /// Envia uma mensagem com mídia.
    ///
    /// # Argumentos
    /// * `service_id` - ID do serviço (canal).
    /// * `contact_id` - ID do contato.
    /// * `media_type` - Tipo de mídia (image, audio, video, document).
    /// * `media_url` - URL da mídia.
    /// * `caption` - Legenda opcional.
    pub async fn enviar_midia(
        &self,
        service_id: &str,
        contact_id: &str,
        media_type: &str,
        media_url: &str,
        caption: Option<&str>,
    ) -> Result<serde_json::Value, DigisacError> {
        let url = format!("{}/messages", self.config.base_url);
        
        let body = SendMessageRequest {
            service_id: service_id.to_string(),
            contact_id: contact_id.to_string(),
            text: caption.unwrap_or("").to_string(),
            message_type: media_type.to_string(),
            media: Some(MediaAttachment {
                media_type: media_type.to_string(),
                url: media_url.to_string(),
                filename: None,
                caption: caption.map(|s| s.to_string()),
            }),
        };
        
        let request = self
            .add_auth_headers(self.client.post(&url))
            .json(&body);
        
        let response = request
            .send()
            .await
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    /// Lista webhooks configurados.
    ///
    /// # Retorna
    /// JSON com a lista de webhooks.
    pub async fn listar_webhooks(&self) -> Result<serde_json::Value, DigisacError> {
        let url = format!("{}/webhooks", self.config.base_url);
        
        let request = self.add_auth_headers(self.client.get(&url));
        
        let response = request
            .send()
            .await
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
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
        
        let api_url = format!("{}/webhooks", self.config.base_url);
        
        let body = CreateWebhookRequest {
            url: url.to_string(),
            events: eventos,
            active: true,
        };
        
        let request = self
            .add_auth_headers(self.client.post(&api_url))
            .json(&body);
        
        let response = request
            .send()
            .await
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    /// Deleta um webhook.
    ///
    /// # Argumentos
    /// * `webhook_id` - ID do webhook a ser deletado.
    pub async fn deletar_webhook(&self, webhook_id: &str) -> Result<(), DigisacError> {
        let url = format!("{}/webhooks/{}", self.config.base_url, webhook_id);
        
        let request = self.add_auth_headers(self.client.delete(&url));
        
        let response = request
            .send()
            .await
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error = self.handle_response(response).await;
            Err(error.unwrap_err())
        }
    }
    
    /// Lista contatos.
    ///
    /// # Argumentos
    /// * `limit` - Limite de resultados.
    /// * `offset` - Offset para paginação.
    pub async fn listar_contatos(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<serde_json::Value, DigisacError> {
        let url = format!("{}/contacts", self.config.base_url);
        
        let mut request = self.add_auth_headers(self.client.get(&url));
        
        if let Some(l) = limit {
            request = request.query(&[("limit", l.to_string())]);
        }
        if let Some(o) = offset {
            request = request.query(&[("offset", o.to_string())]);
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    /// Busca contato por telefone.
    ///
    /// # Argumentos
    /// * `phone` - Número de telefone (com DDI).
    pub async fn buscar_contato_por_telefone(
        &self,
        phone: &str,
    ) -> Result<serde_json::Value, DigisacError> {
        let url = format!("{}/contacts", self.config.base_url);
        
        let request = self
            .add_auth_headers(self.client.get(&url))
            .query(&[("phone", phone)]);
        
        let response = request
            .send()
            .await
            .map_err(|e| DigisacError::NetworkError(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    /// Adiciona headers de autenticação ao request.
    fn add_auth_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut req = request;
        
        if let Some(ref token) = self.config.api_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        
        if let Some(ref account_id) = self.config.account_id {
            req = req.header("X-Account-ID", account_id);
        }
        
        req.header("Content-Type", "application/json")
    }
    
    /// Processa a resposta HTTP.
    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<serde_json::Value, DigisacError> {
        let status = response.status();
        
        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| DigisacError::ApiError(format!("Failed to parse response: {}", e)))
        } else if status.as_u16() == 401 {
            Err(DigisacError::AuthError("Invalid API token".into()))
        } else if status.as_u16() == 404 {
            Err(DigisacError::NotFound("Resource not found".into()))
        } else if status.as_u16() == 422 {
            let error_text = response.text().await.unwrap_or_default();
            Err(DigisacError::ValidationError(error_text))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(DigisacError::ApiError(format!(
                "API error ({}): {}",
                status, error_text
            )))
        }
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
    fn test_config_default() {
        let config = DigisacConfig::default();
        assert!(config.base_url.contains("digisac"));
        assert!(config.api_token.is_none());
    }
    
    #[test]
    fn test_webhook_event_conversion() {
        assert_eq!(WebhookEvent::MessageReceived.as_str(), "message.received");
        assert_eq!(
            WebhookEvent::from_str("message.received"),
            Some(WebhookEvent::MessageReceived)
        );
        assert_eq!(WebhookEvent::from_str("invalid"), None);
    }
    
    #[test]
    fn test_send_message_request_serialization() {
        let request = SendMessageRequest {
            service_id: "svc_123".into(),
            contact_id: "cnt_456".into(),
            text: "Hello".into(),
            message_type: "text".into(),
            media: None,
        };
        
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["service_id"], "svc_123");
        assert_eq!(json["contact_id"], "cnt_456");
    }
    
    #[test]
    fn test_webhook_deserialization() {
        let json = r#"{
            "id": "wh_123",
            "url": "https://example.com/webhook",
            "events": ["message.received", "message.sent"]
        }"#;
        
        let webhook: Webhook = serde_json::from_str(json).unwrap();
        assert_eq!(webhook.id, "wh_123");
        assert!(webhook.active); // default
        assert_eq!(webhook.events.len(), 2);
    }
}
