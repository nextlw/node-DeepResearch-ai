// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MÉTRICAS DE BUSCA (SEARCH METRICS)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Sistema de métricas para monitoramento de performance de buscas.
// Permite comparação direta com implementação TypeScript.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Constante para tamanho do buffer de latências (para percentis)
const LATENCY_BUFFER_SIZE: usize = 1000;

/// Métricas agregadas de busca
///
/// Coleta estatísticas de performance para comparação com TypeScript:
/// - Latências (p50, p95, p99)
/// - Taxa de sucesso
/// - Média de resultados por query
/// - Taxa de cache hit
/// - Bytes por segundo
///
/// # Exemplo
///
/// ```rust,ignore
/// let metrics = SearchMetrics::new();
///
/// // Registrar uma busca
/// metrics.record_search(150, true, 10, 5000);
///
/// // Ver percentis
/// println!("p50: {}ms", metrics.latency_p50());
/// println!("p95: {}ms", metrics.latency_p95());
/// ```
#[derive(Debug)]
pub struct SearchMetrics {
    /// ID único desta instância de métricas
    pub id: Uuid,
    /// Timestamp de criação
    pub created_at: DateTime<Utc>,
    /// Buffer circular de latências (para cálculo de percentis)
    latencies: RwLock<VecDeque<u64>>,
    /// Total de buscas realizadas
    total_searches: AtomicU64,
    /// Buscas bem sucedidas
    successful_searches: AtomicU64,
    /// Buscas que falharam
    failed_searches: AtomicU64,
    /// Total de resultados recebidos
    total_results: AtomicU64,
    /// Total de bytes recebidos
    total_bytes: AtomicU64,
    /// Cache hits
    cache_hits: AtomicU64,
    /// Cache misses
    cache_misses: AtomicU64,
    /// Soma total de latências (para média)
    total_latency_ms: AtomicU64,
    /// Tempo total de execução em ms
    total_execution_time_ms: AtomicU64,
}

impl SearchMetrics {
    /// Cria nova instância de métricas
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            latencies: RwLock::new(VecDeque::with_capacity(LATENCY_BUFFER_SIZE)),
            total_searches: AtomicU64::new(0),
            successful_searches: AtomicU64::new(0),
            failed_searches: AtomicU64::new(0),
            total_results: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            total_execution_time_ms: AtomicU64::new(0),
        }
    }

    /// Registra uma busca completada
    pub fn record_search(&self, latency_ms: u64, success: bool, results_count: usize, bytes: usize) {
        self.total_searches.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.total_results.fetch_add(results_count as u64, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);

        if success {
            self.successful_searches.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_searches.fetch_add(1, Ordering::Relaxed);
        }

        // Adicionar latência ao buffer circular
        if let Ok(mut latencies) = self.latencies.write() {
            if latencies.len() >= LATENCY_BUFFER_SIZE {
                latencies.pop_front();
            }
            latencies.push_back(latency_ms);
        }
    }

    /// Registra um cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Registra um cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Adiciona tempo de execução total
    pub fn add_execution_time(&self, ms: u64) {
        self.total_execution_time_ms.fetch_add(ms, Ordering::Relaxed);
    }

    /// Retorna total de buscas
    pub fn total_searches(&self) -> u64 {
        self.total_searches.load(Ordering::Relaxed)
    }

    /// Retorna buscas bem sucedidas
    pub fn successful_searches(&self) -> u64 {
        self.successful_searches.load(Ordering::Relaxed)
    }

    /// Retorna buscas que falharam
    pub fn failed_searches(&self) -> u64 {
        self.failed_searches.load(Ordering::Relaxed)
    }

    /// Calcula taxa de sucesso (0.0 - 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_searches();
        if total == 0 {
            return 0.0;
        }
        self.successful_searches() as f64 / total as f64
    }

    /// Calcula média de resultados por query
    pub fn avg_results_per_query(&self) -> f64 {
        let total = self.total_searches();
        if total == 0 {
            return 0.0;
        }
        self.total_results.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Calcula taxa de cache hit (0.0 - 1.0)
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            return 0.0;
        }
        hits as f64 / total as f64
    }

    /// Calcula bytes por segundo
    pub fn bytes_per_second(&self) -> f64 {
        let total_ms = self.total_execution_time_ms.load(Ordering::Relaxed);
        if total_ms == 0 {
            return 0.0;
        }
        let total_bytes = self.total_bytes.load(Ordering::Relaxed) as f64;
        let total_seconds = total_ms as f64 / 1000.0;
        total_bytes / total_seconds
    }

    /// Calcula latência média em ms
    pub fn avg_latency(&self) -> f64 {
        let total = self.total_searches();
        if total == 0 {
            return 0.0;
        }
        self.total_latency_ms.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Calcula percentil de latência
    fn calculate_percentile(&self, percentile: f64) -> u64 {
        if let Ok(latencies) = self.latencies.read() {
            if latencies.is_empty() {
                return 0;
            }

            let mut sorted: Vec<u64> = latencies.iter().copied().collect();
            sorted.sort_unstable();

            let idx = ((percentile / 100.0) * (sorted.len() - 1) as f64).round() as usize;
            sorted.get(idx).copied().unwrap_or(0)
        } else {
            0
        }
    }

    /// Retorna latência p50 (mediana)
    pub fn latency_p50(&self) -> u64 {
        self.calculate_percentile(50.0)
    }

    /// Retorna latência p95
    pub fn latency_p95(&self) -> u64 {
        self.calculate_percentile(95.0)
    }

    /// Retorna latência p99
    pub fn latency_p99(&self) -> u64 {
        self.calculate_percentile(99.0)
    }

    /// Retorna latência mínima
    pub fn latency_min(&self) -> u64 {
        if let Ok(latencies) = self.latencies.read() {
            latencies.iter().min().copied().unwrap_or(0)
        } else {
            0
        }
    }

    /// Retorna latência máxima
    pub fn latency_max(&self) -> u64 {
        if let Ok(latencies) = self.latencies.read() {
            latencies.iter().max().copied().unwrap_or(0)
        } else {
            0
        }
    }

    /// Retorna total de bytes recebidos
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }

    /// Retorna total de resultados
    pub fn total_results(&self) -> u64 {
        self.total_results.load(Ordering::Relaxed)
    }

    /// Gera snapshot das métricas atuais
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            id: self.id,
            timestamp: Utc::now(),
            total_searches: self.total_searches(),
            successful_searches: self.successful_searches(),
            failed_searches: self.failed_searches(),
            success_rate: self.success_rate(),
            avg_results_per_query: self.avg_results_per_query(),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            cache_hit_rate: self.cache_hit_rate(),
            total_bytes: self.total_bytes(),
            bytes_per_second: self.bytes_per_second(),
            latency_avg: self.avg_latency(),
            latency_p50: self.latency_p50(),
            latency_p95: self.latency_p95(),
            latency_p99: self.latency_p99(),
            latency_min: self.latency_min(),
            latency_max: self.latency_max(),
        }
    }

    /// Reseta todas as métricas
    pub fn reset(&self) {
        self.total_searches.store(0, Ordering::Relaxed);
        self.successful_searches.store(0, Ordering::Relaxed);
        self.failed_searches.store(0, Ordering::Relaxed);
        self.total_results.store(0, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.total_latency_ms.store(0, Ordering::Relaxed);
        self.total_execution_time_ms.store(0, Ordering::Relaxed);

        if let Ok(mut latencies) = self.latencies.write() {
            latencies.clear();
        }
    }

    /// Retorna resumo formatado
    pub fn summary(&self) -> String {
        format!(
            "SearchMetrics [{}]\n\
             Total: {} searches ({} success, {} failed)\n\
             Success rate: {:.1}%\n\
             Avg results/query: {:.1}\n\
             Cache: {:.1}% hit rate ({} hits, {} misses)\n\
             Latency: avg={:.0}ms, p50={}ms, p95={}ms, p99={}ms\n\
             Throughput: {:.1} KB/s",
            &self.id.to_string()[..8],
            self.total_searches(),
            self.successful_searches(),
            self.failed_searches(),
            self.success_rate() * 100.0,
            self.avg_results_per_query(),
            self.cache_hit_rate() * 100.0,
            self.cache_hits.load(Ordering::Relaxed),
            self.cache_misses.load(Ordering::Relaxed),
            self.avg_latency(),
            self.latency_p50(),
            self.latency_p95(),
            self.latency_p99(),
            self.bytes_per_second() / 1024.0
        )
    }
}

impl Default for SearchMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SearchMetrics {
    fn clone(&self) -> Self {
        let new = Self::new();

        // Copiar valores atômicos
        new.total_searches.store(self.total_searches.load(Ordering::Relaxed), Ordering::Relaxed);
        new.successful_searches.store(self.successful_searches.load(Ordering::Relaxed), Ordering::Relaxed);
        new.failed_searches.store(self.failed_searches.load(Ordering::Relaxed), Ordering::Relaxed);
        new.total_results.store(self.total_results.load(Ordering::Relaxed), Ordering::Relaxed);
        new.total_bytes.store(self.total_bytes.load(Ordering::Relaxed), Ordering::Relaxed);
        new.cache_hits.store(self.cache_hits.load(Ordering::Relaxed), Ordering::Relaxed);
        new.cache_misses.store(self.cache_misses.load(Ordering::Relaxed), Ordering::Relaxed);
        new.total_latency_ms.store(self.total_latency_ms.load(Ordering::Relaxed), Ordering::Relaxed);
        new.total_execution_time_ms.store(self.total_execution_time_ms.load(Ordering::Relaxed), Ordering::Relaxed);

        // Copiar latências
        if let (Ok(src), Ok(mut dst)) = (self.latencies.read(), new.latencies.write()) {
            *dst = src.clone();
        }

        new
    }
}

/// Snapshot imutável das métricas em um ponto no tempo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// ID da instância de métricas
    pub id: Uuid,
    /// Timestamp do snapshot
    pub timestamp: DateTime<Utc>,
    /// Total de buscas
    pub total_searches: u64,
    /// Buscas bem sucedidas
    pub successful_searches: u64,
    /// Buscas que falharam
    pub failed_searches: u64,
    /// Taxa de sucesso (0.0 - 1.0)
    pub success_rate: f64,
    /// Média de resultados por query
    pub avg_results_per_query: f64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Taxa de cache hit (0.0 - 1.0)
    pub cache_hit_rate: f64,
    /// Total de bytes recebidos
    pub total_bytes: u64,
    /// Bytes por segundo
    pub bytes_per_second: f64,
    /// Latência média em ms
    pub latency_avg: f64,
    /// Latência p50 em ms
    pub latency_p50: u64,
    /// Latência p95 em ms
    pub latency_p95: u64,
    /// Latência p99 em ms
    pub latency_p99: u64,
    /// Latência mínima em ms
    pub latency_min: u64,
    /// Latência máxima em ms
    pub latency_max: u64,
}

impl MetricsSnapshot {
    /// Compara com outro snapshot e retorna diferenças
    pub fn diff(&self, other: &MetricsSnapshot) -> MetricsDiff {
        MetricsDiff {
            searches_diff: self.total_searches as i64 - other.total_searches as i64,
            success_rate_diff: self.success_rate - other.success_rate,
            cache_hit_rate_diff: self.cache_hit_rate - other.cache_hit_rate,
            latency_p50_diff: self.latency_p50 as i64 - other.latency_p50 as i64,
            latency_p95_diff: self.latency_p95 as i64 - other.latency_p95 as i64,
            bytes_per_second_diff: self.bytes_per_second - other.bytes_per_second,
        }
    }
}

/// Diferença entre dois snapshots de métricas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDiff {
    /// Diferença no total de buscas
    pub searches_diff: i64,
    /// Diferença na taxa de sucesso
    pub success_rate_diff: f64,
    /// Diferença na taxa de cache hit
    pub cache_hit_rate_diff: f64,
    /// Diferença na latência p50
    pub latency_p50_diff: i64,
    /// Diferença na latência p95
    pub latency_p95_diff: i64,
    /// Diferença no throughput
    pub bytes_per_second_diff: f64,
}

impl MetricsDiff {
    /// Verifica se houve melhoria geral
    pub fn is_improvement(&self) -> bool {
        // Melhoria = mais buscas, melhor sucesso, melhor cache, menor latência, mais throughput
        self.searches_diff >= 0
            && self.success_rate_diff >= 0.0
            && self.cache_hit_rate_diff >= 0.0
            && self.latency_p50_diff <= 0
            && self.latency_p95_diff <= 0
            && self.bytes_per_second_diff >= 0.0
    }

    /// Retorna resumo formatado das diferenças
    pub fn summary(&self) -> String {
        let arrow = |v: f64| if v > 0.0 { "↑" } else if v < 0.0 { "↓" } else { "→" };
        let arrow_i = |v: i64| if v > 0 { "↑" } else if v < 0 { "↓" } else { "→" };

        format!(
            "Diff: searches {} {} | success {} {:.1}% | cache {} {:.1}% | p50 {} {}ms | throughput {} {:.1} KB/s",
            arrow_i(self.searches_diff), self.searches_diff.abs(),
            arrow(self.success_rate_diff), self.success_rate_diff.abs() * 100.0,
            arrow(self.cache_hit_rate_diff), self.cache_hit_rate_diff.abs() * 100.0,
            arrow_i(self.latency_p50_diff), self.latency_p50_diff.abs(),
            arrow(self.bytes_per_second_diff), self.bytes_per_second_diff.abs() / 1024.0
        )
    }
}

/// Coletor global de métricas (thread-safe)
///
/// Singleton para coleta de métricas em toda a aplicação.
///
/// # Exemplo
///
/// ```rust,ignore
/// let collector = MetricsCollector::global();
/// collector.record_search(150, true, 10, 5000);
/// println!("{}", collector.metrics().summary());
/// ```
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    metrics: Arc<SearchMetrics>,
    /// Histórico de snapshots
    history: Arc<RwLock<VecDeque<MetricsSnapshot>>>,
    /// Tamanho máximo do histórico
    max_history: usize,
}

impl MetricsCollector {
    /// Cria um novo coletor
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(SearchMetrics::new()),
            history: Arc::new(RwLock::new(VecDeque::new())),
            max_history: 100,
        }
    }

    /// Cria um coletor com tamanho de histórico customizado
    pub fn with_history_size(max_history: usize) -> Self {
        Self {
            metrics: Arc::new(SearchMetrics::new()),
            history: Arc::new(RwLock::new(VecDeque::new())),
            max_history,
        }
    }

    /// Retorna referência às métricas
    pub fn metrics(&self) -> &SearchMetrics {
        &self.metrics
    }

    /// Registra uma busca
    pub fn record_search(&self, latency_ms: u64, success: bool, results_count: usize, bytes: usize) {
        self.metrics.record_search(latency_ms, success, results_count, bytes);
    }

    /// Registra cache hit
    pub fn record_cache_hit(&self) {
        self.metrics.record_cache_hit();
    }

    /// Registra cache miss
    pub fn record_cache_miss(&self) {
        self.metrics.record_cache_miss();
    }

    /// Adiciona tempo de execução
    pub fn add_execution_time(&self, ms: u64) {
        self.metrics.add_execution_time(ms);
    }

    /// Captura snapshot atual e adiciona ao histórico
    pub fn capture_snapshot(&self) -> MetricsSnapshot {
        let snapshot = self.metrics.snapshot();

        if let Ok(mut history) = self.history.write() {
            if history.len() >= self.max_history {
                history.pop_front();
            }
            history.push_back(snapshot.clone());
        }

        snapshot
    }

    /// Retorna último snapshot do histórico
    pub fn last_snapshot(&self) -> Option<MetricsSnapshot> {
        self.history.read().ok()?.back().cloned()
    }

    /// Retorna histórico de snapshots
    pub fn history(&self) -> Vec<MetricsSnapshot> {
        self.history.read()
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Compara snapshot atual com anterior
    pub fn compare_with_last(&self) -> Option<MetricsDiff> {
        let current = self.metrics.snapshot();
        let last = self.last_snapshot()?;
        Some(current.diff(&last))
    }

    /// Reseta métricas e histórico
    pub fn reset(&self) {
        self.metrics.reset();
        if let Ok(mut history) = self.history.write() {
            history.clear();
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer para medir latência de operações
///
/// # Exemplo
///
/// ```rust,ignore
/// let timer = LatencyTimer::start();
/// // ... operação ...
/// let latency = timer.stop();
/// metrics.record_search(latency, true, 10, 5000);
/// ```
#[derive(Debug)]
pub struct LatencyTimer {
    start: std::time::Instant,
}

impl LatencyTimer {
    /// Inicia o timer
    pub fn start() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }

    /// Para o timer e retorna latência em ms
    pub fn stop(self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// Retorna tempo decorrido sem parar
    pub fn elapsed(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = SearchMetrics::new();
        assert_eq!(metrics.total_searches(), 0);
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_record_search() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 5000);
        metrics.record_search(150, true, 8, 4000);
        metrics.record_search(200, false, 0, 0);

        assert_eq!(metrics.total_searches(), 3);
        assert_eq!(metrics.successful_searches(), 2);
        assert_eq!(metrics.failed_searches(), 1);
        assert_eq!(metrics.total_results(), 18);
        assert_eq!(metrics.total_bytes(), 9000);
    }

    #[test]
    fn test_success_rate() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 5000);
        metrics.record_search(100, true, 10, 5000);
        metrics.record_search(100, false, 0, 0);
        metrics.record_search(100, false, 0, 0);

        assert!((metrics.success_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_hit_rate() {
        let metrics = SearchMetrics::new();

        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();

        assert!((metrics.cache_hit_rate() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_avg_results_per_query() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 5000);
        metrics.record_search(100, true, 20, 5000);

        assert!((metrics.avg_results_per_query() - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_latency_percentiles() {
        let metrics = SearchMetrics::new();

        // Adicionar latências de 1 a 100
        for i in 1..=100 {
            metrics.record_search(i, true, 1, 100);
        }

        // p50 deve ser ~50
        assert!(metrics.latency_p50() >= 45 && metrics.latency_p50() <= 55);
        // p95 deve ser ~95
        assert!(metrics.latency_p95() >= 90 && metrics.latency_p95() <= 99);
        // p99 deve ser ~99
        assert!(metrics.latency_p99() >= 95 && metrics.latency_p99() <= 100);
    }

    #[test]
    fn test_latency_min_max() {
        let metrics = SearchMetrics::new();

        metrics.record_search(50, true, 1, 100);
        metrics.record_search(100, true, 1, 100);
        metrics.record_search(200, true, 1, 100);

        assert_eq!(metrics.latency_min(), 50);
        assert_eq!(metrics.latency_max(), 200);
    }

    #[test]
    fn test_bytes_per_second() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 10000);
        metrics.add_execution_time(1000); // 1 segundo

        // 10000 bytes em 1 segundo = 10000 bytes/s
        assert!((metrics.bytes_per_second() - 10000.0).abs() < 1.0);
    }

    #[test]
    fn test_snapshot() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 5000);
        metrics.record_cache_hit();

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.total_searches, 1);
        assert_eq!(snapshot.successful_searches, 1);
        assert_eq!(snapshot.cache_hits, 1);
    }

    #[test]
    fn test_reset() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 5000);
        metrics.record_cache_hit();

        metrics.reset();

        assert_eq!(metrics.total_searches(), 0);
        assert_eq!(metrics.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_collector() {
        let collector = MetricsCollector::new();

        collector.record_search(100, true, 10, 5000);
        collector.record_search(150, true, 8, 4000);

        assert_eq!(collector.metrics().total_searches(), 2);
    }

    #[test]
    fn test_collector_history() {
        let collector = MetricsCollector::new();

        collector.record_search(100, true, 10, 5000);
        let _snap1 = collector.capture_snapshot();

        collector.record_search(150, true, 8, 4000);
        let _snap2 = collector.capture_snapshot();

        let history = collector.history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].total_searches, 1);
        assert_eq!(history[1].total_searches, 2);
    }

    #[test]
    fn test_snapshot_diff() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 5000);
        let snap1 = metrics.snapshot();

        metrics.record_search(150, true, 8, 4000);
        let snap2 = metrics.snapshot();

        let diff = snap2.diff(&snap1);
        assert_eq!(diff.searches_diff, 1);
    }

    #[test]
    fn test_latency_timer() {
        let timer = LatencyTimer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let latency = timer.stop();

        // Deve ser pelo menos 10ms
        assert!(latency >= 10);
    }

    #[test]
    fn test_summary() {
        let metrics = SearchMetrics::new();

        metrics.record_search(100, true, 10, 5000);

        let summary = metrics.summary();
        assert!(summary.contains("SearchMetrics"));
        assert!(summary.contains("1 searches"));
    }

    #[test]
    fn test_clone() {
        let metrics = SearchMetrics::new();
        metrics.record_search(100, true, 10, 5000);

        let cloned = metrics.clone();

        assert_eq!(cloned.total_searches(), 1);
        assert_eq!(cloned.total_bytes(), 5000);
    }

    #[test]
    fn test_buffer_circular() {
        let metrics = SearchMetrics::new();

        // Adicionar mais do que o buffer suporta
        for i in 0..LATENCY_BUFFER_SIZE + 100 {
            metrics.record_search(i as u64, true, 1, 100);
        }

        // Buffer deve manter apenas as últimas LATENCY_BUFFER_SIZE entradas
        // Verificar através do min (que seria 100 se o buffer circular funcionou)
        assert!(metrics.latency_min() >= 100);
    }
}

