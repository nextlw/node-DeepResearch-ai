// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SISTEMA DE INTERAÇÃO USUÁRIO-AGENTE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Este módulo implementa a comunicação bidirecional entre o agente e o usuário,
// permitindo que:
// - O agente faça perguntas quando falta informação vital
// - O usuário possa interferir durante a pesquisa
//
// Compatível com OpenAI Responses API (conceito de `input_required` state).
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Tipos de pergunta que o agente pode fazer ao usuário
///
/// Cada tipo tem comportamento diferente:
/// - `Clarification`: Blocking - agente para até receber resposta
/// - `Confirmation`: Blocking - aguarda sim/não antes de continuar
/// - `Preference`: Blocking - usuário escolhe entre opções
/// - `Suggestion`: Async - agente sugere algo mas continua
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestionType {
    /// Falta informação vital para completar a tarefa
    ///
    /// Exemplo: "Qual é a cidade de origem da sua viagem?"
    /// Sempre blocking - não faz sentido continuar sem a resposta.
    Clarification,

    /// Confirmação antes de ação importante
    ///
    /// Exemplo: "Posso executar este código que acessa a API?"
    /// Blocking por padrão, mas pode ser configurado como async.
    Confirmation,

    /// Escolha entre múltiplas opções válidas
    ///
    /// Exemplo: "Prefere voo direto (R$ 800) ou com escala (R$ 500)?"
    /// Blocking - apresenta opções e aguarda seleção.
    Preference,

    /// Sugestão ou feedback não crítico
    ///
    /// Exemplo: "Encontrei informações de 2023, deseja dados mais recentes?"
    /// Async por padrão - agente continua mesmo sem resposta.
    Suggestion,
}

impl QuestionType {
    /// Retorna se o tipo de pergunta é blocking por padrão
    pub fn is_blocking_by_default(&self) -> bool {
        match self {
            QuestionType::Clarification => true,
            QuestionType::Confirmation => true,
            QuestionType::Preference => true,
            QuestionType::Suggestion => false,
        }
    }

    /// Converte para string para serialização
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionType::Clarification => "clarification",
            QuestionType::Confirmation => "confirmation",
            QuestionType::Preference => "preference",
            QuestionType::Suggestion => "suggestion",
        }
    }

    /// Cria QuestionType a partir de string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "clarification" => Some(QuestionType::Clarification),
            "confirmation" => Some(QuestionType::Confirmation),
            "preference" => Some(QuestionType::Preference),
            "suggestion" => Some(QuestionType::Suggestion),
            _ => None,
        }
    }
}

/// Pergunta pendente do agente para o usuário
///
/// Quando o agente precisa de informação do usuário, ele cria uma
/// PendingQuestion e a adiciona ao InteractionHub. O sistema de interface
/// (TUI, Chatbot) é responsável por apresentar a pergunta e coletar a resposta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQuestion {
    /// ID único da pergunta
    pub id: String,

    /// Tipo da pergunta (afeta comportamento blocking/async)
    pub question_type: QuestionType,

    /// Texto da pergunta a ser exibida ao usuário
    pub question: String,

    /// Opções de resposta (para perguntas do tipo Preference)
    ///
    /// Se Some, apresentar como seleção. Se None, campo de texto livre.
    pub options: Option<Vec<String>>,

    /// Se deve bloquear a execução até receber resposta
    ///
    /// Quando true, o agente entra no estado `InputRequired` e pausa.
    /// Quando false, a pergunta é enviada mas o agente continua.
    pub is_blocking: bool,

    /// Contexto adicional para ajudar o usuário a responder
    pub context: Option<String>,

    /// Timestamp de criação
    pub created_at: DateTime<Utc>,

    /// Raciocínio do agente (por que está perguntando)
    pub think: String,
}

impl PendingQuestion {
    /// Cria uma nova pergunta de clarificação (blocking)
    pub fn clarification(question: impl Into<String>, think: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            question_type: QuestionType::Clarification,
            question: question.into(),
            options: None,
            is_blocking: true,
            context: None,
            created_at: Utc::now(),
            think: think.into(),
        }
    }

    /// Cria uma nova pergunta de confirmação (blocking)
    pub fn confirmation(question: impl Into<String>, think: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            question_type: QuestionType::Confirmation,
            question: question.into(),
            options: Some(vec!["Sim".into(), "Não".into()]),
            is_blocking: true,
            context: None,
            created_at: Utc::now(),
            think: think.into(),
        }
    }

    /// Cria uma nova pergunta de preferência (blocking)
    pub fn preference(
        question: impl Into<String>,
        options: Vec<String>,
        think: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            question_type: QuestionType::Preference,
            question: question.into(),
            options: Some(options),
            is_blocking: true,
            context: None,
            created_at: Utc::now(),
            think: think.into(),
        }
    }

    /// Cria uma nova sugestão (async, não blocking)
    pub fn suggestion(question: impl Into<String>, think: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            question_type: QuestionType::Suggestion,
            question: question.into(),
            options: None,
            is_blocking: false,
            context: None,
            created_at: Utc::now(),
            think: think.into(),
        }
    }

    /// Adiciona contexto à pergunta
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Define se é blocking
    pub fn blocking(mut self, is_blocking: bool) -> Self {
        self.is_blocking = is_blocking;
        self
    }

    /// Serializa para formato compatível com OpenAI Responses API
    pub fn to_openai_format(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "input_required",
            "pending_input": {
                "id": self.id,
                "type": self.question_type.as_str(),
                "question": self.question,
                "options": self.options,
                "context": self.context,
            }
        })
    }
}

/// Resposta do usuário a uma pergunta ou mensagem espontânea
///
/// Pode ser:
/// - Resposta a uma pergunta específica (question_id presente)
/// - Mensagem espontânea durante a pesquisa (question_id None)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    /// ID da pergunta sendo respondida (None se mensagem espontânea)
    pub question_id: Option<String>,

    /// Conteúdo da resposta/mensagem
    pub content: String,

    /// Timestamp da resposta
    pub timestamp: DateTime<Utc>,

    /// Se a resposta foi uma seleção de opção (índice da opção)
    pub selected_option: Option<usize>,
}

impl UserResponse {
    /// Cria uma nova resposta para uma pergunta específica
    pub fn to_question(question_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            question_id: Some(question_id.into()),
            content: content.into(),
            timestamp: Utc::now(),
            selected_option: None,
        }
    }

    /// Cria uma nova mensagem espontânea (sem pergunta associada)
    pub fn spontaneous(content: impl Into<String>) -> Self {
        Self {
            question_id: None,
            content: content.into(),
            timestamp: Utc::now(),
            selected_option: None,
        }
    }

    /// Cria uma resposta com seleção de opção
    pub fn with_option(
        question_id: impl Into<String>,
        option_index: usize,
        option_text: impl Into<String>,
    ) -> Self {
        Self {
            question_id: Some(question_id.into()),
            content: option_text.into(),
            timestamp: Utc::now(),
            selected_option: Some(option_index),
        }
    }

    /// Verifica se é uma confirmação positiva
    pub fn is_affirmative(&self) -> bool {
        let lower = self.content.to_lowercase();
        lower == "sim"
            || lower == "yes"
            || lower == "s"
            || lower == "y"
            || lower == "ok"
            || lower == "confirmar"
            || lower == "confirmo"
    }

    /// Verifica se é uma confirmação negativa
    pub fn is_negative(&self) -> bool {
        let lower = self.content.to_lowercase();
        lower == "não"
            || lower == "nao"
            || lower == "no"
            || lower == "n"
            || lower == "cancelar"
            || lower == "cancela"
    }
}

/// Hub de interação - gerencia comunicação bidirecional usuário-agente
///
/// O InteractionHub é o ponto central de comunicação entre o agente e
/// qualquer interface (TUI, Chatbot, API). Ele mantém:
/// - Fila de perguntas pendentes do agente
/// - Fila de respostas/mensagens do usuário
/// - Canais para comunicação assíncrona
pub struct InteractionHub {
    /// Perguntas pendentes do agente
    pending_questions: VecDeque<PendingQuestion>,

    /// Respostas/mensagens do usuário ainda não processadas
    user_responses: VecDeque<UserResponse>,

    /// Canal para receber respostas do usuário (lado receptor)
    response_rx: Option<mpsc::Receiver<UserResponse>>,

    /// Canal para enviar perguntas para a interface (lado enviador)
    question_tx: Option<mpsc::Sender<PendingQuestion>>,
}

impl InteractionHub {
    /// Cria um novo InteractionHub sem canais externos
    ///
    /// Use `with_channels` para configurar comunicação com interface externa.
    pub fn new() -> Self {
        Self {
            pending_questions: VecDeque::new(),
            user_responses: VecDeque::new(),
            response_rx: None,
            question_tx: None,
        }
    }

    /// Cria um InteractionHub com canais de comunicação
    ///
    /// # Arguments
    /// * `response_rx` - Canal para receber respostas do usuário
    /// * `question_tx` - Canal para enviar perguntas para a interface
    pub fn with_channels(
        response_rx: mpsc::Receiver<UserResponse>,
        question_tx: mpsc::Sender<PendingQuestion>,
    ) -> Self {
        Self {
            pending_questions: VecDeque::new(),
            user_responses: VecDeque::new(),
            response_rx: Some(response_rx),
            question_tx: Some(question_tx),
        }
    }

    /// Adiciona uma pergunta do agente para o usuário
    ///
    /// Se houver canal configurado, também envia para a interface.
    pub async fn ask(&mut self, question: PendingQuestion) -> Result<(), InteractionError> {
        // Enviar para interface se canal disponível
        if let Some(tx) = &self.question_tx {
            tx.send(question.clone())
                .await
                .map_err(|_| InteractionError::ChannelClosed)?;
        }

        // Adicionar à fila interna
        self.pending_questions.push_back(question);
        Ok(())
    }

    /// Adiciona uma resposta do usuário à fila
    pub fn receive_response(&mut self, response: UserResponse) {
        self.user_responses.push_back(response);
    }

    /// Verifica se há perguntas blocking pendentes
    pub fn has_blocking_question(&self) -> bool {
        self.pending_questions
            .iter()
            .any(|q| q.is_blocking)
    }

    /// Retorna a pergunta blocking mais antiga (se houver)
    pub fn get_blocking_question(&self) -> Option<&PendingQuestion> {
        self.pending_questions
            .iter()
            .find(|q| q.is_blocking)
    }

    /// Processa respostas recebidas via canal
    ///
    /// Deve ser chamado no início de cada step para processar
    /// mensagens que chegaram de forma assíncrona.
    pub fn poll_responses(&mut self) {
        if let Some(rx) = &mut self.response_rx {
            // Tentar receber sem bloquear
            while let Ok(response) = rx.try_recv() {
                self.user_responses.push_back(response);
            }
        }
    }

    /// Retorna a próxima resposta do usuário (se houver)
    pub fn next_response(&mut self) -> Option<UserResponse> {
        self.user_responses.pop_front()
    }

    /// Retorna todas as respostas pendentes
    pub fn drain_responses(&mut self) -> Vec<UserResponse> {
        self.user_responses.drain(..).collect()
    }

    /// Marca uma pergunta como respondida e a remove da fila
    pub fn mark_answered(&mut self, question_id: &str) -> Option<PendingQuestion> {
        if let Some(pos) = self
            .pending_questions
            .iter()
            .position(|q| q.id == question_id)
        {
            self.pending_questions.remove(pos)
        } else {
            None
        }
    }

    /// Verifica se há respostas pendentes
    pub fn has_pending_responses(&self) -> bool {
        !self.user_responses.is_empty()
    }

    /// Retorna número de perguntas pendentes
    pub fn pending_questions_count(&self) -> usize {
        self.pending_questions.len()
    }

    /// Retorna número de respostas não processadas
    pub fn pending_responses_count(&self) -> usize {
        self.user_responses.len()
    }

    /// Encontra resposta para uma pergunta específica
    pub fn find_response_for(&mut self, question_id: &str) -> Option<UserResponse> {
        if let Some(pos) = self
            .user_responses
            .iter()
            .position(|r| r.question_id.as_deref() == Some(question_id))
        {
            self.user_responses.remove(pos)
        } else {
            None
        }
    }

    /// Aguarda resposta para uma pergunta específica (blocking)
    ///
    /// # Arguments
    /// * `question_id` - ID da pergunta
    /// * `timeout` - Timeout em segundos (None = sem timeout)
    pub async fn wait_for_response(
        &mut self,
        question_id: &str,
        timeout_secs: Option<u64>,
    ) -> Result<UserResponse, InteractionError> {
        use tokio::time::{timeout, Duration};

        let wait_future = async {
            loop {
                // Verificar se já temos a resposta
                if let Some(response) = self.find_response_for(question_id) {
                    return Ok(response);
                }

                // Tentar receber do canal
                if let Some(rx) = &mut self.response_rx {
                    match rx.recv().await {
                        Some(response) => {
                            if response.question_id.as_deref() == Some(question_id) {
                                return Ok(response);
                            } else {
                                // Resposta para outra pergunta, guardar
                                self.user_responses.push_back(response);
                            }
                        }
                        None => return Err(InteractionError::ChannelClosed),
                    }
                } else {
                    // Sem canal, aguardar um pouco e verificar novamente
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        };

        match timeout_secs {
            Some(secs) => {
                timeout(Duration::from_secs(secs), wait_future)
                    .await
                    .map_err(|_| InteractionError::Timeout)?
            }
            None => wait_future.await,
        }
    }
}

impl Default for InteractionHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Erros que podem ocorrer na interação
#[derive(Debug, Clone, thiserror::Error)]
pub enum InteractionError {
    /// Canal de comunicação fechado
    #[error("Communication channel closed")]
    ChannelClosed,

    /// Timeout aguardando resposta
    #[error("Timeout waiting for user response")]
    Timeout,

    /// Resposta inválida para o tipo de pergunta
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Cria um par de canais para comunicação com InteractionHub
///
/// Retorna:
/// - `response_tx`: Sender para enviar respostas do usuário para o hub
/// - `question_rx`: Receiver para receber perguntas do agente
/// - `InteractionHub`: Hub configurado com os canais
pub fn create_interaction_channels(
    buffer_size: usize,
) -> (
    mpsc::Sender<UserResponse>,
    mpsc::Receiver<PendingQuestion>,
    InteractionHub,
) {
    let (response_tx, response_rx) = mpsc::channel(buffer_size);
    let (question_tx, question_rx) = mpsc::channel(buffer_size);

    let hub = InteractionHub::with_channels(response_rx, question_tx);

    (response_tx, question_rx, hub)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_type_blocking() {
        assert!(QuestionType::Clarification.is_blocking_by_default());
        assert!(QuestionType::Confirmation.is_blocking_by_default());
        assert!(QuestionType::Preference.is_blocking_by_default());
        assert!(!QuestionType::Suggestion.is_blocking_by_default());
    }

    #[test]
    fn test_pending_question_builders() {
        let clarification = PendingQuestion::clarification("What is your name?", "Need user name");
        assert!(clarification.is_blocking);
        assert_eq!(clarification.question_type, QuestionType::Clarification);
        assert!(clarification.options.is_none());

        let confirmation = PendingQuestion::confirmation("Execute code?", "Need confirmation");
        assert!(confirmation.is_blocking);
        assert_eq!(confirmation.question_type, QuestionType::Confirmation);
        assert!(confirmation.options.is_some());

        let preference = PendingQuestion::preference(
            "Choose option",
            vec!["A".into(), "B".into()],
            "Need preference",
        );
        assert!(preference.is_blocking);
        assert_eq!(preference.question_type, QuestionType::Preference);
        assert_eq!(preference.options.unwrap().len(), 2);

        let suggestion = PendingQuestion::suggestion("Consider this", "Just a suggestion");
        assert!(!suggestion.is_blocking);
        assert_eq!(suggestion.question_type, QuestionType::Suggestion);
    }

    #[test]
    fn test_user_response_affirmative() {
        let yes_responses = ["sim", "Sim", "SIM", "yes", "Yes", "s", "y", "ok", "confirmar"];
        for r in yes_responses {
            let response = UserResponse::spontaneous(r);
            assert!(response.is_affirmative(), "Failed for: {}", r);
        }

        let no_responses = ["não", "Não", "nao", "no", "n", "cancelar"];
        for r in no_responses {
            let response = UserResponse::spontaneous(r);
            assert!(response.is_negative(), "Failed for: {}", r);
        }
    }

    #[test]
    fn test_openai_format() {
        let question = PendingQuestion::clarification("Origin city?", "Need origin");
        let json = question.to_openai_format();

        assert_eq!(json["type"], "input_required");
        assert_eq!(json["pending_input"]["type"], "clarification");
        assert_eq!(json["pending_input"]["question"], "Origin city?");
    }

    #[tokio::test]
    async fn test_interaction_hub_basic() {
        let mut hub = InteractionHub::new();

        // Adicionar pergunta
        let question = PendingQuestion::clarification("Test?", "Testing");
        hub.ask(question.clone()).await.unwrap();

        assert_eq!(hub.pending_questions_count(), 1);
        assert!(hub.has_blocking_question());

        // Receber resposta
        let response = UserResponse::to_question(&question.id, "Answer");
        hub.receive_response(response);

        assert!(hub.has_pending_responses());
        assert_eq!(hub.pending_responses_count(), 1);

        // Processar resposta
        let resp = hub.next_response().unwrap();
        assert_eq!(resp.content, "Answer");
    }
}
