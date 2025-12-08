// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TIPOS COMPARTILHADOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Tipo de URL (alias para String)
pub type Url = String;

/// Idiomas suportados
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Portuguese,
    Spanish,
    German,
    French,
    Italian,
    Japanese,
    Chinese,
    Korean,
    Other,
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

/// Categorias de tópicos
#[derive(Debug, Clone, PartialEq)]
pub enum TopicCategory {
    /// Geral (default)
    General,
    /// Tecnologia
    Technology,
    /// Finanças
    Finance,
    /// Notícias
    News,
    /// Ciência
    Science,
    /// História
    History,
    /// Automotivo (com marca)
    Automotive(String),
    /// Culinária (com tipo)
    Cuisine(String),
    /// Saúde
    Health,
    /// Entretenimento
    Entertainment,
    /// Esportes
    Sports,
    /// Educação
    Education,
}

impl Default for TopicCategory {
    fn default() -> Self {
        Self::General
    }
}

/// Query de busca SERP
#[derive(Debug, Clone, Default)]
pub struct SerpQuery {
    /// Texto da query
    pub q: String,
    /// Filtro de tempo (ex: "qdr:m" para último mês)
    pub tbs: Option<String>,
    /// Localização geográfica
    pub location: Option<String>,
}

/// Referência a uma fonte
#[derive(Debug, Clone)]
pub struct Reference {
    /// URL da fonte
    pub url: String,
    /// Título da página
    pub title: String,
    /// Citação exata (se aplicável)
    pub exact_quote: Option<String>,
    /// Score de relevância (0.0 - 1.0)
    pub relevance_score: Option<f32>,
}

impl Default for Reference {
    fn default() -> Self {
        Self {
            url: String::new(),
            title: String::new(),
            exact_quote: None,
            relevance_score: None,
        }
    }
}

/// Item de conhecimento acumulado
#[derive(Debug, Clone)]
pub struct KnowledgeItem {
    /// Pergunta/contexto
    pub question: String,
    /// Resposta/conteúdo
    pub answer: String,
    /// Tipo de conhecimento
    pub item_type: KnowledgeType,
    /// Referências associadas
    pub references: Vec<Reference>,
}

/// Tipo de item de conhecimento
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeType {
    /// Pergunta e resposta
    Qa,
    /// Informação lateral
    SideInfo,
    /// Histórico de chat
    ChatHistory,
    /// Conteúdo de URL
    Url,
    /// Resultado de código
    Coding,
    /// Erro/falha
    Error,
}

impl KnowledgeType {
    /// Retorna o tipo como string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Qa => "qa",
            Self::SideInfo => "side-info",
            Self::ChatHistory => "chat-history",
            Self::Url => "url",
            Self::Coding => "coding",
            Self::Error => "error",
        }
    }
}

/// Snippet de busca com boost
#[derive(Debug, Clone)]
pub struct BoostedSearchSnippet {
    /// URL do resultado
    pub url: String,
    /// Título do resultado
    pub title: String,
    /// Descrição/snippet
    pub description: String,
    /// Peso inicial
    pub weight: f32,
    /// Boost de frequência
    pub freq_boost: f32,
    /// Boost de hostname
    pub hostname_boost: f32,
    /// Boost de path
    pub path_boost: f32,
    /// Boost de reranking Jina
    pub jina_rerank_boost: f32,
    /// Score final calculado
    pub final_score: f32,
    /// Score normalizado
    pub score: f32,
    /// Descrição merged
    pub merged: String,
}

impl Default for BoostedSearchSnippet {
    fn default() -> Self {
        Self {
            url: String::new(),
            title: String::new(),
            description: String::new(),
            weight: 1.0,
            freq_boost: 1.0,
            hostname_boost: 1.0,
            path_boost: 1.0,
            jina_rerank_boost: 1.0,
            final_score: 1.0,
            score: 1.0,
            merged: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serp_query_default() {
        let query = SerpQuery::default();
        assert!(query.q.is_empty());
        assert!(query.tbs.is_none());
        assert!(query.location.is_none());
    }

    #[test]
    fn test_knowledge_type_as_str() {
        assert_eq!(KnowledgeType::Qa.as_str(), "qa");
        assert_eq!(KnowledgeType::Url.as_str(), "url");
        assert_eq!(KnowledgeType::Error.as_str(), "error");
    }

    #[test]
    fn test_topic_category_default() {
        let topic = TopicCategory::default();
        assert_eq!(topic, TopicCategory::General);
    }

    #[test]
    fn test_language_default() {
        let lang = Language::default();
        assert_eq!(lang, Language::English);
    }
}
