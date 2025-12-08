// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MÉTRICAS DE EXECUÇÃO DE PERSONAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Sistema de coleta de métricas para rastrear execução de personas.
// Permite debugging, benchmarking e comparação com TypeScript.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::time::{Duration, Instant};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::types::SerpQuery;

/// Métricas de execução de uma única persona
#[derive(Debug, Clone)]
pub struct PersonaExecutionMetrics {
    /// Nome da persona que executou
    pub persona_name: &'static str,
    /// Foco/especialidade da persona
    pub persona_focus: &'static str,
    /// Peso da persona
    pub weight: f32,
    /// Timestamp de início
    pub start_time: Instant,
    /// Timestamp de fim
    pub end_time: Instant,
    /// Query de entrada (original)
    pub input_query: String,
    /// Query de saída (expandida)
    pub output_query: SerpQuery,
    /// Se a persona foi aplicável ao contexto
    pub was_applicable: bool,
    /// Estimativa de tokens de entrada (para custo de API)
    pub input_tokens: usize,
    /// Memória alocada em bytes (para monitoramento)
    pub memory_allocated: usize,
}

impl PersonaExecutionMetrics {
    /// Cria novas métricas com timestamps zerados (preencher depois)
    pub fn new(
        persona_name: &'static str,
        persona_focus: &'static str,
        weight: f32,
        input_query: String,
    ) -> Self {
        let now = Instant::now();
        // Estima tokens como ~4 chars por token (aproximação comum)
        let input_tokens = input_query.len() / 4;
        Self {
            persona_name,
            persona_focus,
            weight,
            start_time: now,
            end_time: now,
            input_query,
            output_query: SerpQuery::default(),
            was_applicable: true,
            input_tokens,
            memory_allocated: 0,
        }
    }

    /// Marca o início da execução
    pub fn start(&mut self) {
        self.start_time = Instant::now();
    }

    /// Marca o fim da execução e retorna duração
    pub fn finish(&mut self, output_query: SerpQuery, was_applicable: bool) -> Duration {
        self.end_time = Instant::now();
        // Estima memória alocada pela query de saída
        self.memory_allocated = std::mem::size_of::<SerpQuery>() 
            + output_query.q.capacity()
            + output_query.tbs.as_ref().map(|s| s.capacity()).unwrap_or(0)
            + output_query.location.as_ref().map(|s| s.capacity()).unwrap_or(0);
        self.output_query = output_query;
        self.was_applicable = was_applicable;
        self.duration()
    }

    /// Duração total da execução
    pub fn duration(&self) -> Duration {
        self.end_time.duration_since(self.start_time)
    }

    /// Duração em milissegundos
    pub fn duration_ms(&self) -> f64 {
        self.duration().as_secs_f64() * 1000.0
    }

    /// Retorna estimativa de tokens de entrada
    pub fn tokens(&self) -> usize {
        self.input_tokens
    }

    /// Retorna memória alocada em bytes
    pub fn memory(&self) -> usize {
        self.memory_allocated
    }
}

/// Evidência de execução de uma persona (para relatórios)
#[derive(Debug, Clone)]
pub struct PersonaEvidence {
    /// Nome da persona
    pub persona_name: &'static str,
    /// Foco da persona
    pub focus: &'static str,
    /// Peso da persona
    pub weight: f32,
    /// Query de entrada
    pub input_received: String,
    /// Query de saída
    pub output_generated: SerpQuery,
    /// Tempo de execução
    pub execution_time: Duration,
    /// Se foi aplicável
    pub was_applicable: bool,
    /// Tokens de entrada estimados
    pub input_tokens: usize,
    /// Memória alocada em bytes
    pub memory_allocated: usize,
}

impl From<PersonaExecutionMetrics> for PersonaEvidence {
    fn from(metrics: PersonaExecutionMetrics) -> Self {
        let execution_time = metrics.duration();
        Self {
            persona_name: metrics.persona_name,
            focus: metrics.persona_focus,
            weight: metrics.weight,
            input_received: metrics.input_query,
            output_generated: metrics.output_query,
            execution_time,
            was_applicable: metrics.was_applicable,
            input_tokens: metrics.input_tokens,
            memory_allocated: metrics.memory_allocated,
        }
    }
}

/// Relatório completo de execução de personas
#[derive(Debug, Clone)]
pub struct PersonaEvidenceReport {
    /// ID único desta execução
    pub execution_id: Uuid,
    /// Timestamp da execução
    pub timestamp: DateTime<Utc>,
    /// Query original do usuário
    pub original_query: String,
    /// Evidências de cada persona
    pub personas_executed: Vec<PersonaEvidence>,
    /// Total de queries geradas
    pub total_queries_generated: usize,
    /// Queries únicas (sem duplicatas)
    pub unique_queries: usize,
    /// Tempo total de execução
    pub total_execution_time: Duration,
}

impl PersonaEvidenceReport {
    /// Cria um novo relatório vazio
    pub fn new(execution_id: Uuid, original_query: String) -> Self {
        Self {
            execution_id,
            timestamp: Utc::now(),
            original_query,
            personas_executed: Vec::new(),
            total_queries_generated: 0,
            unique_queries: 0,
            total_execution_time: Duration::ZERO,
        }
    }

    /// Adiciona evidência de uma persona
    pub fn add_evidence(&mut self, evidence: PersonaEvidence) {
        self.total_execution_time += evidence.execution_time;
        self.total_queries_generated += 1;
        self.personas_executed.push(evidence);
    }

    /// Finaliza o relatório calculando queries únicas
    pub fn finalize(&mut self) {
        use std::collections::HashSet;
        let unique: HashSet<_> = self
            .personas_executed
            .iter()
            .map(|e| &e.output_generated.q)
            .collect();
        self.unique_queries = unique.len();
    }

    /// Retorna resumo formatado
    pub fn summary(&self) -> String {
        format!(
            "Execution {} | {} personas | {} queries ({} unique) | {:.2}ms total",
            &self.execution_id.to_string()[..8],
            self.personas_executed.len(),
            self.total_queries_generated,
            self.unique_queries,
            self.total_execution_time.as_secs_f64() * 1000.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_execution_metrics() {
        let mut metrics = PersonaExecutionMetrics::new(
            "Test Persona",
            "testing",
            1.0,
            "test query".into(),
        );

        metrics.start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let output = SerpQuery {
            q: "test query expanded".into(),
            tbs: None,
            location: None,
        };
        
        let duration = metrics.finish(output.clone(), true);
        
        assert!(duration.as_millis() >= 10);
        assert_eq!(metrics.output_query.q, "test query expanded");
        assert!(metrics.was_applicable);
    }

    #[test]
    fn test_persona_evidence_from_metrics() {
        let mut metrics = PersonaExecutionMetrics::new(
            "Expert Skeptic",
            "problems and limitations",
            1.5,
            "rust programming".into(),
        );

        metrics.start();
        let output = SerpQuery {
            q: "rust programming problems issues".into(),
            tbs: None,
            location: None,
        };
        metrics.finish(output, true);

        let evidence: PersonaEvidence = metrics.into();
        
        assert_eq!(evidence.persona_name, "Expert Skeptic");
        assert_eq!(evidence.weight, 1.5);
        assert!(evidence.was_applicable);
    }

    #[test]
    fn test_persona_evidence_report() {
        let execution_id = Uuid::new_v4();
        let mut report = PersonaEvidenceReport::new(execution_id, "test query".into());

        // Adiciona 2 evidências
        report.add_evidence(PersonaEvidence {
            persona_name: "Persona 1",
            focus: "focus 1",
            weight: 1.0,
            input_received: "test".into(),
            output_generated: SerpQuery {
                q: "test expanded 1".into(),
                tbs: None,
                location: None,
            },
            execution_time: Duration::from_millis(10),
            was_applicable: true,
            input_tokens: 1,
            memory_allocated: 64,
        });

        report.add_evidence(PersonaEvidence {
            persona_name: "Persona 2",
            focus: "focus 2",
            weight: 1.2,
            input_received: "test".into(),
            output_generated: SerpQuery {
                q: "test expanded 2".into(),
                tbs: None,
                location: None,
            },
            execution_time: Duration::from_millis(15),
            was_applicable: true,
            input_tokens: 1,
            memory_allocated: 64,
        });

        report.finalize();

        assert_eq!(report.personas_executed.len(), 2);
        assert_eq!(report.total_queries_generated, 2);
        assert_eq!(report.unique_queries, 2);
        assert_eq!(report.total_execution_time.as_millis(), 25);
    }

    #[test]
    fn test_metrics_tokens_and_memory() {
        let mut metrics = PersonaExecutionMetrics::new(
            "Test",
            "testing",
            1.0,
            "this is a test query with some words".into(), // ~40 chars = ~10 tokens
        );

        metrics.start();
        let output = SerpQuery {
            q: "expanded query".into(),
            tbs: Some("qdr:m".into()),
            location: None,
        };
        metrics.finish(output, true);

        // Verifica que tokens foram estimados
        assert!(metrics.tokens() > 0);
        // Verifica que memória foi calculada
        assert!(metrics.memory() > 0);
    }
}

