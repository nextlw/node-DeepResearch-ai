// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CLIENTE DE BUSCA
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Trait e implementações para busca web e leitura de URLs.
// Suporta múltiplos provedores: Jina, SerpAPI, Brave, etc.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use async_trait::async_trait;
use crate::types::{SerpQuery, BoostedSearchSnippet, Url};

/// Erros do cliente de busca
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Search API error: {0}")]
    ApiError(String),

    #[error("Rate limit exceeded")]
    RateLimitError,

    #[error("URL fetch failed: {0}")]
    FetchError(String),

    #[error("Content extraction failed: {0}")]
    ExtractionError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Resultado de uma busca
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub urls: Vec<BoostedSearchSnippet>,
    pub snippets: Vec<String>,
    pub total_results: u64,
}

/// Conteúdo extraído de uma URL
#[derive(Debug, Clone)]
pub struct UrlContent {
    pub title: String,
    pub text: String,
    pub url: String,
    pub word_count: usize,
}

/// Trait principal para clientes de busca
///
/// Define a interface para busca web e leitura de conteúdo.
#[async_trait]
pub trait SearchClient: Send + Sync {
    /// Executa uma única busca
    async fn search(&self, query: &SerpQuery) -> Result<SearchResult, SearchError>;

    /// Executa múltiplas buscas em paralelo
    async fn search_batch(
        &self,
        queries: &[SerpQuery],
    ) -> Vec<Result<SearchResult, SearchError>>;

    /// Lê o conteúdo de uma URL
    async fn read_url(&self, url: &Url) -> Result<UrlContent, SearchError>;

    /// Lê múltiplas URLs em paralelo
    async fn read_urls_batch(&self, urls: &[Url]) -> Vec<Result<UrlContent, SearchError>>;

    /// Rerank de URLs usando embeddings
    async fn rerank(&self, query: &str, urls: &[BoostedSearchSnippet]) -> Vec<BoostedSearchSnippet>;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO MOCK PARA TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cliente mock para testes unitários
#[derive(Debug, Default)]
pub struct MockSearchClient {
    pub mock_results: Option<SearchResult>,
    pub mock_content: Option<UrlContent>,
}

impl MockSearchClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_results(results: SearchResult) -> Self {
        Self {
            mock_results: Some(results),
            mock_content: None,
        }
    }
}

#[async_trait]
impl SearchClient for MockSearchClient {
    async fn search(&self, _query: &SerpQuery) -> Result<SearchResult, SearchError> {
        Ok(self.mock_results.clone().unwrap_or_else(|| SearchResult {
            urls: vec![],
            snippets: vec!["Mock snippet".into()],
            total_results: 0,
        }))
    }

    async fn search_batch(
        &self,
        queries: &[SerpQuery],
    ) -> Vec<Result<SearchResult, SearchError>> {
        queries.iter().map(|q| self.search(q)).collect::<Vec<_>>()
            .into_iter()
            .map(|_| Ok(SearchResult {
                urls: vec![],
                snippets: vec!["Mock batch snippet".into()],
                total_results: 0,
            }))
            .collect()
    }

    async fn read_url(&self, url: &Url) -> Result<UrlContent, SearchError> {
        Ok(self.mock_content.clone().unwrap_or_else(|| UrlContent {
            title: "Mock Title".into(),
            text: "Mock content from URL".into(),
            url: url.clone(),
            word_count: 4,
        }))
    }

    async fn read_urls_batch(&self, urls: &[Url]) -> Vec<Result<UrlContent, SearchError>> {
        urls.iter()
            .map(|url| Ok(UrlContent {
                title: "Mock Title".into(),
                text: "Mock content".into(),
                url: url.clone(),
                word_count: 2,
            }))
            .collect()
    }

    async fn rerank(&self, _query: &str, urls: &[BoostedSearchSnippet]) -> Vec<BoostedSearchSnippet> {
        urls.to_vec()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO JINA (STUB)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cliente para Jina AI APIs
pub struct JinaClient {
    api_key: String,
    search_endpoint: String,
    reader_endpoint: String,
    rerank_endpoint: String,
    client: reqwest::Client,
}

impl JinaClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            search_endpoint: "https://s.jina.ai".into(),
            reader_endpoint: "https://r.jina.ai".into(),
            rerank_endpoint: "https://api.jina.ai/v1/rerank".into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SearchClient for JinaClient {
    async fn search(&self, _query: &SerpQuery) -> Result<SearchResult, SearchError> {
        // TODO: Implementar chamada real à API Jina
        todo!("Implement Jina search")
    }

    async fn search_batch(
        &self,
        _queries: &[SerpQuery],
    ) -> Vec<Result<SearchResult, SearchError>> {
        // TODO: Implementar chamadas paralelas
        todo!("Implement Jina search_batch")
    }

    async fn read_url(&self, _url: &Url) -> Result<UrlContent, SearchError> {
        // TODO: Implementar chamada ao Jina Reader
        todo!("Implement Jina read_url")
    }

    async fn read_urls_batch(&self, _urls: &[Url]) -> Vec<Result<UrlContent, SearchError>> {
        // TODO: Implementar leitura paralela
        todo!("Implement Jina read_urls_batch")
    }

    async fn rerank(&self, _query: &str, _urls: &[BoostedSearchSnippet]) -> Vec<BoostedSearchSnippet> {
        // TODO: Implementar rerank com Jina
        todo!("Implement Jina rerank")
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// UTILITÁRIOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Extrai hostname de uma URL
pub fn extract_hostname(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
}

/// Calcula boost baseado no hostname
pub fn hostname_boost(hostname: &str) -> f32 {
    // Domínios confiáveis recebem boost
    let trusted = [
        "wikipedia.org",
        "arxiv.org",
        "github.com",
        "stackoverflow.com",
        "docs.rs",
        "rust-lang.org",
    ];

    if trusted.iter().any(|t| hostname.contains(t)) {
        1.5
    } else {
        1.0
    }
}

/// Calcula boost baseado no path da URL
pub fn path_boost(url: &str) -> f32 {
    // Paths com indicadores de qualidade
    if url.contains("/docs/")
        || url.contains("/documentation/")
        || url.contains("/guide/")
        || url.contains("/tutorial/")
    {
        1.3
    } else if url.contains("/blog/") || url.contains("/news/") {
        1.1
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hostname() {
        assert_eq!(
            extract_hostname("https://www.example.com/path"),
            Some("www.example.com".into())
        );
        assert_eq!(extract_hostname("invalid"), None);
    }

    #[test]
    fn test_hostname_boost() {
        assert!(hostname_boost("en.wikipedia.org") > 1.0);
        assert_eq!(hostname_boost("random-site.com"), 1.0);
    }

    #[test]
    fn test_path_boost() {
        assert!(path_boost("https://example.com/docs/api") > 1.0);
        assert_eq!(path_boost("https://example.com/about"), 1.0);
    }

    #[tokio::test]
    async fn test_mock_search() {
        let client = MockSearchClient::new();
        let query = SerpQuery {
            q: "test query".into(),
            ..Default::default()
        };

        let result = client.search(&query).await.unwrap();
        assert_eq!(result.snippets.len(), 1);
    }
}
