// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SISTEMA DE PERSONAS COGNITIVAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod all_personas;
mod metrics;
mod orchestrator;
mod registry;
mod traits;
mod validator;

pub use all_personas::*;
pub use metrics::*;
pub use orchestrator::*;
pub use registry::*;
pub use traits::*;
pub use validator::*;

use chrono::Utc;
use uuid::Uuid;

use crate::types::{Language, SerpQuery, TopicCategory};

/// Contexto compartilhado para expansão de queries
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// ID único desta execução (para rastreamento)
    pub execution_id: Uuid,
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
            execution_id: Uuid::new_v4(),
            original_query: String::new(),
            user_intent: String::new(),
            soundbites: Vec::new(),
            current_date: Utc::now().date_naive(),
            detected_language: Language::English,
            detected_topic: TopicCategory::General,
        }
    }
}

impl QueryContext {
    /// Cria um novo contexto com uma query específica
    pub fn with_query(query: impl Into<String>) -> Self {
        Self {
            original_query: query.into(),
            ..Default::default()
        }
    }

    /// Cria um novo contexto com ID de execução específico
    pub fn with_execution_id(execution_id: Uuid) -> Self {
        Self {
            execution_id,
            ..Default::default()
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
    // Remove stop words preservando o contexto completo da query
    // NÃO limita o número de palavras para manter a semântica intacta
    let stop_words = [
        // Inglês - artigos e pronomes
        "the", "a", "an", "this", "that", "these", "those",
        // Inglês - verbos auxiliares
        "is", "are", "was", "were", "be", "been", "being",
        "do", "does", "did", "have", "has", "had",
        "will", "would", "could", "should", "can", "may", "might",
        // Inglês - interrogativos
        "what", "how", "why", "when", "where", "which", "who", "whom",
        // Inglês - preposições comuns no início
        "to", "for", "of", "in", "on", "at", "by", "with",
        // Inglês - outros
        "i", "me", "my", "you", "your", "we", "our", "it", "its",
        "please", "tell", "explain", "describe", "show",
        // Português - artigos
        "o", "a", "os", "as", "um", "uma", "uns", "umas",
        // Português - verbos auxiliares
        "é", "são", "foi", "foram", "ser", "estar", "está", "estão",
        // Português - interrogativos
        "que", "qual", "quais", "como", "porque", "por", "onde", "quando", "quem",
        // Português - preposições
        "de", "do", "da", "dos", "das", "em", "no", "na", "nos", "nas",
        "para", "com", "sem", "sobre",
        // Português - outros
        "eu", "me", "meu", "minha", "você", "seu", "sua",
        "por favor", "explique", "descreva", "mostre",
    ];

    let result: String = query
        .split_whitespace()
        .filter(|word| {
            let lower = word.to_lowercase();
            // Remove pontuação para comparação
            let clean: &str = lower.trim_matches(|c: char| !c.is_alphanumeric());
            !stop_words.contains(&clean)
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Se a filtragem removeu tudo, retorna a query original sem stop words iniciais
    if result.is_empty() {
        query.to_string()
    } else {
        result
    }
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
        // Teste básico - remove stop words
        let topic = extract_main_topic("What is the best programming language");
        assert!(!topic.contains("what"));
        assert!(!topic.contains("is"));
        assert!(!topic.contains("the"));
        assert!(topic.contains("best"));
        assert!(topic.contains("programming"));
        assert!(topic.contains("language"));
    }

    #[test]
    fn test_extract_main_topic_preserves_context() {
        // Teste crítico: NÃO deve cortar a query!
        let topic = extract_main_topic("What is the best programming language for web development in 2024");
        assert!(topic.contains("best"));
        assert!(topic.contains("programming"));
        assert!(topic.contains("language"));
        assert!(topic.contains("web"));
        assert!(topic.contains("development"));
        assert!(topic.contains("2024"));
    }

    #[test]
    fn test_extract_main_topic_portuguese() {
        let topic = extract_main_topic("Qual é a melhor linguagem de programação para iniciantes");
        assert!(topic.contains("melhor"));
        assert!(topic.contains("linguagem"));
        assert!(topic.contains("programação"));
        assert!(topic.contains("iniciantes"));
        // Stop words removidas
        assert!(!topic.to_lowercase().contains("qual"));
        assert!(!topic.to_lowercase().contains(" é "));
    }

    #[test]
    fn test_extract_main_topic_long_query() {
        // Query longa não deve ser cortada
        let topic = extract_main_topic("How to implement authentication with OAuth2 and JWT tokens in a React application with Node.js backend");
        assert!(topic.contains("implement"));
        assert!(topic.contains("authentication"));
        assert!(topic.contains("OAuth2"));
        assert!(topic.contains("JWT"));
        assert!(topic.contains("tokens"));
        assert!(topic.contains("React"));
        assert!(topic.contains("application"));
        assert!(topic.contains("Node.js"));
        assert!(topic.contains("backend"));
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
