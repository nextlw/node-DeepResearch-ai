// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CACHE DE BUSCA (SEARCH CACHE)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Sistema de cache para resultados de busca com TTL configurável.
// Reduz chamadas repetidas à API e mede eficiência do cache.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Duration;

use crate::search_metrics::MetricsCollector;
use crate::types::SerpQuery;

/// Configuração do cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// TTL padrão para entradas (em segundos)
    pub default_ttl_secs: u64,
    /// Tamanho máximo do cache (número de entradas)
    pub max_entries: usize,
    /// Se deve limpar entradas expiradas automaticamente
    pub auto_cleanup: bool,
    /// Intervalo de limpeza automática (em segundos)
    pub cleanup_interval_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl_secs: 300, // 5 minutos
            max_entries: 1000,
            auto_cleanup: true,
            cleanup_interval_secs: 60,
        }
    }
}

impl CacheConfig {
    /// Cria configuração para cache de curta duração
    pub fn short_lived() -> Self {
        Self {
            default_ttl_secs: 60, // 1 minuto
            max_entries: 500,
            ..Default::default()
        }
    }

    /// Cria configuração para cache de longa duração
    pub fn long_lived() -> Self {
        Self {
            default_ttl_secs: 3600, // 1 hora
            max_entries: 5000,
            ..Default::default()
        }
    }

    /// Cria configuração para testes (TTL longo)
    pub fn for_tests() -> Self {
        Self {
            default_ttl_secs: 86400, // 24 horas
            max_entries: 100,
            auto_cleanup: false,
            cleanup_interval_secs: 86400,
        }
    }
}

/// Chave do cache (baseada na query)
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Query normalizada
    pub query: String,
    /// Filtro temporal (opcional)
    pub tbs: Option<String>,
    /// Localização (opcional)
    pub location: Option<String>,
}

impl CacheKey {
    /// Cria chave a partir de SerpQuery
    pub fn from_query(query: &SerpQuery) -> Self {
        Self {
            query: query.q.to_lowercase().trim().to_string(),
            tbs: query.tbs.clone(),
            location: query.location.clone(),
        }
    }

    /// Retorna representação única da chave
    pub fn to_string_key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.query,
            self.tbs.as_deref().unwrap_or(""),
            self.location.as_deref().unwrap_or("")
        )
    }
}

impl From<&SerpQuery> for CacheKey {
    fn from(query: &SerpQuery) -> Self {
        Self::from_query(query)
    }
}

/// Entrada do cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// Dados armazenados
    pub data: T,
    /// Timestamp de criação
    pub created_at: DateTime<Utc>,
    /// Timestamp de expiração
    pub expires_at: DateTime<Utc>,
    /// Número de vezes que foi acessado
    pub hit_count: u64,
    /// Último acesso
    pub last_accessed: DateTime<Utc>,
}

impl<T: Clone> CacheEntry<T> {
    /// Cria nova entrada com TTL em segundos
    pub fn new(data: T, ttl_secs: u64) -> Self {
        let now = Utc::now();
        Self {
            data,
            created_at: now,
            expires_at: now + ChronoDuration::seconds(ttl_secs as i64),
            hit_count: 0,
            last_accessed: now,
        }
    }

    /// Verifica se a entrada expirou
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Retorna tempo restante até expiração
    pub fn time_to_live(&self) -> Duration {
        let now = Utc::now();
        if now > self.expires_at {
            Duration::ZERO
        } else {
            let diff = self.expires_at - now;
            Duration::from_secs(diff.num_seconds().max(0) as u64)
        }
    }

    /// Marca como acessado
    pub fn touch(&mut self) {
        self.hit_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Retorna idade da entrada em segundos
    pub fn age_secs(&self) -> u64 {
        let diff = Utc::now() - self.created_at;
        diff.num_seconds().max(0) as u64
    }
}

/// Estatísticas do cache
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total de hits
    pub hits: u64,
    /// Total de misses
    pub misses: u64,
    /// Entradas atuais
    pub entries: usize,
    /// Bytes aproximados em uso
    pub bytes_used: usize,
    /// Entradas expiradas removidas
    pub evictions: u64,
    /// Taxa de hit (0.0 - 1.0)
    pub hit_rate: f64,
}

impl CacheStats {
    /// Atualiza taxa de hit
    pub fn update_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }
}

/// Cache genérico thread-safe com TTL
///
/// # Exemplo
///
/// ```rust,ignore
/// let cache: SearchCache<SearchResult> = SearchCache::new(CacheConfig::default());
///
/// // Armazenar resultado
/// let key = CacheKey::from_query(&query);
/// cache.set(key.clone(), result);
///
/// // Recuperar resultado
/// if let Some(cached) = cache.get(&key) {
///     println!("Cache hit!");
/// }
/// ```
pub struct SearchCache<T: Clone + Send + Sync> {
    /// Armazenamento interno
    store: RwLock<HashMap<CacheKey, CacheEntry<T>>>,
    /// Configuração
    config: CacheConfig,
    /// Estatísticas
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    /// Coletor de métricas (opcional)
    metrics: Option<MetricsCollector>,
}

impl<T: Clone + Send + Sync> SearchCache<T> {
    /// Cria novo cache com configuração padrão
    pub fn new(config: CacheConfig) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            metrics: None,
        }
    }

    /// Cria cache com coletor de métricas
    pub fn with_metrics(config: CacheConfig, metrics: MetricsCollector) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            metrics: Some(metrics),
        }
    }

    /// Armazena valor no cache com TTL padrão
    pub fn set(&self, key: CacheKey, value: T) {
        self.set_with_ttl(key, value, self.config.default_ttl_secs);
    }

    /// Armazena valor no cache com TTL customizado
    pub fn set_with_ttl(&self, key: CacheKey, value: T, ttl_secs: u64) {
        if let Ok(mut store) = self.store.write() {
            // Verificar limite de entradas
            if store.len() >= self.config.max_entries {
                self.evict_oldest(&mut store);
            }

            let entry = CacheEntry::new(value, ttl_secs);
            store.insert(key, entry);
        }
    }

    /// Recupera valor do cache
    pub fn get(&self, key: &CacheKey) -> Option<T> {
        // Primeiro tenta leitura rápida
        if let Ok(store) = self.store.read() {
            if let Some(entry) = store.get(key) {
                if !entry.is_expired() {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref metrics) = self.metrics {
                        metrics.record_cache_hit();
                    }
                    return Some(entry.data.clone());
                }
            }
        }

        // Miss ou expirado
        self.misses.fetch_add(1, Ordering::Relaxed);
        if let Some(ref metrics) = self.metrics {
            metrics.record_cache_miss();
        }

        // Se expirado, remover
        if let Ok(mut store) = self.store.write() {
            if let Some(entry) = store.get(key) {
                if entry.is_expired() {
                    store.remove(key);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        None
    }

    /// Recupera valor e atualiza contador de hits
    pub fn get_and_touch(&self, key: &CacheKey) -> Option<T> {
        if let Ok(mut store) = self.store.write() {
            if let Some(entry) = store.get_mut(key) {
                if !entry.is_expired() {
                    entry.touch();
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref metrics) = self.metrics {
                        metrics.record_cache_hit();
                    }
                    return Some(entry.data.clone());
                } else {
                    // Expirado, remover
                    store.remove(key);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        if let Some(ref metrics) = self.metrics {
            metrics.record_cache_miss();
        }
        None
    }

    /// Verifica se chave existe e não expirou
    pub fn contains(&self, key: &CacheKey) -> bool {
        if let Ok(store) = self.store.read() {
            if let Some(entry) = store.get(key) {
                return !entry.is_expired();
            }
        }
        false
    }

    /// Remove entrada do cache
    pub fn remove(&self, key: &CacheKey) -> Option<T> {
        if let Ok(mut store) = self.store.write() {
            return store.remove(key).map(|e| e.data);
        }
        None
    }

    /// Limpa todas as entradas
    pub fn clear(&self) {
        if let Ok(mut store) = self.store.write() {
            store.clear();
        }
    }

    /// Remove entradas expiradas
    pub fn cleanup(&self) -> usize {
        let mut removed = 0;
        if let Ok(mut store) = self.store.write() {
            let expired_keys: Vec<CacheKey> = store
                .iter()
                .filter(|(_, entry)| entry.is_expired())
                .map(|(key, _)| key.clone())
                .collect();

            for key in expired_keys {
                store.remove(&key);
                removed += 1;
            }
        }

        self.evictions.fetch_add(removed as u64, Ordering::Relaxed);
        removed
    }

    /// Remove a entrada mais antiga
    fn evict_oldest(&self, store: &mut HashMap<CacheKey, CacheEntry<T>>) {
        if let Some(oldest_key) = store
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone())
        {
            store.remove(&oldest_key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Retorna número de entradas
    pub fn len(&self) -> usize {
        self.store.read().map(|s| s.len()).unwrap_or(0)
    }

    /// Verifica se está vazio
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Retorna estatísticas do cache
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        CacheStats {
            hits,
            misses,
            entries: self.len(),
            bytes_used: 0, // Seria necessário serializar para calcular
            evictions: self.evictions.load(Ordering::Relaxed),
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Retorna taxa de hit
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let total = hits + self.misses.load(Ordering::Relaxed);
        if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Reseta estatísticas
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
    }

    /// Retorna configuração
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Retorna resumo formatado
    pub fn summary(&self) -> String {
        let stats = self.stats();
        format!(
            "SearchCache: {} entries | {:.1}% hit rate ({} hits, {} misses) | {} evictions | TTL: {}s",
            stats.entries,
            stats.hit_rate * 100.0,
            stats.hits,
            stats.misses,
            stats.evictions,
            self.config.default_ttl_secs
        )
    }

    /// Lista todas as chaves (não expiradas)
    pub fn keys(&self) -> Vec<CacheKey> {
        if let Ok(store) = self.store.read() {
            store
                .iter()
                .filter(|(_, entry)| !entry.is_expired())
                .map(|(key, _)| key.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Retorna informações sobre uma entrada específica
    pub fn entry_info(&self, key: &CacheKey) -> Option<CacheEntryInfo> {
        if let Ok(store) = self.store.read() {
            store.get(key).map(|entry| CacheEntryInfo {
                key: key.clone(),
                created_at: entry.created_at,
                expires_at: entry.expires_at,
                hit_count: entry.hit_count,
                last_accessed: entry.last_accessed,
                is_expired: entry.is_expired(),
                ttl_remaining: entry.time_to_live(),
            })
        } else {
            None
        }
    }
}

impl<T: Clone + Send + Sync> std::fmt::Debug for SearchCache<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchCache")
            .field("entries", &self.len())
            .field("hit_rate", &self.hit_rate())
            .field("config", &self.config)
            .finish()
    }
}

/// Informações sobre uma entrada do cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryInfo {
    /// Chave
    pub key: CacheKey,
    /// Timestamp de criação
    pub created_at: DateTime<Utc>,
    /// Timestamp de expiração
    pub expires_at: DateTime<Utc>,
    /// Número de hits
    pub hit_count: u64,
    /// Último acesso
    pub last_accessed: DateTime<Utc>,
    /// Se está expirado
    pub is_expired: bool,
    /// Tempo restante
    pub ttl_remaining: Duration,
}

/// Wrapper para SearchResult no cache
/// (definido aqui para evitar dependência circular)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSearchResult {
    /// URLs encontradas
    pub urls: Vec<CachedSnippet>,
    /// Snippets de texto
    pub snippets: Vec<String>,
    /// Total de resultados
    pub total_results: u64,
}

/// Snippet simplificado para cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSnippet {
    /// URL
    pub url: String,
    /// Título
    pub title: String,
    /// Descrição
    pub description: String,
    /// Score final
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_from_query() {
        let query = SerpQuery {
            q: "Test Query".into(),
            tbs: Some("qdr:m".into()),
            location: None,
        };

        let key = CacheKey::from_query(&query);
        assert_eq!(key.query, "test query"); // Normalizado para lowercase
        assert_eq!(key.tbs, Some("qdr:m".into()));
    }

    #[test]
    fn test_cache_key_equality() {
        let key1 = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        let key2 = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_entry_new() {
        let entry = CacheEntry::new("test data", 60);
        assert!(!entry.is_expired());
        assert_eq!(entry.hit_count, 0);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test", 0); // TTL de 0 segundos
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_touch() {
        let mut entry = CacheEntry::new("test", 60);
        assert_eq!(entry.hit_count, 0);
        
        entry.touch();
        assert_eq!(entry.hit_count, 1);
        
        entry.touch();
        assert_eq!(entry.hit_count, 2);
    }

    #[test]
    fn test_cache_set_get() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set(key.clone(), "cached value".into());
        
        let result = cache.get(&key);
        assert_eq!(result, Some("cached value".into()));
    }

    #[test]
    fn test_cache_miss() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "nonexistent".into(),
            tbs: None,
            location: None,
        };
        
        let result = cache.get(&key);
        assert_eq!(result, None);
    }

    #[test]
    fn test_cache_expiration() {
        let config = CacheConfig {
            default_ttl_secs: 0, // Expira imediatamente
            ..Default::default()
        };
        let cache: SearchCache<String> = SearchCache::new(config);
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set(key.clone(), "value".into());
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let result = cache.get(&key);
        assert_eq!(result, None);
    }

    #[test]
    fn test_cache_remove() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set(key.clone(), "value".into());
        assert!(cache.contains(&key));
        
        let removed = cache.remove(&key);
        assert_eq!(removed, Some("value".into()));
        assert!(!cache.contains(&key));
    }

    #[test]
    fn test_cache_clear() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        for i in 0..5 {
            let key = CacheKey {
                query: format!("test{}", i),
                tbs: None,
                location: None,
            };
            cache.set(key, format!("value{}", i));
        }
        
        assert_eq!(cache.len(), 5);
        
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_cleanup() {
        let config = CacheConfig {
            default_ttl_secs: 0, // Expira imediatamente
            ..Default::default()
        };
        let cache: SearchCache<String> = SearchCache::new(config);
        
        for i in 0..5 {
            let key = CacheKey {
                query: format!("test{}", i),
                tbs: None,
                location: None,
            };
            cache.set(key, format!("value{}", i));
        }
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let removed = cache.cleanup();
        assert_eq!(removed, 5);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_max_entries() {
        let config = CacheConfig {
            max_entries: 3,
            default_ttl_secs: 300,
            ..Default::default()
        };
        let cache: SearchCache<String> = SearchCache::new(config);
        
        for i in 0..5 {
            let key = CacheKey {
                query: format!("test{}", i),
                tbs: None,
                location: None,
            };
            cache.set(key, format!("value{}", i));
        }
        
        // Deve manter apenas 3 entradas
        assert!(cache.len() <= 3);
    }

    #[test]
    fn test_cache_stats() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set(key.clone(), "value".into());
        
        // 2 hits
        cache.get(&key);
        cache.get(&key);
        
        // 1 miss
        let missing = CacheKey {
            query: "missing".into(),
            tbs: None,
            location: None,
        };
        cache.get(&missing);
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_hit_rate() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set(key.clone(), "value".into());
        
        cache.get(&key); // hit
        cache.get(&key); // hit
        cache.get(&key); // hit
        
        let missing = CacheKey {
            query: "missing".into(),
            tbs: None,
            location: None,
        };
        cache.get(&missing); // miss
        
        assert!((cache.hit_rate() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_cache_contains() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        assert!(!cache.contains(&key));
        
        cache.set(key.clone(), "value".into());
        assert!(cache.contains(&key));
    }

    #[test]
    fn test_cache_keys() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        for i in 0..3 {
            let key = CacheKey {
                query: format!("test{}", i),
                tbs: None,
                location: None,
            };
            cache.set(key, format!("value{}", i));
        }
        
        let keys = cache.keys();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn test_cache_entry_info() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set(key.clone(), "value".into());
        
        let info = cache.entry_info(&key);
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert!(!info.is_expired);
        assert_eq!(info.hit_count, 0);
    }

    #[test]
    fn test_cache_get_and_touch() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set(key.clone(), "value".into());
        
        // Primeiro acesso
        cache.get_and_touch(&key);
        
        let info = cache.entry_info(&key).unwrap();
        assert_eq!(info.hit_count, 1);
        
        // Segundo acesso
        cache.get_and_touch(&key);
        
        let info = cache.entry_info(&key).unwrap();
        assert_eq!(info.hit_count, 2);
    }

    #[test]
    fn test_cache_summary() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        cache.set(key.clone(), "value".into());
        cache.get(&key);
        
        let summary = cache.summary();
        assert!(summary.contains("SearchCache"));
        assert!(summary.contains("1 entries"));
    }

    #[test]
    fn test_config_presets() {
        let short = CacheConfig::short_lived();
        assert_eq!(short.default_ttl_secs, 60);
        
        let long = CacheConfig::long_lived();
        assert_eq!(long.default_ttl_secs, 3600);
    }

    #[test]
    fn test_cache_with_custom_ttl() {
        let cache: SearchCache<String> = SearchCache::new(CacheConfig::for_tests());
        
        let key = CacheKey {
            query: "test".into(),
            tbs: None,
            location: None,
        };
        
        cache.set_with_ttl(key.clone(), "value".into(), 0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // Deve ter expirado
        assert!(cache.get(&key).is_none());
    }
}
