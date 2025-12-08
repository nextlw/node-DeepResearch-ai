// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// AÇÕES DO AGENTE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::types::{Reference, SerpQuery, Url};

/// Cada ação carrega seus próprios dados - impossível ter ação "Search" sem queries
///
/// Este enum implementa o padrão de dados associados, garantindo que cada ação
/// tenha todos os dados necessários para sua execução em compile-time.
///
/// # Exemplo
/// ```rust
/// let action = AgentAction::Search {
///     queries: vec![SerpQuery { q: "rust programming".into(), ..Default::default() }],
///     think: "Need to find information about Rust".into(),
/// };
///
/// match action {
///     AgentAction::Search { queries, think } => {
///         // queries e think sempre disponíveis aqui
///     }
///     // Compiler FORÇA tratamento de todos os outros casos
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone)]
pub enum AgentAction {
    /// Buscar informações na web
    ///
    /// Esta ação expande as queries usando personas cognitivas,
    /// deduplica contra buscas anteriores, e executa as buscas.
    Search {
        /// Queries de busca a serem expandidas
        queries: Vec<SerpQuery>,
        /// Raciocínio do agente para esta ação
        think: String,
    },

    /// Ler uma URL em profundidade
    ///
    /// Esta ação extrai o conteúdo completo de uma página,
    /// processa o HTML, e adiciona ao knowledge base.
    Read {
        /// URLs a serem lidas (limitado a MAX_URLS_PER_STEP)
        urls: Vec<Url>,
        /// Raciocínio do agente para esta ação
        think: String,
    },

    /// Gerar perguntas de gap-closing
    ///
    /// Esta ação analisa o conhecimento atual e identifica
    /// lacunas que precisam ser preenchidas.
    Reflect {
        /// Novas perguntas identificadas
        gap_questions: Vec<String>,
        /// Raciocínio do agente para esta ação
        think: String,
    },

    /// Fornecer a resposta final
    ///
    /// Esta ação passa por avaliação multidimensional.
    /// Se falhar, a resposta é adicionada ao knowledge base como erro.
    Answer {
        /// Resposta proposta
        answer: String,
        /// Referências citadas
        references: Vec<Reference>,
        /// Raciocínio do agente para esta ação
        think: String,
    },

    /// Executar código para processamento de dados
    ///
    /// Esta ação executa código JavaScript em sandbox
    /// para processar e transformar dados.
    Coding {
        /// Código a ser executado
        code: String,
        /// Raciocínio do agente para esta ação
        think: String,
    },
}

impl AgentAction {
    /// Retorna o nome da ação como string
    pub fn name(&self) -> &'static str {
        match self {
            AgentAction::Search { .. } => "search",
            AgentAction::Read { .. } => "read",
            AgentAction::Reflect { .. } => "reflect",
            AgentAction::Answer { .. } => "answer",
            AgentAction::Coding { .. } => "coding",
        }
    }

    /// Retorna o raciocínio (think) da ação
    pub fn think(&self) -> &str {
        match self {
            AgentAction::Search { think, .. } => think,
            AgentAction::Read { think, .. } => think,
            AgentAction::Reflect { think, .. } => think,
            AgentAction::Answer { think, .. } => think,
            AgentAction::Coding { think, .. } => think,
        }
    }

    /// Verifica se é uma ação de busca
    pub fn is_search(&self) -> bool {
        matches!(self, AgentAction::Search { .. })
    }

    /// Verifica se é uma ação de resposta
    pub fn is_answer(&self) -> bool {
        matches!(self, AgentAction::Answer { .. })
    }

    /// Verifica se é uma ação de reflexão
    pub fn is_reflect(&self) -> bool {
        matches!(self, AgentAction::Reflect { .. })
    }
}

/// Entrada do diário do agente.
///
/// O diário registra todas as ações tomadas pelo agente durante uma pesquisa.
/// Isso é útil para:
/// - **Debugging**: Entender o que deu errado
/// - **Contexto para LLM**: O modelo vê o histórico de ações
/// - **Auditoria**: Rastrear o processo de pesquisa
///
/// Cada variante corresponde a um tipo de ação executada.
#[derive(Debug, Clone)]
pub enum DiaryEntry {
    /// Registro de uma busca executada.
    ///
    /// Armazena as queries enviadas, o raciocínio do agente,
    /// e quantas URLs foram encontradas como resultado.
    Search {
        /// Lista de queries que foram executadas na busca.
        queries: Vec<SerpQuery>,
        /// Raciocínio do agente explicando por que fez esta busca.
        think: String,
        /// Quantidade de URLs únicas encontradas nos resultados.
        urls_found: usize,
    },

    /// Registro de URLs lidas.
    ///
    /// Armazena quais páginas foram lidas e extraído o conteúdo.
    Read {
        /// Lista de URLs que foram lidas e processadas.
        urls: Vec<Url>,
        /// Raciocínio do agente para escolher estas URLs.
        think: String,
    },

    /// Registro de uma reflexão executada.
    ///
    /// O agente analisou o conhecimento atual e identificou
    /// novas perguntas para preencher lacunas.
    Reflect {
        /// Novas perguntas identificadas durante a reflexão.
        questions: Vec<String>,
        /// Raciocínio sobre quais lacunas foram encontradas.
        think: String,
    },

    /// Registro de uma resposta que falhou na avaliação.
    ///
    /// Isso é importante para evitar que o agente repita
    /// erros e para ajustar a estratégia de resposta.
    FailedAnswer {
        /// Texto da resposta que foi rejeitada.
        answer: String,
        /// Tipo de avaliação que reprovou a resposta.
        eval_type: crate::evaluation::EvaluationType,
        /// Motivo pelo qual a resposta foi rejeitada.
        reason: String,
    },

    /// Registro de código executado.
    ///
    /// Para ações que envolvem processamento de dados
    /// através de execução de código.
    Coding {
        /// Código que foi executado (JavaScript).
        code: String,
        /// Raciocínio do agente para executar este código.
        think: String,
    },
}

impl DiaryEntry {
    /// Formata a entrada do diário como string legível
    pub fn format(&self) -> String {
        match self {
            DiaryEntry::Search { queries, think, urls_found } => {
                format!(
                    "[SEARCH] {} queries -> {} URLs found\nThink: {}",
                    queries.len(),
                    urls_found,
                    think
                )
            }
            DiaryEntry::Read { urls, think } => {
                format!(
                    "[READ] {} URLs\nThink: {}",
                    urls.len(),
                    think
                )
            }
            DiaryEntry::Reflect { questions, think } => {
                format!(
                    "[REFLECT] {} questions\nThink: {}",
                    questions.len(),
                    think
                )
            }
            DiaryEntry::FailedAnswer { eval_type, reason, .. } => {
                format!(
                    "[FAILED] {:?} evaluation failed\nReason: {}",
                    eval_type,
                    reason
                )
            }
            DiaryEntry::Coding { think, .. } => {
                format!("[CODING]\nThink: {}", think)
            }
        }
    }
}

/// Prompt para o agente decidir a próxima ação
#[derive(Debug, Clone)]
pub struct AgentPrompt {
    /// Prompt do sistema
    pub system: String,
    /// Prompt do usuário
    pub user: String,
    /// Histórico de ações (diário)
    pub diary: Vec<DiaryEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_names() {
        let search = AgentAction::Search {
            queries: vec![],
            think: "test".into(),
        };
        assert_eq!(search.name(), "search");

        let answer = AgentAction::Answer {
            answer: "test".into(),
            references: vec![],
            think: "test".into(),
        };
        assert_eq!(answer.name(), "answer");
    }

    #[test]
    fn test_action_think() {
        let action = AgentAction::Search {
            queries: vec![],
            think: "my reasoning".into(),
        };
        assert_eq!(action.think(), "my reasoning");
    }

    #[test]
    fn test_action_type_checks() {
        let search = AgentAction::Search {
            queries: vec![],
            think: "test".into(),
        };
        assert!(search.is_search());
        assert!(!search.is_answer());
        assert!(!search.is_reflect());
    }
}
