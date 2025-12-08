//! Módulo de otimizações de performance.
//!
//! Este módulo contém implementações otimizadas para operações
//! que são executadas milhares de vezes durante uma pesquisa.
//!
//! ## Por que otimizar?
//!
//! Durante uma pesquisa típica, o sistema:
//! - Compara centenas de queries para deduplicação
//! - Calcula similaridade entre milhares de embeddings
//! - Ranqueia dezenas de URLs por relevância
//!
//! Sem otimização, essas operações seriam o gargalo.
//!
//! ## Técnicas Utilizadas
//!
//! - **SIMD (AVX2)**: Processa 8 floats por instrução
//! - **Paralelismo**: Usa todos os cores via Rayon
//! - **Cache-friendly**: Acesso sequencial à memória

/// Operações vetoriais otimizadas com SIMD.
///
/// Implementações de alta performance para:
/// - [`cosine_similarity`]: Similaridade entre dois vetores
/// - [`find_similar`]: Encontrar vetores similares em batch
/// - [`dedup_queries`]: Remover queries duplicadas semanticamente
///
/// Usa instruções AVX2 quando disponíveis (x86_64),
/// com fallback para implementação escalar.
pub mod simd;

pub use simd::{cosine_similarity, dedup_queries, find_similar};
