//! # Sistema de Evidências
//!
//! Este módulo coleta e reporta evidências de funcionamento do sistema,
//! permitindo auditar o comportamento das personas, buscas e avaliações.
//!
//! ## Estrutura
//!
//! - `PersonaEvidenceReport` - Evidências de personas (já em personas/metrics.rs)
//! - `SearchEvidenceReport` - Evidências de operações de busca
//! - `EvaluationEvidenceReport` - Evidências de operações de avaliação

mod search_evidence;
mod evaluation_evidence;

pub use search_evidence::*;
pub use evaluation_evidence::*;

use chrono::{DateTime, Utc};
use std::time::Duration;
use uuid::Uuid;

/// Trait comum para todos os relatórios de evidências
pub trait EvidenceReport: Send + Sync {
    /// Retorna o ID único da execução
    fn execution_id(&self) -> Uuid;
    
    /// Retorna o timestamp de quando o relatório foi criado
    fn timestamp(&self) -> DateTime<Utc>;
    
    /// Retorna uma representação textual resumida
    fn summary(&self) -> String;
    
    /// Retorna o JSON do relatório para serialização
    fn to_json(&self) -> serde_json::Value;
}

/// Estatísticas de latência agregadas
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LatencyStats {
    /// Menor latência observada
    pub min: Duration,
    /// Maior latência observada  
    pub max: Duration,
    /// Média das latências
    pub avg: Duration,
    /// Percentil 50 (mediana)
    pub p50: Duration,
    /// Percentil 95
    pub p95: Duration,
    /// Percentil 99
    pub p99: Duration,
    /// Número total de amostras
    pub count: usize,
}

impl LatencyStats {
    /// Cria estatísticas a partir de uma lista de durações
    pub fn from_durations(mut durations: Vec<Duration>) -> Self {
        if durations.is_empty() {
            return Self::default();
        }
        
        durations.sort();
        let count = durations.len();
        let sum: Duration = durations.iter().sum();
        
        Self {
            min: durations[0],
            max: durations[count - 1],
            avg: sum / count as u32,
            p50: durations[count / 2],
            p95: durations[(count as f64 * 0.95) as usize],
            p99: durations[(count as f64 * 0.99) as usize],
            count,
        }
    }
    
    /// Formata a latência para exibição
    pub fn format_summary(&self) -> String {
        format!(
            "min={:?} p50={:?} p95={:?} p99={:?} max={:?} (n={})",
            self.min, self.p50, self.p95, self.p99, self.max, self.count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_latency_stats_empty() {
        let stats = LatencyStats::from_durations(vec![]);
        assert_eq!(stats.count, 0);
    }
    
    #[test]
    fn test_latency_stats_single() {
        let stats = LatencyStats::from_durations(vec![Duration::from_millis(100)]);
        assert_eq!(stats.count, 1);
        assert_eq!(stats.min, Duration::from_millis(100));
        assert_eq!(stats.max, Duration::from_millis(100));
    }
    
    #[test]
    fn test_latency_stats_multiple() {
        let stats = LatencyStats::from_durations(vec![
            Duration::from_millis(50),
            Duration::from_millis(100),
            Duration::from_millis(150),
            Duration::from_millis(200),
            Duration::from_millis(250),
        ]);
        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, Duration::from_millis(50));
        assert_eq!(stats.max, Duration::from_millis(250));
        assert_eq!(stats.p50, Duration::from_millis(150));
    }
    
    #[test]
    fn test_latency_stats_format() {
        let stats = LatencyStats::from_durations(vec![Duration::from_millis(100)]);
        let summary = stats.format_summary();
        assert!(summary.contains("p50="));
        assert!(summary.contains("n=1"));
    }
}

