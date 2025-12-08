// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CLIENTE DE BUSCA
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Trait e implementações para busca web e leitura de URLs.
// Suporta múltiplos provedores: Jina, SerpAPI, Brave, etc.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::types::{SerpQuery, BoostedSearchSnippet, Url};

/// Erros que podem ocorrer em operações de busca.
///
/// Cobre tanto erros de busca na API quanto erros
/// ao ler e extrair conteúdo de URLs.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// Erro retornado pela API de busca (Jina, SerpAPI, etc).
    ///
    /// Exemplos: API key inválida, query malformada, serviço fora.
    #[error("Search API error: {0}")]
    ApiError(String),

    /// Limite de requisições excedido na API de busca.
    ///
    /// Aguarde antes de tentar novamente.
    #[error("Rate limit exceeded")]
    RateLimitError,

    /// Falha ao baixar conteúdo de uma URL.
    ///
    /// O site pode estar fora, bloqueando acesso, ou retornando erro.
    #[error("URL fetch failed: {0}")]
    FetchError(String),

    /// Falha ao extrair texto do HTML da página.
    ///
    /// A página pode ter estrutura incomum ou ser
    /// majoritariamente JavaScript (SPA).
    #[error("Content extraction failed: {0}")]
    ExtractionError(String),

    /// Erro de rede (DNS, timeout, conexão recusada).
    #[error("Network error: {0}")]
    NetworkError(String),

    /// URL fornecida está malformada ou é inválida.
    ///
    /// Verifique se a URL começa com http:// ou https://.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Erro ao interpretar resposta da API de busca.
    ///
    /// O formato da resposta não é o esperado.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Resultado de uma busca na web.
///
/// Contém os URLs encontrados (com scores de relevância),
/// snippets de texto para preview, e o total de resultados.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// URLs encontradas com metadados e scores de ranking.
    ///
    /// Já vem ordenadas por relevância (maior score primeiro).
    pub urls: Vec<BoostedSearchSnippet>,
    /// Snippets de texto extraídos dos resultados.
    ///
    /// Úteis para preview rápido sem precisar ler a página.
    pub snippets: Vec<String>,
    /// Total de resultados encontrados pela API.
    ///
    /// Pode ser maior que `urls.len()` pois só retornamos
    /// os primeiros N resultados.
    pub total_results: u64,
}

/// Conteúdo extraído de uma página web.
///
/// Após baixar e processar uma URL, o texto limpo
/// (sem HTML, scripts, ads) é armazenado aqui.
#[derive(Debug, Clone)]
pub struct UrlContent {
    /// Título da página (tag `<title>`).
    pub title: String,
    /// Texto principal extraído da página.
    ///
    /// HTML, scripts, estilos e navegação são removidos.
    /// Apenas o conteúdo relevante é mantido.
    pub text: String,
    /// URL original da página.
    pub url: String,
    /// Contagem de palavras do texto extraído.
    ///
    /// Útil para estimar tokens e filtrar páginas vazias.
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

/// Cliente mock para testes unitários.
///
/// Simula buscas e leitura de URLs sem fazer requisições reais.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct MockSearchClient {
    /// Resultado de busca a retornar quando `search` é chamado.
    pub mock_results: Option<SearchResult>,
    /// Conteúdo a retornar quando `read_url` é chamado.
    pub mock_content: Option<UrlContent>,
}

#[cfg(test)]
impl MockSearchClient {
    /// Cria um novo cliente MockSearchClient com valores padrão.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = MockSearchClient::new();
    /// ```
    pub fn new() -> Self {
        Self { mock_results: None, mock_content: None }
    }

    /// Cria um novo cliente MockSearchClient com um resultado de busca padrão.
    ///
    /// # Argumentos
    /// * `results` - O resultado de busca padrão a retornar quando `search` é chamado.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = MockSearchClient::with_results(SearchResult {
    ///     urls: vec![],
    ///     snippets: vec!["Mock snippet".into()],
    ///     total_results: 0,
    /// });
    /// ```
    pub fn with_results(results: SearchResult) -> Self {
        Self {
            mock_results: Some(results),
            mock_content: None,
        }
    }
}

#[cfg(test)]
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
    /// Chave da API Jina
    api_key: String,
    /// Endpoint para busca
    search_endpoint: String,
    /// Endpoint para leitura de URLs
    reader_endpoint: String,
    /// Endpoint para reranking
    rerank_endpoint: String,
    /// Cliente HTTP
    client: reqwest::Client,
}

impl JinaClient {
    /// Cria um novo cliente Jina AI com configurações padrão.
    ///
    /// # Argumentos
    /// * `api_key` - Sua chave de API Jina AI
    ///
    /// # Endpoints Configurados
    /// - Busca: `https://s.jina.ai`
    /// - Reader: `https://r.jina.ai`
    /// - Rerank: `https://api.jina.ai/v1/rerank`
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = JinaClient::new("jina_api_key".into());
    /// let results = client.search(&query).await?;
    /// ```
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

// Estruturas para deserialização da API Jina
#[derive(Deserialize)]
struct JinaSearchResponse {
    results: Option<Vec<JinaSearchResult>>,
    #[serde(default)]
    total: u64,
}

#[derive(Deserialize)]
struct JinaSearchResult {
    title: String,
    url: String,
    snippet: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct JinaReaderResponse {
    title: Option<String>,
    content: String,
    text: Option<String>,
}

#[derive(Serialize)]
struct JinaRerankRequest {
    model: String,
    query: String,
    documents: Vec<String>,
    top_n: Option<usize>,
}

#[derive(Deserialize)]
struct JinaRerankResponse {
    results: Vec<JinaRerankResult>,
}

#[derive(Deserialize)]
struct JinaRerankResult {
    index: usize,
    relevance_score: f32,
}

#[async_trait]
impl SearchClient for JinaClient {
    async fn search(&self, query: &SerpQuery) -> Result<SearchResult, SearchError> {
        // Construir query string com parâmetros opcionais
        let mut query_str = query.q.clone();
        if let Some(ref tbs) = query.tbs {
            query_str.push_str(&format!(" tbs:{}", tbs));
        }
        if let Some(ref location) = query.location {
            query_str.push_str(&format!(" location:{}", location));
        }

        let encoded_query = urlencoding::encode(&query_str);
        let url = format!("{}/{}", self.search_endpoint, encoded_query);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| SearchError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SearchError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SearchError::ApiError(format!("Jina Search API error: {}", error_text)));
        }

        let search_response: JinaSearchResponse = response
            .json()
            .await
            .map_err(|e| SearchError::ParseError(format!("Failed to parse search response: {}", e)))?;

        let results = search_response.results.unwrap_or_default();
        let mut snippets: Vec<BoostedSearchSnippet> = Vec::new();
        let mut snippet_strings: Vec<String> = Vec::new();

        for r in results {
            let description = r.snippet.or(r.description.clone()).unwrap_or_default();
            let hostname = extract_hostname(&r.url);
            let hostname_boost_val = hostname.as_ref()
                .map(|h| hostname_boost(h))
                .unwrap_or(1.0);
            let path_boost_val = path_boost(&r.url);

            snippets.push(BoostedSearchSnippet {
                url: r.url.clone(),
                title: r.title,
                description: description.clone(),
                weight: 1.0,
                freq_boost: 1.0,
                hostname_boost: hostname_boost_val,
                path_boost: path_boost_val,
                jina_rerank_boost: 1.0,
                final_score: 1.0,
                score: 1.0,
                merged: description.clone(),
            });

            snippet_strings.push(description);
        }

        Ok(SearchResult {
            urls: snippets,
            snippets: snippet_strings,
            total_results: search_response.total,
        })
    }

    async fn search_batch(
        &self,
        queries: &[SerpQuery],
    ) -> Vec<Result<SearchResult, SearchError>> {
        use futures::future::join_all;

        let futures: Vec<_> = queries.iter()
            .map(|q| self.search(q))
            .collect();

        join_all(futures).await
    }

    async fn read_url(&self, url: &Url) -> Result<UrlContent, SearchError> {
        // Validar URL
        url::Url::parse(url)
            .map_err(|e| SearchError::InvalidUrl(format!("Invalid URL: {}", e)))?;

        let encoded_url = urlencoding::encode(url);
        let reader_url = format!("{}/{}", self.reader_endpoint, encoded_url);

        let response = self.client
            .get(&reader_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| SearchError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SearchError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SearchError::FetchError(format!("Jina Reader API error: {}", error_text)));
        }

        let reader_response: JinaReaderResponse = response
            .json()
            .await
            .map_err(|e| SearchError::ExtractionError(format!("Failed to parse reader response: {}", e)))?;

        let text = reader_response.text.unwrap_or(reader_response.content);
        let word_count = text.split_whitespace().count();

        Ok(UrlContent {
            title: reader_response.title.unwrap_or_default(),
            text,
            url: url.clone(),
            word_count,
        })
    }

    async fn read_urls_batch(&self, urls: &[Url]) -> Vec<Result<UrlContent, SearchError>> {
        use futures::future::join_all;

        let futures: Vec<_> = urls.iter()
            .map(|url| self.read_url(url))
            .collect();

        join_all(futures).await
    }

    async fn rerank(&self, query: &str, urls: &[BoostedSearchSnippet]) -> Vec<BoostedSearchSnippet> {
        if urls.is_empty() {
            return vec![];
        }

        let documents: Vec<String> = urls.iter()
            .map(|snippet| format!("{} {}", snippet.title, snippet.description))
            .collect();

        let request = JinaRerankRequest {
            model: "jina-reranker-v1-base-en".into(),
            query: query.to_string(),
            documents,
            top_n: Some(urls.len()),
        };

        let response = match self.client
            .post(&self.rerank_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => {
                // Em caso de erro, retorna URLs sem reranking
                return urls.to_vec();
            }
        };

        if !response.status().is_success() {
            // Em caso de erro, retorna URLs sem reranking
            return urls.to_vec();
        }

        let rerank_response: JinaRerankResponse = match response.json().await {
            Ok(r) => r,
            Err(_) => {
                return urls.to_vec();
            }
        };

        // Criar mapa de índices para scores
        let mut score_map = std::collections::HashMap::new();
        for result in rerank_response.results {
            score_map.insert(result.index, result.relevance_score);
        }

        // Aplicar scores de reranking aos snippets
        let mut reranked: Vec<BoostedSearchSnippet> = urls.iter()
            .enumerate()
            .map(|(idx, snippet)| {
                let rerank_score = score_map.get(&idx).copied().unwrap_or(1.0);
                let mut updated = snippet.clone();
                updated.jina_rerank_boost = rerank_score;
                updated.final_score = updated.weight
                    * updated.freq_boost
                    * updated.hostname_boost
                    * updated.path_boost
                    * rerank_score;
                updated.score = updated.final_score;
                updated
            })
            .collect();

        // Ordenar por score final
        reranked.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));

        reranked
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
