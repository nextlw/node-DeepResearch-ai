// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SISTEMA DE PERSONAS COGNITIVAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod all_personas;
mod orchestrator;
mod traits;

pub use all_personas::*;
pub use orchestrator::*;
pub use traits::*;

use chrono::Utc;

use crate::types::{Language, SerpQuery, TopicCategory};

/// Contexto compartilhado para expansão de queries
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// Query original do usuário
    pub original_query: String,
    /// Intenção do usuário (interpretada)
    pub user_intent: String,
    /// Snippets de contexto das buscas anteriores
    pub soundbites: Vec<String>,
    /// Data atual
    pub current_date: chrono::NaiveDate,
    /// Idioma detectado
    pub detected_language: Language,
    /// Tópico detectado
    pub detected_topic: TopicCategory,
}

impl Default for QueryContext {
    fn default() -> Self {
        Self {
            original_query: String::new(),
            user_intent: String::new(),
            soundbites: Vec::new(),
            current_date: Utc::now().date_naive(),
            detected_language: Language::English,
            detected_topic: TopicCategory::General,
        }
    }
}

/// Query com peso e origem
#[derive(Debug, Clone)]
pub struct WeightedQuery {
    /// Query gerada
    pub query: SerpQuery,
    /// Peso da query (0.0 - 2.0)
    pub weight: f32,
    /// Nome da persona que gerou
    pub source_persona: &'static str,
}

/// Funções auxiliares para extração de tópico
pub fn extract_main_topic(query: &str) -> String {
    // Implementação simplificada - remove stop words e extrai substantivos principais
    let stop_words = [
        "the", "a", "an", "is", "are", "was", "were", "what", "how", "why", "when", "where",
    ];

    query
        .split_whitespace()
        .filter(|word| !stop_words.contains(&word.to_lowercase().as_str()))
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Nega uma suposição (para Reality Skepticalist)
pub fn negate_assumption(query: &str) -> String {
    // Implementação simplificada
    let topic = extract_main_topic(query);

    if query.contains("best") {
        format!("{} worst", topic)
    } else if query.contains("good") {
        format!("{} bad", topic)
    } else if query.contains("benefit") {
        format!("{} drawback", topic)
    } else {
        topic
    }
}

/// Traduz para alemão (simplificado)
pub fn translate_to_german(query: &str) -> String {
    // Em produção, usaria uma API de tradução
    let topic = extract_main_topic(query);
    format!("{} Erfahrungen Probleme", topic)
}

/// Traduz para japonês (simplificado)
pub fn translate_to_japanese(query: &str) -> String {
    // Em produção, usaria uma API de tradução
    let topic = extract_main_topic(query);
    format!("{} 問題 レビュー", topic)
}

/// Traduz para italiano (simplificado)
pub fn translate_to_italian(query: &str) -> String {
    // Em produção, usaria uma API de tradução
    let topic = extract_main_topic(query);
    format!("{} problemi recensioni", topic)
}

/// Traduz para francês (simplificado)
pub fn translate_to_french(query: &str) -> String {
    // Em produção, usaria uma API de tradução
    let topic = extract_main_topic(query);
    format!("{} problèmes avis", topic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_main_topic() {
        let topic = extract_main_topic("What is the best programming language");
        assert!(!topic.contains("what"));
        assert!(!topic.contains("is"));
        assert!(!topic.contains("the"));
    }

    #[test]
    fn test_negate_assumption() {
        let negated = negate_assumption("best programming language");
        assert!(negated.contains("worst"));

        let negated2 = negate_assumption("good practices");
        assert!(negated2.contains("bad"));
    }

    #[test]
    fn test_query_context_default() {
        let ctx = QueryContext::default();
        assert!(ctx.original_query.is_empty());
        assert!(ctx.soundbites.is_empty());
    }
}
