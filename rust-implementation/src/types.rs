// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TIPOS COMPARTILHADOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Tipo de URL (alias para String)
pub type Url = String;

/// Idiomas suportados pelo sistema de pesquisa.
///
/// O idioma afeta como as queries são construídas e como os resultados
/// são filtrados e apresentados. Por exemplo, uma pesquisa em Português
/// priorizará fontes em português e formatará datas no padrão brasileiro.
///
/// # Exemplo
/// ```rust
/// use deep_research::types::Language;
///
/// let lang = Language::Portuguese;
/// assert_ne!(lang, Language::English);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Inglês - Idioma padrão, maior cobertura de fontes
    English,
    /// Português - Inclui variantes BR e PT
    Portuguese,
    /// Espanhol - Inclui variantes ES e LATAM
    Spanish,
    /// Alemão - Fontes da Alemanha, Áustria e Suíça
    German,
    /// Francês - Fontes da França, Bélgica, Canadá e África
    French,
    /// Italiano - Fontes da Itália e Suíça italiana
    Italian,
    /// Japonês - Fontes do Japão (requer fontes específicas)
    Japanese,
    /// Chinês - Simplificado e tradicional
    Chinese,
    /// Coreano - Fontes da Coreia do Sul
    Korean,
    /// Outro idioma não listado - Usa heurísticas genéricas
    Other,
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

impl Language {
    /// Cria Language a partir de string (ex: "pt", "pt-BR", "Portuguese")
    pub fn from_str(s: &str) -> Self {
        let s_lower = s.to_lowercase();
        match s_lower.as_str() {
            "pt" | "pt-br" | "portuguese" | "portugues" | "português" => Self::Portuguese,
            "en" | "en-us" | "english" | "ingles" | "inglês" => Self::English,
            "es" | "es-es" | "spanish" | "espanhol" | "español" => Self::Spanish,
            "de" | "de-de" | "german" | "alemao" | "alemão" | "deutsch" => Self::German,
            "fr" | "fr-fr" | "french" | "frances" | "français" => Self::French,
            "it" | "it-it" | "italian" | "italiano" => Self::Italian,
            "ja" | "japanese" | "japones" | "japonês" => Self::Japanese,
            "zh" | "chinese" | "chines" | "chinês" => Self::Chinese,
            "ko" | "korean" | "coreano" => Self::Korean,
            _ => Self::English,
        }
    }

    /// Retorna a instrução de idioma para o LLM
    pub fn llm_instruction(&self) -> &'static str {
        match self {
            Self::Portuguese => "IMPORTANTE: Responda SEMPRE em Português do Brasil (PT-BR). NÃO use português de Portugal. Use expressões, vocabulário e gramática brasileira. Todas as respostas, análises e explicações devem ser em português brasileiro.",
            Self::Spanish => "IMPORTANTE: Responda SIEMPRE en Español. Todas las respuestas, análisis y explicaciones deben ser en español.",
            Self::German => "WICHTIG: Antworten Sie IMMER auf Deutsch. Alle Antworten, Analysen und Erklärungen müssen auf Deutsch sein.",
            Self::French => "IMPORTANT: Répondez TOUJOURS en Français. Toutes les réponses, analyses et explications doivent être en français.",
            Self::Italian => "IMPORTANTE: Rispondi SEMPRE in Italiano. Tutte le risposte, analisi e spiegazioni devono essere in italiano.",
            Self::Japanese => "重要：常に日本語で回答してください。すべての回答、分析、説明は日本語である必要があります。",
            Self::Chinese => "重要：请始终使用中文回答。所有回答、分析和解释都必须是中文。",
            Self::Korean => "중요: 항상 한국어로 답변해 주세요. 모든 답변, 분석, 설명은 한국어로 작성되어야 합니다.",
            _ => "", // English não precisa de instrução especial
        }
    }

    /// Nome do idioma para exibição
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português",
            Self::Spanish => "Español",
            Self::German => "Deutsch",
            Self::French => "Français",
            Self::Italian => "Italiano",
            Self::Japanese => "日本語",
            Self::Chinese => "中文",
            Self::Korean => "한국어",
            Self::Other => "Other",
        }
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
    /// Chunk da resposta que fez match com esta referência
    pub answer_chunk: Option<String>,
    /// Posição (start, end) do chunk na resposta original
    pub answer_position: Option<(usize, usize)>,
}

impl Default for Reference {
    fn default() -> Self {
        Self {
            url: String::new(),
            title: String::new(),
            exact_quote: None,
            relevance_score: None,
            answer_chunk: None,
            answer_position: None,
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
    /// Histórico de sessões anteriores
    History,
    /// Informação fornecida diretamente pelo usuário
    UserProvided,
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
            Self::History => "history",
            Self::UserProvided => "user-provided",
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
