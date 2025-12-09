// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// RASTREAMENTO DE BUSCA (SEARCH TRACE)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Sistema de rastreamento para fluxo de dados de busca.
// Permite saber onde cada dado foi acionado e para onde foi.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use crate::types::SerpQuery;

/// Origem de uma query de busca
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryOrigin {
    /// Query original do usuário
    User,
    /// Query expandida por uma persona
    Persona {
        /// Nome da persona que gerou a query
        name: String,
    },
    /// Query gerada por reflexão do agente
    Reflection {
        /// Número da iteração de reflexão
        iteration: u32,
    },
    /// Query de refinamento baseada em resultados anteriores
    Refinement {
        /// ID do trace da query original que foi refinada
        parent_trace_id: Uuid,
    },
    /// Query de follow-up para aprofundar um tópico
    FollowUp {
        /// Tópico sendo aprofundado
        topic: String,
    },
}

impl Default for QueryOrigin {
    fn default() -> Self {
        Self::User
    }
}

impl std::fmt::Display for QueryOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "User"),
            Self::Persona { name } => write!(f, "Persona({})", name),
            Self::Reflection { iteration } => write!(f, "Reflection(#{})", iteration),
            Self::Refinement { parent_trace_id } => {
                write!(f, "Refinement({})", &parent_trace_id.to_string()[..8])
            }
            Self::FollowUp { topic } => write!(f, "FollowUp({})", topic),
        }
    }
}

/// Status de uma busca
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchStatus {
    /// Busca em andamento
    InProgress,
    /// Busca completada com sucesso
    Success,
    /// Busca falhou
    Failed {
        /// Mensagem de erro
        error: String,
    },
    /// Busca foi cancelada (timeout, etc.)
    Cancelled {
        /// Motivo do cancelamento
        reason: String,
    },
    /// Busca foi pulada (cache hit, etc.)
    Skipped {
        /// Motivo do skip
        reason: String,
    },
}

impl Default for SearchStatus {
    fn default() -> Self {
        Self::InProgress
    }
}

/// Trace de uma única operação de busca
///
/// Captura todos os detalhes de uma busca individual:
/// - Query enviada e sua origem
/// - API chamada e timestamps
/// - Resultados recebidos e URLs extraídas
///
/// # Exemplo
///
/// ```rust,ignore
/// let trace = SearchTrace::new(query, QueryOrigin::User);
/// // ... executa busca ...
/// trace.complete(results_count, bytes_received, urls);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTrace {
    /// ID único deste trace
    pub trace_id: Uuid,
    /// ID da execução pai (para agrupar traces)
    pub execution_id: Uuid,
    /// Origem da query
    pub query_origin: QueryOrigin,
    /// Query enviada para a API
    pub query_sent: SerpQuery,
    /// Nome da API chamada (jina, serper, brave, etc.)
    pub api_called: String,
    /// Timestamp do request
    pub request_timestamp: DateTime<Utc>,
    /// Timestamp da response (None se ainda em andamento)
    pub response_timestamp: Option<DateTime<Utc>>,
    /// Status da busca
    pub status: SearchStatus,
    /// Número de resultados recebidos
    pub results_count: usize,
    /// Bytes recebidos na resposta
    pub bytes_received: usize,
    /// URLs extraídas dos resultados
    pub urls_extracted: Vec<String>,
    /// Metadados adicionais
    pub metadata: HashMap<String, String>,
}

impl SearchTrace {
    /// Cria um novo trace de busca
    pub fn new(execution_id: Uuid, query: SerpQuery, origin: QueryOrigin, api: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            execution_id,
            query_origin: origin,
            query_sent: query,
            api_called: api.into(),
            request_timestamp: Utc::now(),
            response_timestamp: None,
            status: SearchStatus::InProgress,
            results_count: 0,
            bytes_received: 0,
            urls_extracted: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Marca o trace como concluído com sucesso
    pub fn complete(&mut self, results_count: usize, bytes_received: usize, urls: Vec<String>) {
        self.response_timestamp = Some(Utc::now());
        self.status = SearchStatus::Success;
        self.results_count = results_count;
        self.bytes_received = bytes_received;
        self.urls_extracted = urls;
    }

    /// Marca o trace como falha
    pub fn fail(&mut self, error: impl Into<String>) {
        self.response_timestamp = Some(Utc::now());
        self.status = SearchStatus::Failed {
            error: error.into(),
        };
    }

    /// Marca o trace como cancelado
    pub fn cancel(&mut self, reason: impl Into<String>) {
        self.response_timestamp = Some(Utc::now());
        self.status = SearchStatus::Cancelled {
            reason: reason.into(),
        };
    }

    /// Marca o trace como pulado
    pub fn skip(&mut self, reason: impl Into<String>) {
        self.response_timestamp = Some(Utc::now());
        self.status = SearchStatus::Skipped {
            reason: reason.into(),
        };
    }

    /// Adiciona metadados ao trace
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Calcula a latência da busca
    pub fn latency(&self) -> Option<Duration> {
        self.response_timestamp.map(|end| {
            let start = self.request_timestamp;
            let diff = end - start;
            Duration::from_millis(diff.num_milliseconds().max(0) as u64)
        })
    }

    /// Verifica se a busca foi bem sucedida
    pub fn is_success(&self) -> bool {
        matches!(self.status, SearchStatus::Success)
    }

    /// Verifica se a busca está em andamento
    pub fn is_in_progress(&self) -> bool {
        matches!(self.status, SearchStatus::InProgress)
    }

    /// Retorna resumo formatado do trace
    pub fn summary(&self) -> String {
        let latency_str = self
            .latency()
            .map(|d| format!("{:.0}ms", d.as_millis()))
            .unwrap_or_else(|| "in progress".into());

        format!(
            "[{}] {} -> {} | {} | {} results | {} bytes | {}",
            &self.trace_id.to_string()[..8],
            self.query_origin,
            self.api_called,
            self.query_sent.q,
            self.results_count,
            self.bytes_received,
            latency_str
        )
    }
}

/// Coletor de traces de busca
///
/// Agrega todos os traces de uma execução de pesquisa.
/// Permite análise posterior e métricas agregadas.
///
/// # Exemplo
///
/// ```rust,ignore
/// let mut collector = SearchTraceCollector::new(execution_id);
/// collector.add(trace1);
/// collector.add(trace2);
/// 
/// println!("{}", collector.summary());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTraceCollector {
    /// ID da execução
    pub execution_id: Uuid,
    /// Timestamp de início da coleta
    pub started_at: DateTime<Utc>,
    /// Timestamp de fim da coleta (None se ainda em andamento)
    pub finished_at: Option<DateTime<Utc>>,
    /// Traces coletados
    pub traces: Vec<SearchTrace>,
    /// Query original do usuário
    pub original_query: String,
}

impl SearchTraceCollector {
    /// Cria um novo coletor
    pub fn new(execution_id: Uuid, original_query: impl Into<String>) -> Self {
        Self {
            execution_id,
            started_at: Utc::now(),
            finished_at: None,
            traces: Vec::new(),
            original_query: original_query.into(),
        }
    }

    /// Adiciona um trace ao coletor
    pub fn add(&mut self, trace: SearchTrace) {
        self.traces.push(trace);
    }

    /// Cria e adiciona um novo trace
    pub fn start_trace(
        &mut self,
        query: SerpQuery,
        origin: QueryOrigin,
        api: impl Into<String>,
    ) -> usize {
        let trace = SearchTrace::new(self.execution_id, query, origin, api);
        self.traces.push(trace);
        self.traces.len() - 1
    }

    /// Completa um trace pelo índice
    pub fn complete_trace(
        &mut self,
        index: usize,
        results_count: usize,
        bytes_received: usize,
        urls: Vec<String>,
    ) {
        if let Some(trace) = self.traces.get_mut(index) {
            trace.complete(results_count, bytes_received, urls);
        }
    }

    /// Falha um trace pelo índice
    pub fn fail_trace(&mut self, index: usize, error: impl Into<String>) {
        if let Some(trace) = self.traces.get_mut(index) {
            trace.fail(error);
        }
    }

    /// Finaliza a coleta
    pub fn finish(&mut self) {
        self.finished_at = Some(Utc::now());
    }

    /// Retorna o número total de traces
    pub fn len(&self) -> usize {
        self.traces.len()
    }

    /// Verifica se está vazio
    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    /// Retorna traces bem sucedidos
    pub fn successful_traces(&self) -> Vec<&SearchTrace> {
        self.traces.iter().filter(|t| t.is_success()).collect()
    }

    /// Retorna traces que falharam
    pub fn failed_traces(&self) -> Vec<&SearchTrace> {
        self.traces
            .iter()
            .filter(|t| matches!(t.status, SearchStatus::Failed { .. }))
            .collect()
    }

    /// Retorna traces por origem
    pub fn traces_by_origin(&self, origin: &QueryOrigin) -> Vec<&SearchTrace> {
        self.traces
            .iter()
            .filter(|t| &t.query_origin == origin)
            .collect()
    }

    /// Retorna traces de personas
    pub fn persona_traces(&self) -> Vec<&SearchTrace> {
        self.traces
            .iter()
            .filter(|t| matches!(t.query_origin, QueryOrigin::Persona { .. }))
            .collect()
    }

    /// Calcula latência total
    pub fn total_latency(&self) -> Duration {
        self.traces
            .iter()
            .filter_map(|t| t.latency())
            .sum()
    }

    /// Calcula latência média
    pub fn avg_latency(&self) -> Duration {
        let total = self.total_latency();
        let count = self.successful_traces().len();
        if count > 0 {
            total / count as u32
        } else {
            Duration::ZERO
        }
    }

    /// Calcula total de bytes recebidos
    pub fn total_bytes(&self) -> usize {
        self.traces.iter().map(|t| t.bytes_received).sum()
    }

    /// Calcula total de URLs extraídas
    pub fn total_urls(&self) -> usize {
        self.traces.iter().map(|t| t.urls_extracted.len()).sum()
    }

    /// Calcula URLs únicas extraídas
    pub fn unique_urls(&self) -> Vec<String> {
        use std::collections::HashSet;
        let unique: HashSet<_> = self
            .traces
            .iter()
            .flat_map(|t| &t.urls_extracted)
            .cloned()
            .collect();
        unique.into_iter().collect()
    }

    /// Calcula taxa de sucesso
    pub fn success_rate(&self) -> f32 {
        if self.traces.is_empty() {
            return 0.0;
        }
        let successful = self.successful_traces().len() as f32;
        successful / self.traces.len() as f32
    }

    /// Retorna resumo formatado do coletor
    pub fn summary(&self) -> String {
        let duration = self.finished_at
            .map(|end| (end - self.started_at).num_milliseconds())
            .unwrap_or(0);

        format!(
            "SearchTraceCollector [{}]\n\
             Query: '{}'\n\
             Total traces: {} ({} success, {} failed)\n\
             Total latency: {:.0}ms | Avg: {:.0}ms\n\
             Total bytes: {} | Total URLs: {} ({} unique)\n\
             Success rate: {:.1}%\n\
             Duration: {}ms",
            &self.execution_id.to_string()[..8],
            self.original_query,
            self.traces.len(),
            self.successful_traces().len(),
            self.failed_traces().len(),
            self.total_latency().as_millis(),
            self.avg_latency().as_millis(),
            self.total_bytes(),
            self.total_urls(),
            self.unique_urls().len(),
            self.success_rate() * 100.0,
            duration
        )
    }

    /// Retorna relatório detalhado por origem
    pub fn report_by_origin(&self) -> HashMap<String, OriginReport> {
        let mut reports: HashMap<String, OriginReport> = HashMap::new();

        for trace in &self.traces {
            let origin_key = match &trace.query_origin {
                QueryOrigin::User => "User".to_string(),
                QueryOrigin::Persona { name } => format!("Persona:{}", name),
                QueryOrigin::Reflection { .. } => "Reflection".to_string(),
                QueryOrigin::Refinement { .. } => "Refinement".to_string(),
                QueryOrigin::FollowUp { .. } => "FollowUp".to_string(),
            };

            let report = reports.entry(origin_key).or_insert_with(OriginReport::new);
            report.add_trace(trace);
        }

        reports
    }
}

/// Relatório de métricas por origem
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OriginReport {
    /// Número de traces
    pub count: usize,
    /// Traces bem sucedidos
    pub successful: usize,
    /// Traces que falharam
    pub failed: usize,
    /// Latência total
    pub total_latency_ms: u64,
    /// Bytes recebidos
    pub total_bytes: usize,
    /// URLs extraídas
    pub total_urls: usize,
}

impl OriginReport {
    /// Cria um novo relatório
    pub fn new() -> Self {
        Self::default()
    }

    /// Adiciona um trace ao relatório
    pub fn add_trace(&mut self, trace: &SearchTrace) {
        self.count += 1;
        
        if trace.is_success() {
            self.successful += 1;
        } else if matches!(trace.status, SearchStatus::Failed { .. }) {
            self.failed += 1;
        }

        if let Some(latency) = trace.latency() {
            self.total_latency_ms += latency.as_millis() as u64;
        }

        self.total_bytes += trace.bytes_received;
        self.total_urls += trace.urls_extracted.len();
    }

    /// Calcula latência média
    pub fn avg_latency_ms(&self) -> f64 {
        if self.successful > 0 {
            self.total_latency_ms as f64 / self.successful as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_query() -> SerpQuery {
        SerpQuery {
            q: "test query".into(),
            tbs: None,
            location: None,
        }
    }

    #[test]
    fn test_search_trace_new() {
        let execution_id = Uuid::new_v4();
        let trace = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");

        assert_eq!(trace.execution_id, execution_id);
        assert!(matches!(trace.query_origin, QueryOrigin::User));
        assert_eq!(trace.api_called, "jina");
        assert!(trace.is_in_progress());
    }

    #[test]
    fn test_search_trace_complete() {
        let execution_id = Uuid::new_v4();
        let mut trace = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");

        trace.complete(10, 5000, vec!["https://example.com".into()]);

        assert!(trace.is_success());
        assert_eq!(trace.results_count, 10);
        assert_eq!(trace.bytes_received, 5000);
        assert_eq!(trace.urls_extracted.len(), 1);
        assert!(trace.latency().is_some());
    }

    #[test]
    fn test_search_trace_fail() {
        let execution_id = Uuid::new_v4();
        let mut trace = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");

        trace.fail("API error");

        assert!(!trace.is_success());
        assert!(matches!(trace.status, SearchStatus::Failed { .. }));
    }

    #[test]
    fn test_search_trace_metadata() {
        let execution_id = Uuid::new_v4();
        let mut trace = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");

        trace.add_metadata("cache_hit", "false");
        trace.add_metadata("retry_count", "0");

        assert_eq!(trace.metadata.get("cache_hit"), Some(&"false".to_string()));
        assert_eq!(trace.metadata.get("retry_count"), Some(&"0".to_string()));
    }

    #[test]
    fn test_query_origin_display() {
        assert_eq!(format!("{}", QueryOrigin::User), "User");
        assert_eq!(
            format!(
                "{}",
                QueryOrigin::Persona {
                    name: "Skeptic".into()
                }
            ),
            "Persona(Skeptic)"
        );
        assert_eq!(
            format!("{}", QueryOrigin::Reflection { iteration: 3 }),
            "Reflection(#3)"
        );
    }

    #[test]
    fn test_collector_new() {
        let execution_id = Uuid::new_v4();
        let collector = SearchTraceCollector::new(execution_id, "test query");

        assert_eq!(collector.execution_id, execution_id);
        assert_eq!(collector.original_query, "test query");
        assert!(collector.is_empty());
    }

    #[test]
    fn test_collector_add_traces() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        let mut trace1 = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");
        trace1.complete(10, 5000, vec!["https://a.com".into()]);

        let mut trace2 = SearchTrace::new(
            execution_id,
            sample_query(),
            QueryOrigin::Persona {
                name: "Skeptic".into(),
            },
            "jina",
        );
        trace2.complete(5, 2500, vec!["https://b.com".into()]);

        collector.add(trace1);
        collector.add(trace2);

        assert_eq!(collector.len(), 2);
        assert_eq!(collector.successful_traces().len(), 2);
        assert_eq!(collector.total_bytes(), 7500);
    }

    #[test]
    fn test_collector_start_and_complete_trace() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        let idx = collector.start_trace(sample_query(), QueryOrigin::User, "jina");
        assert_eq!(idx, 0);
        assert!(collector.traces[0].is_in_progress());

        collector.complete_trace(idx, 10, 5000, vec!["https://example.com".into()]);
        assert!(collector.traces[0].is_success());
    }

    #[test]
    fn test_collector_fail_trace() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        let idx = collector.start_trace(sample_query(), QueryOrigin::User, "jina");
        collector.fail_trace(idx, "API timeout");

        assert_eq!(collector.failed_traces().len(), 1);
    }

    #[test]
    fn test_collector_unique_urls() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        let mut trace1 = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");
        trace1.complete(2, 1000, vec!["https://a.com".into(), "https://b.com".into()]);

        let mut trace2 = SearchTrace::new(
            execution_id,
            sample_query(),
            QueryOrigin::Persona {
                name: "Test".into(),
            },
            "jina",
        );
        trace2.complete(2, 1000, vec!["https://b.com".into(), "https://c.com".into()]);

        collector.add(trace1);
        collector.add(trace2);

        assert_eq!(collector.total_urls(), 4);
        assert_eq!(collector.unique_urls().len(), 3); // a, b, c
    }

    #[test]
    fn test_collector_success_rate() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        let mut success = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");
        success.complete(10, 5000, vec![]);

        let mut failed = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");
        failed.fail("Error");

        collector.add(success);
        collector.add(failed);

        assert_eq!(collector.success_rate(), 0.5);
    }

    #[test]
    fn test_collector_report_by_origin() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        let mut user_trace = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");
        user_trace.complete(10, 5000, vec![]);

        let mut persona_trace = SearchTrace::new(
            execution_id,
            sample_query(),
            QueryOrigin::Persona {
                name: "Skeptic".into(),
            },
            "jina",
        );
        persona_trace.complete(5, 2500, vec![]);

        collector.add(user_trace);
        collector.add(persona_trace);

        let report = collector.report_by_origin();

        assert_eq!(report.get("User").unwrap().count, 1);
        assert_eq!(report.get("Persona:Skeptic").unwrap().count, 1);
    }

    #[test]
    fn test_collector_persona_traces() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        collector.add(SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina"));
        collector.add(SearchTrace::new(
            execution_id,
            sample_query(),
            QueryOrigin::Persona {
                name: "Skeptic".into(),
            },
            "jina",
        ));
        collector.add(SearchTrace::new(
            execution_id,
            sample_query(),
            QueryOrigin::Persona {
                name: "Analyst".into(),
            },
            "jina",
        ));

        assert_eq!(collector.persona_traces().len(), 2);
    }

    #[test]
    fn test_trace_summary() {
        let execution_id = Uuid::new_v4();
        let mut trace = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");
        trace.complete(10, 5000, vec![]);

        let summary = trace.summary();
        assert!(summary.contains("User"));
        assert!(summary.contains("jina"));
        assert!(summary.contains("test query"));
    }

    #[test]
    fn test_collector_summary() {
        let execution_id = Uuid::new_v4();
        let mut collector = SearchTraceCollector::new(execution_id, "test query");

        let mut trace = SearchTrace::new(execution_id, sample_query(), QueryOrigin::User, "jina");
        trace.complete(10, 5000, vec!["https://example.com".into()]);
        collector.add(trace);
        collector.finish();

        let summary = collector.summary();
        assert!(summary.contains("test query"));
        assert!(summary.contains("1 success"));
    }
}

