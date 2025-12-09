// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONTEXTO DO AGENTE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::agent_analyzer::AgentAnalysis;
use super::DiaryEntry;
use crate::types::{BoostedSearchSnippet, KnowledgeItem, KnowledgeType};

/// Contexto acumulado durante a execução do agente
///
/// Armazena todo o estado mutável da pesquisa, incluindo:
/// - URLs coletadas e visitadas
/// - Conhecimento acumulado
/// - Perguntas de gap
/// - Histórico de ações (diário)
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// Pergunta original do usuário
    pub original_question: String,

    /// Perguntas de gap a serem respondidas
    pub gap_questions: Vec<String>,

    /// Conhecimento acumulado durante a pesquisa
    pub knowledge: Vec<KnowledgeItem>,

    /// Todas as URLs coletadas (com scores)
    pub collected_urls: Vec<BoostedSearchSnippet>,

    /// URLs rankeadas e ponderadas
    pub weighted_urls: Vec<BoostedSearchSnippet>,

    /// URLs já visitadas (lidas)
    pub visited_urls: Vec<String>,

    /// URLs que falharam na leitura
    pub bad_urls: Vec<String>,

    /// Snippets de contexto das buscas
    pub snippets: Vec<String>,

    /// Histórico de ações
    pub diary: Vec<DiaryEntry>,

    /// Contador de passos totais
    pub total_step: usize,

    /// Se permite resposta direta no step 1
    pub allow_direct_answer: bool,

    /// Keywords já utilizadas em buscas
    pub all_keywords: Vec<String>,

    /// Embeddings das queries já executadas (para deduplicação SIMD)
    pub executed_query_embeddings: Vec<Vec<f32>>,

    /// Queries já executadas (texto para referência)
    pub executed_queries: Vec<String>,

    /// Hints de melhoria do AgentAnalyzer (para injetar no prompt)
    pub improvement_hints: Vec<String>,

    /// Última análise de erro realizada (para display na TUI)
    pub last_agent_analysis: Option<AgentAnalysis>,
}

impl AgentContext {
    /// Cria um novo contexto vazio
    pub fn new() -> Self {
        Self {
            original_question: String::new(),
            gap_questions: Vec::new(),
            knowledge: Vec::new(),
            collected_urls: Vec::new(),
            weighted_urls: Vec::new(),
            visited_urls: Vec::new(),
            bad_urls: Vec::new(),
            snippets: Vec::new(),
            diary: Vec::new(),
            total_step: 0,
            allow_direct_answer: false, // Forçar pesquisa antes de responder
            all_keywords: Vec::new(),
            executed_query_embeddings: Vec::new(),
            executed_queries: Vec::new(),
            improvement_hints: Vec::new(),
            last_agent_analysis: None,
        }
    }

    /// Adiciona embedding de uma query executada
    pub fn add_executed_query(&mut self, query: String, embedding: Vec<f32>) {
        self.executed_queries.push(query);
        self.executed_query_embeddings.push(embedding);
    }

    /// Adiciona múltiplos embeddings de queries executadas
    pub fn add_executed_queries(&mut self, queries: Vec<String>, embeddings: Vec<Vec<f32>>) {
        self.executed_queries.extend(queries);
        self.executed_query_embeddings.extend(embeddings);
    }

    /// Retorna a pergunta atual sendo processada
    pub fn current_question(&self) -> &str {
        if self.gap_questions.is_empty() {
            &self.original_question
        } else {
            let idx = self.total_step % self.gap_questions.len();
            &self.gap_questions[idx]
        }
    }

    /// Adiciona URLs ao contexto
    pub fn add_urls(&mut self, urls: Vec<BoostedSearchSnippet>) {
        for url in urls {
            // Evita duplicatas
            if !self.collected_urls.iter().any(|u| u.url == url.url) {
                self.collected_urls.push(url);
            }
        }
    }

    /// Adiciona uma URL simples
    pub fn add_url(&mut self, url: String, title: String, description: String) {
        let snippet = BoostedSearchSnippet {
            url,
            title,
            description: description.clone(),
            weight: 1.0,
            freq_boost: 1.0,
            hostname_boost: 1.0,
            path_boost: 1.0,
            jina_rerank_boost: 1.0,
            final_score: 1.0,
            score: 1.0,
            merged: description,
        };
        self.add_urls(vec![snippet]);
    }

    /// Adiciona snippets de contexto
    pub fn add_snippets(&mut self, snippets: Vec<String>) {
        self.snippets.extend(snippets);
    }

    /// Adiciona um item de conhecimento
    pub fn add_knowledge(&mut self, item: KnowledgeItem) {
        // Evita duplicatas baseado na pergunta
        if !self
            .knowledge
            .iter()
            .any(|k| k.question == item.question && k.answer == item.answer)
        {
            self.knowledge.push(item);
        }
    }

    /// Adiciona conhecimento de Q&A
    pub fn add_qa_knowledge(&mut self, question: String, answer: String) {
        self.add_knowledge(KnowledgeItem {
            question,
            answer,
            item_type: KnowledgeType::Qa,
            references: Vec::new(),
        });
    }

    /// Verifica se uma URL já foi visitada
    pub fn is_url_visited(&self, url: &str) -> bool {
        self.visited_urls.contains(&url.to_string())
    }

    /// Verifica se uma URL é ruim (falhou anteriormente)
    ///
    /// NOTA: URLs de redes sociais, paywalls e JS-heavy agora são
    /// automaticamente roteadas para Jina Reader em vez de bloqueadas.
    /// Veja `search::url_requires_jina()` para a lista de domínios.
    pub fn is_url_bad(&self, url: &str) -> bool {
        self.bad_urls.contains(&url.to_string())
    }

    /// Retorna o número total de URLs coletadas
    pub fn total_urls(&self) -> usize {
        self.collected_urls.len()
    }

    /// Retorna o número de URLs disponíveis (não visitadas, não ruins)
    pub fn available_urls(&self) -> usize {
        self.collected_urls
            .iter()
            .filter(|u| !self.is_url_visited(&u.url) && !self.is_url_bad(&u.url))
            .count()
    }

    /// Formata o conhecimento para incluir no prompt
    pub fn format_knowledge(&self) -> String {
        self.knowledge
            .iter()
            .enumerate()
            .map(|(i, k)| {
                format!(
                    "{}. [{}] Q: {}\n   A: {}",
                    i + 1,
                    k.item_type.as_str(),
                    k.question,
                    k.answer
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Formata o diário para análise de erros
    pub fn format_diary(&self) -> String {
        self.diary
            .iter()
            .enumerate()
            .map(|(i, entry)| format!("Step {}: {}", i + 1, entry.format()))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Limpa o contexto para reutilização
    pub fn reset(&mut self) {
        self.original_question.clear();
        self.gap_questions.clear();
        self.knowledge.clear();
        self.collected_urls.clear();
        self.weighted_urls.clear();
        self.visited_urls.clear();
        self.bad_urls.clear();
        self.snippets.clear();
        self.diary.clear();
        self.total_step = 0;
        self.allow_direct_answer = false; // Forçar pesquisa antes de responder
        self.all_keywords.clear();
        self.executed_query_embeddings.clear();
        self.executed_queries.clear();
        self.improvement_hints.clear();
        self.last_agent_analysis = None;
    }

    /// Adiciona um hint de melhoria do AgentAnalyzer
    pub fn add_improvement_hint(&mut self, hint: String) {
        // Evita hints duplicados
        if !self.improvement_hints.contains(&hint) {
            self.improvement_hints.push(hint);
        }
    }

    /// Define a última análise do agente
    pub fn set_agent_analysis(&mut self, analysis: AgentAnalysis) {
        self.last_agent_analysis = Some(analysis);
    }

    /// Verifica se há hints de melhoria disponíveis
    pub fn has_improvement_hints(&self) -> bool {
        !self.improvement_hints.is_empty()
    }
}

impl Default for AgentContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = AgentContext::new();
        assert!(ctx.original_question.is_empty());
        assert!(ctx.gap_questions.is_empty());
        assert!(ctx.knowledge.is_empty());
        assert_eq!(ctx.total_step, 0);
    }

    #[test]
    fn test_add_urls_dedup() {
        let mut ctx = AgentContext::new();

        ctx.add_url(
            "https://example.com".into(),
            "Example".into(),
            "Description".into(),
        );
        ctx.add_url(
            "https://example.com".into(),
            "Example".into(),
            "Description".into(),
        );

        assert_eq!(ctx.collected_urls.len(), 1);
    }

    #[test]
    fn test_current_question_rotation() {
        let mut ctx = AgentContext::new();
        ctx.original_question = "Original".into();
        ctx.gap_questions = vec!["Q1".into(), "Q2".into(), "Q3".into()];

        ctx.total_step = 0;
        assert_eq!(ctx.current_question(), "Q1");

        ctx.total_step = 1;
        assert_eq!(ctx.current_question(), "Q2");

        ctx.total_step = 2;
        assert_eq!(ctx.current_question(), "Q3");

        ctx.total_step = 3;
        assert_eq!(ctx.current_question(), "Q1"); // Volta ao início
    }

    #[test]
    fn test_url_status() {
        let mut ctx = AgentContext::new();

        ctx.visited_urls.push("https://visited.com".into());
        ctx.bad_urls.push("https://bad.com".into());

        assert!(ctx.is_url_visited("https://visited.com"));
        assert!(!ctx.is_url_visited("https://other.com"));

        assert!(ctx.is_url_bad("https://bad.com"));
        assert!(!ctx.is_url_bad("https://other.com"));
    }

    #[test]
    fn test_improvement_hints() {
        let mut ctx = AgentContext::new();

        // Inicialmente vazio
        assert!(!ctx.has_improvement_hints());
        assert!(ctx.improvement_hints.is_empty());

        // Adicionar hint
        ctx.add_improvement_hint("Avoid repetitive searches".into());
        assert!(ctx.has_improvement_hints());
        assert_eq!(ctx.improvement_hints.len(), 1);

        // Adicionar outro hint
        ctx.add_improvement_hint("Focus on reliable sources".into());
        assert_eq!(ctx.improvement_hints.len(), 2);

        // Tentar adicionar hint duplicado (não deve adicionar)
        ctx.add_improvement_hint("Avoid repetitive searches".into());
        assert_eq!(ctx.improvement_hints.len(), 2);
    }

    #[test]
    fn test_agent_analysis() {
        let mut ctx = AgentContext::new();

        // Inicialmente nenhuma análise
        assert!(ctx.last_agent_analysis.is_none());

        // Definir análise
        let analysis = AgentAnalysis {
            recap: "Test recap".into(),
            blame: "Test blame".into(),
            improvement: "Test improvement".into(),
            duration_ms: Some(100),
        };
        ctx.set_agent_analysis(analysis);

        assert!(ctx.last_agent_analysis.is_some());
        let stored = ctx.last_agent_analysis.as_ref().unwrap();
        assert_eq!(stored.recap, "Test recap");
        assert_eq!(stored.blame, "Test blame");
        assert_eq!(stored.improvement, "Test improvement");
    }

    #[test]
    fn test_reset_clears_hints_and_analysis() {
        let mut ctx = AgentContext::new();

        // Adicionar dados
        ctx.add_improvement_hint("Test hint".into());
        ctx.set_agent_analysis(AgentAnalysis {
            recap: "recap".into(),
            blame: "blame".into(),
            improvement: "improvement".into(),
            duration_ms: None,
        });

        assert!(ctx.has_improvement_hints());
        assert!(ctx.last_agent_analysis.is_some());

        // Reset deve limpar tudo
        ctx.reset();

        assert!(!ctx.has_improvement_hints());
        assert!(ctx.last_agent_analysis.is_none());
    }
}
