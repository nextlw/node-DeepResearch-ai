// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEEP RESEARCH - IMPLEMENTAÇÃO RUST
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Este crate implementa o sistema DeepResearch em Rust, oferecendo:
//
// 1. Máquina de Estados para raciocínio de agente
//    - Estados: Processing, BeastMode, Completed, Failed
//    - Ações: Search, Read, Reflect, Answer, Coding
//    - Transições explícitas e type-safe
//
// 2. Sistema de Personas Cognitivas
//    - 7 personas com perspectivas diferentes
//    - Expansão paralela de queries com Rayon
//    - Trait extensível para personas customizadas
//
// 3. Avaliação Multidimensional
//    - 5 tipos: Definitive, Freshness, Plurality, Completeness, Strict
//    - Pipeline com falha rápida
//    - Configuração por tipo de avaliação
//
// 4. Performance Otimizada
//    - SIMD (AVX2) para similaridade cosseno
//    - Paralelismo real com Rayon
//    - Zero-copy onde possível
//
// Ganhos estimados vs TypeScript:
// - Throughput: 10-20x
// - Memória: 80-90% menos
// - Latência: previsível (sem GC)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod types;
pub mod agent;
pub mod personas;
pub mod evaluation;
pub mod performance;
pub mod llm;
pub mod search;
pub mod utils;

// Re-exports principais
pub use types::*;
pub use agent::DeepResearchAgent;
pub use personas::PersonaOrchestrator;
pub use evaluation::{EvaluationPipeline, EvaluationType};
pub use performance::simd::cosine_similarity;

/// Versão da biblioteca
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude com imports comuns
pub mod prelude {
    pub use crate::agent::{
        DeepResearchAgent, AgentState, AgentAction, AgentContext, ActionPermissions,
    };
    pub use crate::personas::{
        PersonaOrchestrator, CognitivePersona, QueryContext, WeightedQuery,
    };
    pub use crate::evaluation::{
        EvaluationPipeline, EvaluationType, EvaluationResult, EvaluationContext,
    };
    pub use crate::types::*;
    pub use crate::performance::simd::{cosine_similarity, find_similar, dedup_queries};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
