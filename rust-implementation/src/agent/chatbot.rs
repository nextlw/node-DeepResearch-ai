// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CHATBOT ADAPTER - INTERFACE PARA INTEGRAÇÃO COM PLATAFORMAS DE CHAT
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Este módulo define a interface para integração do agente com plataformas
// de chat externas como DigiSac, Suri (tlw_irus) e Parrachos.
//
// A trait ChatbotAdapter permite:
// - Enviar mensagens para o usuário
// - Receber mensagens do usuário
// - Fazer perguntas e aguardar respostas (blocking)
// - Apresentar opções para seleção
//
// Compatível com OpenAI Responses API (input_required state).
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::interaction::{PendingQuestion, UserResponse};
use async_trait::async_trait;
use thiserror::Error;

/// Erros que podem ocorrer na comunicação com chatbot
#[derive(Debug, Error)]
pub enum ChatbotError {
    /// Erro de conexão com a plataforma
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Erro de autenticação
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Timeout aguardando resposta
    #[error("Timeout waiting for response")]
    Timeout,

    /// Usuário desconectou/saiu
    #[error("User disconnected")]
    UserDisconnected,

    /// Erro ao enviar mensagem
    #[error("Failed to send message: {0}")]
    SendError(String),

    /// Erro ao receber mensagem
    #[error("Failed to receive message: {0}")]
    ReceiveError(String),

    /// Erro de rate limit
    #[error("Rate limit exceeded")]
    RateLimitError,

    /// Erro interno da plataforma
    #[error("Platform error: {0}")]
    PlatformError(String),
}

/// Metadados do usuário conectado
#[derive(Debug, Clone)]
pub struct UserMetadata {
    /// ID único do usuário na plataforma
    pub user_id: String,
    /// Nome do usuário (se disponível)
    pub name: Option<String>,
    /// Número de telefone (se disponível, ex: WhatsApp)
    pub phone: Option<String>,
    /// Email (se disponível)
    pub email: Option<String>,
    /// ID da conversa/sessão
    pub conversation_id: Option<String>,
    /// Plataforma de origem (digisac, suri, parrachos, etc)
    pub platform: String,
    /// Metadados adicionais específicos da plataforma
    pub extra: std::collections::HashMap<String, String>,
}

impl UserMetadata {
    /// Cria metadados básicos
    pub fn new(user_id: impl Into<String>, platform: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            name: None,
            phone: None,
            email: None,
            conversation_id: None,
            platform: platform.into(),
            extra: std::collections::HashMap::new(),
        }
    }

    /// Adiciona nome
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adiciona telefone
    pub fn with_phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = Some(phone.into());
        self
    }

    /// Adiciona conversation_id
    pub fn with_conversation_id(mut self, id: impl Into<String>) -> Self {
        self.conversation_id = Some(id.into());
        self
    }
}

/// Status da conexão com o chatbot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Conectado e pronto
    Connected,
    /// Desconectado
    Disconnected,
    /// Reconectando
    Reconnecting,
    /// Erro de conexão
    Error,
}

/// Trait para adaptadores de chatbot
///
/// Implementações desta trait permitem que o agente se comunique
/// com diferentes plataformas de chat (DigiSac, Suri, Parrachos, etc).
///
/// # Exemplo de Implementação
///
/// ```rust,ignore
/// struct DigiSacAdapter {
///     api_key: String,
///     base_url: String,
///     client: reqwest::Client,
/// }
///
/// #[async_trait]
/// impl ChatbotAdapter for DigiSacAdapter {
///     async fn send_message(&self, message: &str) -> Result<(), ChatbotError> {
///         // Implementar envio via API DigiSac
///         Ok(())
///     }
///     // ... outros métodos
/// }
/// ```
#[async_trait]
pub trait ChatbotAdapter: Send + Sync {
    /// Retorna o nome da plataforma
    fn platform_name(&self) -> &str;

    /// Verifica status da conexão
    fn connection_status(&self) -> ConnectionStatus;

    /// Retorna metadados do usuário atual
    fn user_metadata(&self) -> Option<&UserMetadata>;

    /// Envia uma mensagem de texto para o usuário
    ///
    /// # Arguments
    /// * `message` - Texto da mensagem (suporta markdown básico)
    async fn send_message(&self, message: &str) -> Result<(), ChatbotError>;

    /// Envia uma mensagem com formatação rica (se suportado)
    ///
    /// Plataformas que não suportam formatação devem converter para texto.
    async fn send_rich_message(&self, message: &RichMessage) -> Result<(), ChatbotError> {
        // Implementação padrão: converter para texto simples
        self.send_message(&message.to_plain_text()).await
    }

    /// Faz uma pergunta ao usuário e aguarda resposta (blocking)
    ///
    /// Este método deve pausar até receber a resposta ou timeout.
    ///
    /// # Arguments
    /// * `question` - Pergunta pendente com tipo, texto e opções
    /// * `timeout_secs` - Timeout em segundos (None = sem timeout)
    async fn ask_user(
        &self,
        question: &PendingQuestion,
        timeout_secs: Option<u64>,
    ) -> Result<UserResponse, ChatbotError>;

    /// Envia opções para o usuário escolher (como botões ou lista)
    ///
    /// # Arguments
    /// * `question` - Texto da pergunta
    /// * `options` - Lista de opções
    ///
    /// # Returns
    /// Texto da opção selecionada
    async fn send_options(
        &self,
        question: &str,
        options: &[String],
    ) -> Result<String, ChatbotError>;

    /// Tenta receber uma mensagem do usuário (não blocking)
    ///
    /// Retorna `None` se não houver mensagem disponível.
    async fn try_receive(&self) -> Result<Option<UserResponse>, ChatbotError>;

    /// Aguarda próxima mensagem do usuário (blocking)
    ///
    /// # Arguments
    /// * `timeout_secs` - Timeout em segundos (None = aguardar indefinidamente)
    async fn receive_message(
        &self,
        timeout_secs: Option<u64>,
    ) -> Result<UserResponse, ChatbotError>;

    /// Marca conversa como "digitando" (typing indicator)
    async fn set_typing(&self, _is_typing: bool) -> Result<(), ChatbotError> {
        // Implementação padrão: não faz nada
        Ok(())
    }

    /// Finaliza a conversa/sessão
    async fn end_conversation(&self) -> Result<(), ChatbotError> {
        // Implementação padrão: não faz nada
        Ok(())
    }
}

/// Mensagem com formatação rica
#[derive(Debug, Clone)]
pub struct RichMessage {
    /// Texto principal
    pub text: String,
    /// Título (se suportado)
    pub title: Option<String>,
    /// URL de imagem (se houver)
    pub image_url: Option<String>,
    /// Botões de ação
    pub buttons: Vec<MessageButton>,
    /// Cor da mensagem (hex)
    pub color: Option<String>,
}

impl RichMessage {
    /// Cria mensagem de texto simples
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            title: None,
            image_url: None,
            buttons: Vec::new(),
            color: None,
        }
    }

    /// Adiciona título
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Adiciona imagem
    pub fn with_image(mut self, url: impl Into<String>) -> Self {
        self.image_url = Some(url.into());
        self
    }

    /// Adiciona botão
    pub fn add_button(mut self, button: MessageButton) -> Self {
        self.buttons.push(button);
        self
    }

    /// Converte para texto simples
    pub fn to_plain_text(&self) -> String {
        let mut text = String::new();

        if let Some(title) = &self.title {
            text.push_str(&format!("*{}*\n\n", title));
        }

        text.push_str(&self.text);

        if !self.buttons.is_empty() {
            text.push_str("\n\nOpções:\n");
            for (i, btn) in self.buttons.iter().enumerate() {
                text.push_str(&format!("{}. {}\n", i + 1, btn.label));
            }
        }

        text
    }
}

/// Botão de ação em mensagem rica
#[derive(Debug, Clone)]
pub struct MessageButton {
    /// Texto do botão
    pub label: String,
    /// Valor a enviar quando clicado
    pub value: String,
    /// Tipo do botão
    pub button_type: ButtonType,
}

impl MessageButton {
    /// Cria botão de resposta
    pub fn reply(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            button_type: ButtonType::Reply,
        }
    }

    /// Cria botão de URL
    pub fn url(label: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: url.into(),
            button_type: ButtonType::Url,
        }
    }
}

/// Tipo de botão
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonType {
    /// Botão de resposta (envia valor)
    Reply,
    /// Botão de URL (abre link)
    Url,
    /// Botão de telefone (liga)
    Phone,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO MOCK PARA TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Adapter mock para testes
#[derive(Debug)]
pub struct MockChatbotAdapter {
    platform: String,
    user: UserMetadata,
    messages_sent: std::sync::Mutex<Vec<String>>,
    responses: std::sync::Mutex<std::collections::VecDeque<String>>,
}

impl MockChatbotAdapter {
    /// Cria novo adapter mock
    pub fn new() -> Self {
        Self {
            platform: "mock".into(),
            user: UserMetadata::new("test_user", "mock"),
            messages_sent: std::sync::Mutex::new(Vec::new()),
            responses: std::sync::Mutex::new(std::collections::VecDeque::new()),
        }
    }

    /// Define resposta a ser retornada
    pub fn set_response(&self, response: impl Into<String>) {
        self.responses.lock().unwrap().push_back(response.into());
    }

    /// Retorna mensagens enviadas
    pub fn get_sent_messages(&self) -> Vec<String> {
        self.messages_sent.lock().unwrap().clone()
    }
}

impl Default for MockChatbotAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatbotAdapter for MockChatbotAdapter {
    fn platform_name(&self) -> &str {
        &self.platform
    }

    fn connection_status(&self) -> ConnectionStatus {
        ConnectionStatus::Connected
    }

    fn user_metadata(&self) -> Option<&UserMetadata> {
        Some(&self.user)
    }

    async fn send_message(&self, message: &str) -> Result<(), ChatbotError> {
        self.messages_sent.lock().unwrap().push(message.to_string());
        Ok(())
    }

    async fn ask_user(
        &self,
        _question: &PendingQuestion,
        _timeout_secs: Option<u64>,
    ) -> Result<UserResponse, ChatbotError> {
        if let Some(response) = self.responses.lock().unwrap().pop_front() {
            Ok(UserResponse::spontaneous(response))
        } else {
            Err(ChatbotError::Timeout)
        }
    }

    async fn send_options(
        &self,
        question: &str,
        options: &[String],
    ) -> Result<String, ChatbotError> {
        self.messages_sent.lock().unwrap().push(format!(
            "{}\nOpções: {}",
            question,
            options.join(", ")
        ));

        if let Some(response) = self.responses.lock().unwrap().pop_front() {
            Ok(response)
        } else {
            Err(ChatbotError::Timeout)
        }
    }

    async fn try_receive(&self) -> Result<Option<UserResponse>, ChatbotError> {
        Ok(self.responses.lock().unwrap().pop_front()
            .map(UserResponse::spontaneous))
    }

    async fn receive_message(
        &self,
        _timeout_secs: Option<u64>,
    ) -> Result<UserResponse, ChatbotError> {
        if let Some(response) = self.responses.lock().unwrap().pop_front() {
            Ok(UserResponse::spontaneous(response))
        } else {
            Err(ChatbotError::Timeout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_adapter() {
        let adapter = MockChatbotAdapter::new();

        // Testar envio
        adapter.send_message("Hello").await.unwrap();
        assert_eq!(adapter.get_sent_messages(), vec!["Hello"]);

        // Testar recebimento
        adapter.set_response("Response 1");
        let response = adapter.receive_message(None).await.unwrap();
        assert_eq!(response.content, "Response 1");
    }

    #[test]
    fn test_rich_message() {
        let msg = RichMessage::text("Hello")
            .with_title("Title")
            .add_button(MessageButton::reply("Option 1", "opt1"))
            .add_button(MessageButton::reply("Option 2", "opt2"));

        let plain = msg.to_plain_text();
        assert!(plain.contains("*Title*"));
        assert!(plain.contains("Hello"));
        assert!(plain.contains("1. Option 1"));
        assert!(plain.contains("2. Option 2"));
    }

    #[test]
    fn test_user_metadata() {
        let user = UserMetadata::new("user123", "digisac")
            .with_name("John Doe")
            .with_phone("+5511999999999");

        assert_eq!(user.user_id, "user123");
        assert_eq!(user.platform, "digisac");
        assert_eq!(user.name, Some("John Doe".into()));
        assert_eq!(user.phone, Some("+5511999999999".into()));
    }
}
