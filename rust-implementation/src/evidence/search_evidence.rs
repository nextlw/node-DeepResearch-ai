//! # Evidências de Busca
//!
//! Estruturas para coletar e reportar evidências de operações de busca,
//! incluindo detalhes de cada query, resultados e métricas.

use crate::types::SerpQuery;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use uuid::Uuid;

use super::{EvidenceReport, LatencyStats};

/// Evidência de uma URL extraída
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEvidence {
    /// URL completa
    pub url: String,
    /// Hostname extraído
    pub hostname: String,
    /// Boost aplicado pelo hostname (ex: Wikipedia = 1.3)
    pub hostname_boost: f32,
    /// Boost aplicado pelo path (ex: /docs = 1.2)
    pub path_boost: f32,
    /// Score final calculado
    pub final_score: f32,
}

impl UrlEvidence {
    /// Cria uma nova evidência de URL
    pub fn new(url: impl Into<String>, hostname: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            hostname: hostname.into(),
            hostname_boost: 1.0,
            path_boost: 1.0,
            final_score: 1.0,
        }
    }
    
    /// Define os boosts e calcula score final
    pub fn with_boosts(mut self, hostname_boost: f32, path_boost: f32) -> Self {
        self.hostname_boost = hostname_boost;
        self.path_boost = path_boost;
        self.final_score = hostname_boost * path_boost;
        self
    }
}

/// Evidência de uma query de busca individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryEvidence {
    /// ID único da query
    pub query_id: Uuid,
    /// Query enviada
    pub query: SerpQuery,
    /// Persona que originou a query (se aplicável)
    pub source_persona: Option<String>,
    /// Endpoint de API chamado
    pub api_endpoint: String,
    /// Timestamp da requisição
    pub request_time: DateTime<Utc>,
    /// Timestamp da resposta
    pub response_time: DateTime<Utc>,
    /// Código HTTP da resposta
    pub http_status: u16,
    /// Número de resultados retornados
    pub results_count: usize,
    /// Bytes recebidos na resposta
    pub bytes_received: usize,
    /// URLs extraídas com suas evidências
    pub urls_extracted: Vec<UrlEvidence>,
    /// Se veio do cache
    pub from_cache: bool,
}

impl SearchQueryEvidence {
    /// Cria uma nova evidência de query
    pub fn new(query: SerpQuery, api_endpoint: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            query_id: Uuid::new_v4(),
            query,
            source_persona: None,
            api_endpoint: api_endpoint.into(),
            request_time: now,
            response_time: now,
            http_status: 0,
            results_count: 0,
            bytes_received: 0,
            urls_extracted: vec![],
            from_cache: false,
        }
    }
    
    /// Define a persona de origem
    pub fn with_persona(mut self, persona: impl Into<String>) -> Self {
        self.source_persona = Some(persona.into());
        self
    }
    
    /// Registra a conclusão da query
    pub fn complete(&mut self, status: u16, results_count: usize, bytes: usize) {
        self.response_time = Utc::now();
        self.http_status = status;
        self.results_count = results_count;
        self.bytes_received = bytes;
    }
    
    /// Adiciona uma URL extraída
    pub fn add_url(&mut self, url: UrlEvidence) {
        self.urls_extracted.push(url);
    }
    
    /// Retorna a latência da query
    pub fn latency(&self) -> Duration {
        let diff = self.response_time - self.request_time;
        Duration::from_millis(diff.num_milliseconds().max(0) as u64)
    }
    
    /// Verifica se a query foi bem sucedida
    pub fn is_success(&self) -> bool {
        self.http_status >= 200 && self.http_status < 300
    }
}

/// Relatório completo de evidências de busca para uma execução
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEvidenceReport {
    /// ID da execução
    pub execution_id: Uuid,
    /// Timestamp de criação do relatório
    pub timestamp: DateTime<Utc>,
    /// Todas as queries enviadas
    pub queries_sent: Vec<SearchQueryEvidence>,
    /// Total de chamadas à API
    pub total_api_calls: usize,
    /// Total de bytes transferidos
    pub total_bytes_transferred: usize,
    /// Total de URLs descobertas
    pub total_urls_discovered: usize,
    /// Hostnames únicos encontrados
    pub unique_hostnames: HashSet<String>,
    /// Estatísticas de latência
    pub latency_stats: LatencyStats,
    /// Taxa de cache hit
    pub cache_hit_rate: f32,
    /// Taxa de sucesso das queries
    pub success_rate: f32,
}

impl Default for SearchEvidenceReport {
    fn default() -> Self {
        Self::new(Uuid::new_v4())
    }
}

impl SearchEvidenceReport {
    /// Cria um novo relatório de evidências de busca
    pub fn new(execution_id: Uuid) -> Self {
        Self {
            execution_id,
            timestamp: Utc::now(),
            queries_sent: vec![],
            total_api_calls: 0,
            total_bytes_transferred: 0,
            total_urls_discovered: 0,
            unique_hostnames: HashSet::new(),
            latency_stats: LatencyStats::default(),
            cache_hit_rate: 0.0,
            success_rate: 0.0,
        }
    }
    
    /// Adiciona uma evidência de query
    pub fn add_query(&mut self, query: SearchQueryEvidence) {
        // Atualiza contadores
        if !query.from_cache {
            self.total_api_calls += 1;
        }
        self.total_bytes_transferred += query.bytes_received;
        self.total_urls_discovered += query.urls_extracted.len();
        
        // Coleta hostnames únicos
        for url in &query.urls_extracted {
            self.unique_hostnames.insert(url.hostname.clone());
        }
        
        self.queries_sent.push(query);
    }
    
    /// Finaliza o relatório calculando estatísticas
    pub fn finalize(&mut self) {
        let total = self.queries_sent.len();
        if total == 0 {
            return;
        }
        
        // Calcular estatísticas de latência
        let latencies: Vec<Duration> = self.queries_sent.iter()
            .map(|q| q.latency())
            .collect();
        self.latency_stats = LatencyStats::from_durations(latencies);
        
        // Calcular taxa de cache hit
        let cache_hits = self.queries_sent.iter().filter(|q| q.from_cache).count();
        self.cache_hit_rate = cache_hits as f32 / total as f32;
        
        // Calcular taxa de sucesso
        let successes = self.queries_sent.iter().filter(|q| q.is_success()).count();
        self.success_rate = successes as f32 / total as f32;
    }
    
    /// Retorna queries agrupadas por persona
    pub fn queries_by_persona(&self) -> std::collections::HashMap<String, Vec<&SearchQueryEvidence>> {
        let mut map: std::collections::HashMap<String, Vec<&SearchQueryEvidence>> = std::collections::HashMap::new();
        
        for query in &self.queries_sent {
            let key = query.source_persona.clone().unwrap_or_else(|| "unknown".to_string());
            map.entry(key).or_default().push(query);
        }
        
        map
    }
    
    /// Retorna as top URLs por score
    pub fn top_urls(&self, limit: usize) -> Vec<&UrlEvidence> {
        let mut all_urls: Vec<&UrlEvidence> = self.queries_sent.iter()
            .flat_map(|q| q.urls_extracted.iter())
            .collect();
        
        all_urls.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));
        all_urls.into_iter().take(limit).collect()
    }
    
    /// Gera resumo textual
    pub fn summary_text(&self) -> String {
        format!(
            "SearchEvidenceReport [{}]\n\
             - Queries: {}\n\
             - API Calls: {}\n\
             - Cache Hit Rate: {:.1}%\n\
             - Success Rate: {:.1}%\n\
             - URLs Discovered: {}\n\
             - Unique Hostnames: {}\n\
             - Bytes Transferred: {}\n\
             - Latency: {}",
            self.execution_id,
            self.queries_sent.len(),
            self.total_api_calls,
            self.cache_hit_rate * 100.0,
            self.success_rate * 100.0,
            self.total_urls_discovered,
            self.unique_hostnames.len(),
            self.total_bytes_transferred,
            self.latency_stats.format_summary()
        )
    }
}

impl EvidenceReport for SearchEvidenceReport {
    fn execution_id(&self) -> Uuid {
        self.execution_id
    }
    
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    
    fn summary(&self) -> String {
        self.summary_text()
    }
    
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Builder para criar SearchEvidenceReport facilmente
#[derive(Debug, Default)]
pub struct SearchEvidenceBuilder {
    execution_id: Option<Uuid>,
    queries: Vec<SearchQueryEvidence>,
}

impl SearchEvidenceBuilder {
    /// Cria um novo builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Define o ID da execução
    pub fn execution_id(mut self, id: Uuid) -> Self {
        self.execution_id = Some(id);
        self
    }
    
    /// Adiciona uma query
    pub fn add_query(mut self, query: SearchQueryEvidence) -> Self {
        self.queries.push(query);
        self
    }
    
    /// Constrói o relatório
    pub fn build(self) -> SearchEvidenceReport {
        let mut report = SearchEvidenceReport::new(
            self.execution_id.unwrap_or_else(Uuid::new_v4)
        );
        
        for query in self.queries {
            report.add_query(query);
        }
        
        report.finalize();
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Testes para UrlEvidence
    // ========================================================================

    #[test]
    fn test_url_evidence_new() {
        let url = UrlEvidence::new("https://example.com/page", "example.com");
        assert_eq!(url.url, "https://example.com/page");
        assert_eq!(url.hostname, "example.com");
        assert_eq!(url.hostname_boost, 1.0);
        assert_eq!(url.path_boost, 1.0);
        assert_eq!(url.final_score, 1.0);
    }

    #[test]
    fn test_url_evidence_with_boosts() {
        let url = UrlEvidence::new("https://wikipedia.org/wiki/Rust", "wikipedia.org")
            .with_boosts(1.3, 1.2);
        
        assert_eq!(url.hostname_boost, 1.3);
        assert_eq!(url.path_boost, 1.2);
        assert!((url.final_score - 1.56).abs() < 0.01); // 1.3 * 1.2 = 1.56
    }

    // ========================================================================
    // Testes para SearchQueryEvidence
    // ========================================================================

    #[test]
    fn test_search_query_evidence_new() {
        let query = SerpQuery {
            q: "test query".to_string(),
            ..Default::default()
        };
        
        let evidence = SearchQueryEvidence::new(query, "https://api.serper.dev");
        
        assert_eq!(evidence.query.q, "test query");
        assert_eq!(evidence.api_endpoint, "https://api.serper.dev");
        assert!(evidence.source_persona.is_none());
    }

    #[test]
    fn test_search_query_evidence_with_persona() {
        let query = SerpQuery::default();
        let evidence = SearchQueryEvidence::new(query, "api")
            .with_persona("Skeptic");
        
        assert_eq!(evidence.source_persona, Some("Skeptic".to_string()));
    }

    #[test]
    fn test_search_query_evidence_complete() {
        let query = SerpQuery::default();
        let mut evidence = SearchQueryEvidence::new(query, "api");
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        evidence.complete(200, 10, 5000);
        
        assert_eq!(evidence.http_status, 200);
        assert_eq!(evidence.results_count, 10);
        assert_eq!(evidence.bytes_received, 5000);
        assert!(evidence.latency() >= Duration::from_millis(10));
    }

    #[test]
    fn test_search_query_evidence_is_success() {
        let query = SerpQuery::default();
        let mut evidence = SearchQueryEvidence::new(query, "api");
        
        evidence.http_status = 200;
        assert!(evidence.is_success());
        
        evidence.http_status = 404;
        assert!(!evidence.is_success());
        
        evidence.http_status = 500;
        assert!(!evidence.is_success());
    }

    #[test]
    fn test_search_query_evidence_add_url() {
        let query = SerpQuery::default();
        let mut evidence = SearchQueryEvidence::new(query, "api");
        
        evidence.add_url(UrlEvidence::new("https://example.com", "example.com"));
        evidence.add_url(UrlEvidence::new("https://test.org", "test.org"));
        
        assert_eq!(evidence.urls_extracted.len(), 2);
    }

    // ========================================================================
    // Testes para SearchEvidenceReport
    // ========================================================================

    #[test]
    fn test_search_evidence_report_new() {
        let id = Uuid::new_v4();
        let report = SearchEvidenceReport::new(id);
        
        assert_eq!(report.execution_id, id);
        assert!(report.queries_sent.is_empty());
        assert_eq!(report.total_api_calls, 0);
    }

    #[test]
    fn test_search_evidence_report_add_query() {
        let mut report = SearchEvidenceReport::default();
        
        let query = SerpQuery { q: "test".to_string(), ..Default::default() };
        let mut evidence = SearchQueryEvidence::new(query, "api");
        evidence.complete(200, 5, 2000);
        evidence.add_url(UrlEvidence::new("https://example.com", "example.com"));
        
        report.add_query(evidence);
        
        assert_eq!(report.queries_sent.len(), 1);
        assert_eq!(report.total_api_calls, 1);
        assert_eq!(report.total_bytes_transferred, 2000);
        assert_eq!(report.total_urls_discovered, 1);
        assert!(report.unique_hostnames.contains("example.com"));
    }

    #[test]
    fn test_search_evidence_report_cache_hit() {
        let mut report = SearchEvidenceReport::default();
        
        // Query normal
        let mut q1 = SearchQueryEvidence::new(SerpQuery::default(), "api");
        q1.complete(200, 1, 100);
        report.add_query(q1);
        
        // Query de cache
        let mut q2 = SearchQueryEvidence::new(SerpQuery::default(), "api");
        q2.from_cache = true;
        q2.complete(200, 1, 100);
        report.add_query(q2);
        
        // Apenas 1 API call (o segundo foi cache)
        assert_eq!(report.total_api_calls, 1);
    }

    #[test]
    fn test_search_evidence_report_finalize() {
        let mut report = SearchEvidenceReport::default();
        
        for i in 0..10 {
            let mut q = SearchQueryEvidence::new(SerpQuery::default(), "api");
            q.http_status = if i < 8 { 200 } else { 500 };
            q.from_cache = i % 3 == 0;
            report.add_query(q);
        }
        
        report.finalize();
        
        // 8/10 foram sucesso
        assert!((report.success_rate - 0.8).abs() < 0.01);
        
        // 4/10 foram cache (0, 3, 6, 9)
        assert!((report.cache_hit_rate - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_search_evidence_report_queries_by_persona() {
        let mut report = SearchEvidenceReport::default();
        
        let q1 = SearchQueryEvidence::new(SerpQuery::default(), "api")
            .with_persona("Skeptic");
        let q2 = SearchQueryEvidence::new(SerpQuery::default(), "api")
            .with_persona("Skeptic");
        let q3 = SearchQueryEvidence::new(SerpQuery::default(), "api")
            .with_persona("Historical");
        
        report.add_query(q1);
        report.add_query(q2);
        report.add_query(q3);
        
        let by_persona = report.queries_by_persona();
        
        assert_eq!(by_persona.get("Skeptic").unwrap().len(), 2);
        assert_eq!(by_persona.get("Historical").unwrap().len(), 1);
    }

    #[test]
    fn test_search_evidence_report_top_urls() {
        let mut report = SearchEvidenceReport::default();
        
        let mut q = SearchQueryEvidence::new(SerpQuery::default(), "api");
        q.add_url(UrlEvidence::new("url1", "h1").with_boosts(1.0, 1.0));
        q.add_url(UrlEvidence::new("url2", "h2").with_boosts(1.5, 1.0));
        q.add_url(UrlEvidence::new("url3", "h3").with_boosts(2.0, 1.0));
        
        report.add_query(q);
        
        let top = report.top_urls(2);
        
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].url, "url3"); // score 2.0
        assert_eq!(top[1].url, "url2"); // score 1.5
    }

    #[test]
    fn test_search_evidence_report_summary() {
        let mut report = SearchEvidenceReport::new(Uuid::nil());
        
        let mut q = SearchQueryEvidence::new(SerpQuery::default(), "api");
        q.complete(200, 5, 1000);
        report.add_query(q);
        report.finalize();
        
        let summary = report.summary_text();
        
        assert!(summary.contains("SearchEvidenceReport"));
        assert!(summary.contains("Queries: 1"));
        assert!(summary.contains("API Calls: 1"));
    }

    // ========================================================================
    // Testes para SearchEvidenceBuilder
    // ========================================================================

    #[test]
    fn test_search_evidence_builder() {
        let id = Uuid::new_v4();
        
        let mut q = SearchQueryEvidence::new(SerpQuery::default(), "api");
        q.complete(200, 1, 100);
        
        let report = SearchEvidenceBuilder::new()
            .execution_id(id)
            .add_query(q)
            .build();
        
        assert_eq!(report.execution_id, id);
        assert_eq!(report.queries_sent.len(), 1);
        // finalize() foi chamado
        assert!(report.success_rate > 0.0);
    }

    // ========================================================================
    // Testes para trait EvidenceReport
    // ========================================================================

    #[test]
    fn test_evidence_report_trait() {
        let id = Uuid::new_v4();
        let report = SearchEvidenceReport::new(id);
        
        // Testar trait methods
        assert_eq!(report.execution_id(), id);
        assert!(!report.summary().is_empty());
        assert!(!report.to_json().is_null());
    }
}

