//! # Ferramentas de Processamento de Respostas
//!
//! Este módulo contém ferramentas para processar, polir e combinar
//! respostas geradas pelo agente de pesquisa.
//!
//! ## Componentes
//!
//! - [`ResponseFinalizer`]: Polir respostas como um "editor sênior"
//! - [`ResponseReducer`]: Mesclar respostas de múltiplos agentes
//! - [`ResearchPlanner`]: Dividir problemas em subproblemas ortogonais

pub mod finalizer;
pub mod reducer;
pub mod research_planner;

pub use finalizer::ResponseFinalizer;
pub use reducer::ResponseReducer;
pub use research_planner::ResearchPlanner;
