// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CLIENTE LLM
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Trait e implementações para interação com modelos de linguagem.
// Suporta múltiplos provedores: OpenAI, Anthropic, local, etc.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use async_trait::async_trait;
use crate::agent::{AgentAction, AgentPrompt, ActionPermissions};
use crate::types::Reference;

/// Erros do cliente LLM
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Rate limit exceeded")]
    RateLimitError,

    #[error("Invalid response format: {0}")]
    ParseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Token limit exceeded: {used} > {limit}")]
    TokenLimitError { used: u64, limit: u64 },
}

/// Resposta gerada pelo LLM
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub answer: String,
    pub references: Vec<Reference>,
    pub tokens_used: u64,
}

/// Resultado de embedding
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub vector: Vec<f32>,
    pub tokens_used: u64,
}

/// Trait principal para clientes LLM
///
/// Esta trait define a interface que qualquer provedor de LLM deve implementar.
/// Permite fácil substituição entre provedores (OpenAI, Anthropic, local).
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Decide a próxima ação baseado no prompt e permissões
    async fn decide_action(
        &self,
        prompt: &AgentPrompt,
        permissions: &ActionPermissions,
    ) -> Result<AgentAction, LlmError>;

    /// Gera uma resposta final para a pergunta
    async fn generate_answer(
        &self,
        prompt: &AgentPrompt,
        temperature: f32,
    ) -> Result<LlmResponse, LlmError>;

    /// Gera embeddings para um texto
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, LlmError>;

    /// Gera embeddings em batch
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError>;

    /// Avalia se uma resposta atende aos critérios
    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        criteria: &str,
    ) -> Result<EvaluationResponse, LlmError>;

    /// Determina os tipos de avaliação necessários
    async fn determine_eval_types(
        &self,
        question: &str,
    ) -> Result<Vec<crate::evaluation::EvaluationType>, LlmError>;
}

/// Resposta de avaliação
#[derive(Debug, Clone)]
pub struct EvaluationResponse {
    pub passed: bool,
    pub reasoning: String,
    pub confidence: f32,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO MOCK PARA TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cliente mock para testes unitários
#[derive(Debug, Default)]
pub struct MockLlmClient {
    pub default_action: Option<AgentAction>,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_action(action: AgentAction) -> Self {
        Self {
            default_action: Some(action),
        }
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn decide_action(
        &self,
        _prompt: &AgentPrompt,
        permissions: &ActionPermissions,
    ) -> Result<AgentAction, LlmError> {
        if let Some(action) = &self.default_action {
            return Ok(action.clone());
        }

        // Retorna primeira ação permitida
        if permissions.search {
            Ok(AgentAction::Search {
                queries: vec![],
                think: "Mock search".into(),
            })
        } else if permissions.answer {
            Ok(AgentAction::Answer {
                answer: "Mock answer".into(),
                references: vec![],
                think: "Mock answer".into(),
            })
        } else {
            Err(LlmError::ApiError("No valid action".into()))
        }
    }

    async fn generate_answer(
        &self,
        _prompt: &AgentPrompt,
        _temperature: f32,
    ) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            answer: "Mock generated answer".into(),
            references: vec![],
            tokens_used: 100,
        })
    }

    async fn embed(&self, _text: &str) -> Result<EmbeddingResult, LlmError> {
        Ok(EmbeddingResult {
            vector: vec![0.0; 1536],
            tokens_used: 10,
        })
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError> {
        Ok(texts
            .iter()
            .map(|_| EmbeddingResult {
                vector: vec![0.0; 1536],
                tokens_used: 10,
            })
            .collect())
    }

    async fn evaluate(
        &self,
        _question: &str,
        _answer: &str,
        _criteria: &str,
    ) -> Result<EvaluationResponse, LlmError> {
        Ok(EvaluationResponse {
            passed: true,
            reasoning: "Mock evaluation passed".into(),
            confidence: 0.95,
        })
    }

    async fn determine_eval_types(
        &self,
        _question: &str,
    ) -> Result<Vec<crate::evaluation::EvaluationType>, LlmError> {
        Ok(vec![crate::evaluation::EvaluationType::Definitive])
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO OPENAI (STUB)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cliente para OpenAI API
pub struct OpenAiClient {
    api_key: String,
    model: String,
    embedding_model: String,
    client: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "gpt-4-turbo-preview".into(),
            embedding_model: "text-embedding-3-small".into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn decide_action(
        &self,
        _prompt: &AgentPrompt,
        _permissions: &ActionPermissions,
    ) -> Result<AgentAction, LlmError> {
        // TODO: Implementar chamada real à API
        todo!("Implement OpenAI decide_action")
    }

    async fn generate_answer(
        &self,
        _prompt: &AgentPrompt,
        _temperature: f32,
    ) -> Result<LlmResponse, LlmError> {
        // TODO: Implementar chamada real à API
        todo!("Implement OpenAI generate_answer")
    }

    async fn embed(&self, _text: &str) -> Result<EmbeddingResult, LlmError> {
        // TODO: Implementar chamada real à API
        todo!("Implement OpenAI embed")
    }

    async fn embed_batch(&self, _texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError> {
        // TODO: Implementar chamada real à API
        todo!("Implement OpenAI embed_batch")
    }

    async fn evaluate(
        &self,
        _question: &str,
        _answer: &str,
        _criteria: &str,
    ) -> Result<EvaluationResponse, LlmError> {
        // TODO: Implementar chamada real à API
        todo!("Implement OpenAI evaluate")
    }

    async fn determine_eval_types(
        &self,
        _question: &str,
    ) -> Result<Vec<crate::evaluation::EvaluationType>, LlmError> {
        // TODO: Implementar chamada real à API
        todo!("Implement OpenAI determine_eval_types")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client() {
        let client = MockLlmClient::new();
        let prompt = AgentPrompt {
            system: "test".into(),
            user: "test".into(),
            diary: vec![],
        };
        let permissions = ActionPermissions::all();

        let action = client.decide_action(&prompt, &permissions).await.unwrap();
        assert!(action.is_search());
    }
}
