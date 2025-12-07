// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SISTEMA DE AVALIAÇÃO MULTIDIMENSIONAL
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod types;
mod evaluators;
mod pipeline;

pub use types::*;
pub use evaluators::*;
pub use pipeline::*;

use std::sync::Arc;
use std::time::Duration;

use crate::types::{KnowledgeItem, TopicCategory};
use crate::llm::LlmClient;

/// Contexto para avaliação
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Categoria do tópico
    pub topic: TopicCategory,
    /// Itens de conhecimento acumulados
    pub knowledge_items: Vec<KnowledgeItem>,
}

/// Par de prompts (sistema + usuário)
#[derive(Debug, Clone)]
pub struct PromptPair {
    pub system: String,
    pub user: String,
}

/// Tipos de avaliação - enum garante que não existem tipos "inventados"
///
/// Cada tipo representa uma dimensão diferente de qualidade da resposta:
/// - Definitive: A resposta é confiante, sem hedging excessivo?
/// - Freshness: A informação é recente o suficiente para o tópico?
/// - Plurality: Se pediu N exemplos, tem N exemplos?
/// - Completeness: Todos os aspectos da pergunta foram cobertos?
/// - Strict: Avaliação brutal - tem insights reais e profundos?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvaluationType {
    /// Verifica se a resposta é confiante e definitiva
    Definitive,
    /// Verifica se a informação é recente o suficiente
    Freshness,
    /// Verifica se a quantidade de itens está correta
    Plurality,
    /// Verifica se todos os aspectos foram cobertos
    Completeness,
    /// Avaliação brutal - rejeita respostas mediocres
    Strict,
}

impl EvaluationType {
    /// Retorna o nome do tipo como string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Definitive => "definitive",
            Self::Freshness => "freshness",
            Self::Plurality => "plurality",
            Self::Completeness => "completeness",
            Self::Strict => "strict",
        }
    }

    /// Retorna configuração padrão para cada tipo
    pub fn default_config(self) -> EvaluationConfig {
        match self {
            Self::Definitive => EvaluationConfig {
                eval_type: self,
                max_retries: 2,
                timeout: Duration::from_secs(30),
                weight: 1.0,
            },
            Self::Freshness => EvaluationConfig {
                eval_type: self,
                max_retries: 1,
                timeout: Duration::from_secs(20),
                weight: 0.8,
            },
            Self::Plurality => EvaluationConfig {
                eval_type: self,
                max_retries: 1,
                timeout: Duration::from_secs(15),
                weight: 0.6,
            },
            Self::Completeness => EvaluationConfig {
                eval_type: self,
                max_retries: 2,
                timeout: Duration::from_secs(25),
                weight: 0.9,
            },
            Self::Strict => EvaluationConfig {
                eval_type: self,
                max_retries: 3,
                timeout: Duration::from_secs(45),
                weight: 1.5,  // Mais importante
            },
        }
    }

    /// Determina freshness threshold baseado no tópico
    pub fn freshness_threshold(&self, topic: &TopicCategory) -> Duration {
        match topic {
            TopicCategory::Finance => Duration::from_secs(60 * 60 * 2),       // 2 horas
            TopicCategory::News => Duration::from_secs(60 * 60 * 24),         // 1 dia
            TopicCategory::Technology => Duration::from_secs(60 * 60 * 24 * 30), // 30 dias
            TopicCategory::Science => Duration::from_secs(60 * 60 * 24 * 365),   // 1 ano
            TopicCategory::History => Duration::MAX,                           // Sem limite
            _ => Duration::from_secs(60 * 60 * 24 * 7),                        // 7 dias padrão
        }
    }
}

impl std::fmt::Display for EvaluationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuração específica para cada tipo de avaliação
#[derive(Debug, Clone)]
pub struct EvaluationConfig {
    pub eval_type: EvaluationType,
    pub max_retries: u8,
    pub timeout: Duration,
    pub weight: f32,  // Importância relativa (0.0 - 2.0)
}

/// Resultado de uma avaliação individual
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub eval_type: EvaluationType,
    pub passed: bool,
    pub confidence: f32,        // 0.0 - 1.0
    pub reasoning: String,
    pub suggestions: Vec<String>,
    pub duration: Duration,
}

impl EvaluationResult {
    /// Cria um resultado de sucesso
    pub fn success(eval_type: EvaluationType, reasoning: String, confidence: f32) -> Self {
        Self {
            eval_type,
            passed: true,
            confidence,
            reasoning,
            suggestions: vec![],
            duration: Duration::ZERO,
        }
    }

    /// Cria um resultado de falha
    pub fn failure(
        eval_type: EvaluationType,
        reasoning: String,
        suggestions: Vec<String>,
        confidence: f32,
    ) -> Self {
        Self {
            eval_type,
            passed: false,
            confidence,
            reasoning,
            suggestions,
            duration: Duration::ZERO,
        }
    }

    /// Define a duração da avaliação
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

/// Erro de avaliação
#[derive(Debug, Clone)]
pub enum EvalError {
    LlmError(String),
    Timeout,
    ParseError(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LlmError(msg) => write!(f, "LLM error: {}", msg),
            Self::Timeout => write!(f, "Evaluation timeout"),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for EvalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluation_type_as_str() {
        assert_eq!(EvaluationType::Definitive.as_str(), "definitive");
        assert_eq!(EvaluationType::Freshness.as_str(), "freshness");
        assert_eq!(EvaluationType::Plurality.as_str(), "plurality");
        assert_eq!(EvaluationType::Completeness.as_str(), "completeness");
        assert_eq!(EvaluationType::Strict.as_str(), "strict");
    }

    #[test]
    fn test_default_configs() {
        let definitive = EvaluationType::Definitive.default_config();
        assert_eq!(definitive.weight, 1.0);
        assert_eq!(definitive.max_retries, 2);

        let strict = EvaluationType::Strict.default_config();
        assert_eq!(strict.weight, 1.5);
        assert_eq!(strict.max_retries, 3);
    }

    #[test]
    fn test_freshness_threshold() {
        let eval = EvaluationType::Freshness;

        let finance = eval.freshness_threshold(&TopicCategory::Finance);
        assert_eq!(finance.as_secs(), 60 * 60 * 2); // 2 horas

        let history = eval.freshness_threshold(&TopicCategory::History);
        assert_eq!(history, Duration::MAX);
    }

    #[test]
    fn test_evaluation_result() {
        let success = EvaluationResult::success(
            EvaluationType::Definitive,
            "Good answer".into(),
            0.9,
        );
        assert!(success.passed);
        assert_eq!(success.confidence, 0.9);

        let failure = EvaluationResult::failure(
            EvaluationType::Freshness,
            "Outdated".into(),
            vec!["Update data".into()],
            0.3,
        );
        assert!(!failure.passed);
        assert_eq!(failure.suggestions.len(), 1);
    }
}
