// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ORQUESTRADOR DE PERSONAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use rayon::prelude::*;

use super::{
    CognitivePersona, ComparativeThinker, DetailAnalyst, ExpertSkeptic, Globalizer,
    HistoricalResearcher, QueryContext, RealitySkepticalist, TemporalContext, WeightedQuery,
};

/// Orquestrador que gerencia todas as personas cognitivas
///
/// O orquestrador é responsável por:
/// 1. Manter a lista de personas ativas
/// 2. Expandir queries usando todas as personas em paralelo
/// 3. Filtrar personas não aplicáveis ao contexto
/// 4. Coletar e retornar queries com pesos
///
/// # Paralelismo
///
/// O orquestrador usa Rayon para executar as personas em paralelo,
/// aproveitando todos os cores da CPU. Isso é muito mais eficiente
/// que Promise.all do JavaScript, que é single-threaded.
///
/// # Exemplo
///
/// ```rust
/// let orchestrator = PersonaOrchestrator::new();
/// let context = QueryContext::default();
///
/// // Expande uma query usando todas as personas em paralelo
/// let expanded = orchestrator.expand_query_parallel("rust programming", &context);
///
/// // Resultado: 7 queries de perspectivas diferentes
/// for wq in expanded {
///     println!("[{}] {} (weight: {})", wq.source_persona, wq.query.q, wq.weight);
/// }
/// ```
pub struct PersonaOrchestrator {
    personas: Vec<Box<dyn CognitivePersona>>,
}

impl PersonaOrchestrator {
    /// Cria um novo orquestrador com todas as 7 personas padrão
    pub fn new() -> Self {
        Self {
            personas: vec![
                Box::new(ExpertSkeptic),
                Box::new(DetailAnalyst),
                Box::new(HistoricalResearcher),
                Box::new(ComparativeThinker),
                Box::new(TemporalContext),
                Box::new(Globalizer),
                Box::new(RealitySkepticalist),
            ],
        }
    }

    /// Cria um orquestrador com personas customizadas
    pub fn with_personas(personas: Vec<Box<dyn CognitivePersona>>) -> Self {
        Self { personas }
    }

    /// Cria um orquestrador com apenas personas técnicas
    pub fn technical() -> Self {
        Self {
            personas: vec![
                Box::new(DetailAnalyst),
                Box::new(ComparativeThinker),
                Box::new(TemporalContext),
            ],
        }
    }

    /// Cria um orquestrador com apenas personas investigativas
    pub fn investigative() -> Self {
        Self {
            personas: vec![
                Box::new(ExpertSkeptic),
                Box::new(RealitySkepticalist),
                Box::new(HistoricalResearcher),
            ],
        }
    }

    /// Expande uma query usando TODAS as personas em PARALELO
    ///
    /// # Argumentos
    ///
    /// * `original` - Query original a ser expandida
    /// * `context` - Contexto compartilhado com informações adicionais
    ///
    /// # Retorno
    ///
    /// Um vetor de `WeightedQuery`, uma para cada persona aplicável.
    ///
    /// # Performance
    ///
    /// Esta função usa `par_iter()` do Rayon, que distribui o trabalho
    /// entre múltiplas threads. Para 7 personas em uma CPU de 8 cores,
    /// todas as expansões rodam verdadeiramente em paralelo.
    pub fn expand_query_parallel(
        &self,
        original: &str,
        context: &QueryContext,
    ) -> Vec<WeightedQuery> {
        // Rayon: paralelismo real em múltiplos cores
        self.personas
            .par_iter() // Iterator paralelo!
            .filter(|persona| persona.is_applicable(context))
            .map(|persona| {
                let query = persona.expand_query(original, context);
                WeightedQuery {
                    query,
                    weight: persona.weight(),
                    source_persona: persona.name(),
                }
            })
            .collect()
    }

    /// Expande múltiplas queries de entrada
    ///
    /// Útil quando há múltiplas queries iniciais (ex: do usuário ou de reflexão).
    /// Usa dois níveis de paralelismo:
    /// 1. Paralelo entre queries de entrada
    /// 2. Paralelo entre personas para cada query
    ///
    /// # Retorno
    ///
    /// Um vetor flat com todas as queries expandidas.
    pub fn expand_batch(&self, queries: &[String], context: &QueryContext) -> Vec<WeightedQuery> {
        queries
            .par_iter() // Paralelo no nível das queries
            .flat_map(|q| self.expand_query_parallel(q, context))
            .collect()
    }

    /// Expande query de forma sequencial (para debugging ou testes)
    pub fn expand_query_sequential(
        &self,
        original: &str,
        context: &QueryContext,
    ) -> Vec<WeightedQuery> {
        self.personas
            .iter()
            .filter(|persona| persona.is_applicable(context))
            .map(|persona| {
                let query = persona.expand_query(original, context);
                WeightedQuery {
                    query,
                    weight: persona.weight(),
                    source_persona: persona.name(),
                }
            })
            .collect()
    }

    /// Retorna a lista de nomes das personas ativas
    pub fn persona_names(&self) -> Vec<&'static str> {
        self.personas.iter().map(|p| p.name()).collect()
    }

    /// Retorna o número de personas
    pub fn persona_count(&self) -> usize {
        self.personas.len()
    }

    /// Adiciona uma persona ao orquestrador
    pub fn add_persona(&mut self, persona: Box<dyn CognitivePersona>) {
        self.personas.push(persona);
    }

    /// Remove uma persona pelo nome
    pub fn remove_persona(&mut self, name: &str) {
        self.personas.retain(|p| p.name() != name);
    }

    /// Retorna descrições de todas as personas (para prompts)
    pub fn persona_descriptions(&self) -> Vec<String> {
        self.personas
            .iter()
            .map(|p| p.prompt_description())
            .collect()
    }
}

impl Default for PersonaOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_orchestrator() {
        let orchestrator = PersonaOrchestrator::new();
        assert_eq!(orchestrator.persona_count(), 7);
    }

    #[test]
    fn test_persona_names() {
        let orchestrator = PersonaOrchestrator::new();
        let names = orchestrator.persona_names();

        assert!(names.contains(&"Expert Skeptic"));
        assert!(names.contains(&"Detail Analyst"));
        assert!(names.contains(&"Historical Researcher"));
        assert!(names.contains(&"Comparative Thinker"));
        assert!(names.contains(&"Temporal Context"));
        assert!(names.contains(&"Globalizer"));
        assert!(names.contains(&"Reality Skepticalist"));
    }

    #[test]
    fn test_expand_query_parallel() {
        let orchestrator = PersonaOrchestrator::new();
        let context = QueryContext::default();

        let expanded = orchestrator.expand_query_parallel("rust programming", &context);

        // Deve ter 7 queries (uma por persona)
        assert_eq!(expanded.len(), 7);

        // Cada query deve ter um source_persona diferente
        let sources: Vec<_> = expanded.iter().map(|wq| wq.source_persona).collect();
        assert!(sources.contains(&"Expert Skeptic"));
        assert!(sources.contains(&"Detail Analyst"));
    }

    #[test]
    fn test_expand_batch() {
        let orchestrator = PersonaOrchestrator::new();
        let context = QueryContext::default();
        let queries = vec!["rust".into(), "programming".into()];

        let expanded = orchestrator.expand_batch(&queries, &context);

        // 2 queries × 7 personas = 14 queries expandidas
        assert_eq!(expanded.len(), 14);
    }

    #[test]
    fn test_technical_orchestrator() {
        let orchestrator = PersonaOrchestrator::technical();
        assert_eq!(orchestrator.persona_count(), 3);

        let names = orchestrator.persona_names();
        assert!(names.contains(&"Detail Analyst"));
        assert!(names.contains(&"Comparative Thinker"));
        assert!(names.contains(&"Temporal Context"));
    }

    #[test]
    fn test_investigative_orchestrator() {
        let orchestrator = PersonaOrchestrator::investigative();
        assert_eq!(orchestrator.persona_count(), 3);

        let names = orchestrator.persona_names();
        assert!(names.contains(&"Expert Skeptic"));
        assert!(names.contains(&"Reality Skepticalist"));
        assert!(names.contains(&"Historical Researcher"));
    }

    #[test]
    fn test_add_remove_persona() {
        let mut orchestrator = PersonaOrchestrator::new();
        let initial_count = orchestrator.persona_count();

        orchestrator.remove_persona("Expert Skeptic");
        assert_eq!(orchestrator.persona_count(), initial_count - 1);
        assert!(!orchestrator.persona_names().contains(&"Expert Skeptic"));
    }

    #[test]
    fn test_sequential_matches_parallel() {
        let orchestrator = PersonaOrchestrator::new();
        let context = QueryContext::default();

        let sequential = orchestrator.expand_query_sequential("test", &context);
        let parallel = orchestrator.expand_query_parallel("test", &context);

        // Mesmo número de resultados
        assert_eq!(sequential.len(), parallel.len());

        // Mesmas queries (ordem pode variar devido ao paralelismo)
        let seq_queries: std::collections::HashSet<_> =
            sequential.iter().map(|wq| &wq.query.q).collect();
        let par_queries: std::collections::HashSet<_> =
            parallel.iter().map(|wq| &wq.query.q).collect();
        assert_eq!(seq_queries, par_queries);
    }
}
