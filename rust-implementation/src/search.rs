// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CLIENTE DE BUSCA
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Trait e implementações para busca web e leitura de URLs.
// Suporta múltiplos provedores: Jina, SerpAPI, Brave, etc.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::types::{BoostedSearchSnippet, SerpQuery, Url};
use crate::utils::ActionTimer;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
    /// Tempo de leitura em milissegundos (opcional)
    pub read_time_ms: Option<u128>,
    /// Fonte da leitura (jina, rust_local, etc.)
    pub source: Option<String>,
}

/// Resultado de leitura comparativa entre Jina e Rust local
#[derive(Debug, Clone)]
pub struct ComparativeReadResult {
    /// URL que foi lida
    pub url: String,
    /// Resultado da leitura via Jina API
    pub jina_result: Option<UrlContent>,
    /// Resultado da leitura via Rust local
    pub rust_result: Option<UrlContent>,
    /// Tempo de leitura via Jina (ms)
    pub jina_time_ms: u128,
    /// Tempo de leitura via Rust local (ms)
    pub rust_time_ms: u128,
    /// Diferença de velocidade (positivo = Rust mais rápido)
    pub speed_diff_ms: i128,
    /// Qual método foi mais rápido
    pub faster: ReadMethod,
}

/// Método de leitura
#[derive(Debug, Clone, PartialEq)]
pub enum ReadMethod {
    Jina,
    RustLocal,
    Tie,
}

/// Trait principal para clientes de busca
///
/// Define a interface para busca web e leitura de conteúdo.
#[async_trait]
pub trait SearchClient: Send + Sync {
    /// Executa uma única busca
    async fn search(&self, query: &SerpQuery) -> Result<SearchResult, SearchError>;

    /// Executa múltiplas buscas em paralelo
    async fn search_batch(&self, queries: &[SerpQuery]) -> Vec<Result<SearchResult, SearchError>>;

    /// Lê o conteúdo de uma URL
    async fn read_url(&self, url: &Url) -> Result<UrlContent, SearchError>;

    /// Lê múltiplas URLs em paralelo
    async fn read_urls_batch(&self, urls: &[Url]) -> Vec<Result<UrlContent, SearchError>>;

    /// Rerank de URLs usando embeddings
    async fn rerank(&self, query: &str, urls: &[BoostedSearchSnippet])
        -> Vec<BoostedSearchSnippet>;

    /// Lê uma URL comparando Jina API vs Rust local em paralelo
    async fn read_url_comparative(&self, url: &Url) -> ComparativeReadResult;

    /// Lê múltiplas URLs com comparação em paralelo
    async fn read_urls_comparative_batch(&self, urls: &[Url]) -> Vec<ComparativeReadResult>;
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
        Self {
            mock_results: None,
            mock_content: None,
        }
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

    async fn search_batch(&self, queries: &[SerpQuery]) -> Vec<Result<SearchResult, SearchError>> {
        queries
            .iter()
            .map(|q| self.search(q))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|_| {
                Ok(SearchResult {
                    urls: vec![],
                    snippets: vec!["Mock batch snippet".into()],
                    total_results: 0,
                })
            })
            .collect()
    }

    async fn read_url(&self, url: &Url) -> Result<UrlContent, SearchError> {
        Ok(self.mock_content.clone().unwrap_or_else(|| UrlContent {
            title: "Mock Title".into(),
            text: "Mock content from URL".into(),
            url: url.clone(),
            word_count: 4,
            read_time_ms: Some(100),
            source: Some("mock".into()),
        }))
    }

    async fn read_urls_batch(&self, urls: &[Url]) -> Vec<Result<UrlContent, SearchError>> {
        urls.iter()
            .map(|url| {
                Ok(UrlContent {
                    title: "Mock Title".into(),
                    text: "Mock content".into(),
                    url: url.clone(),
                    word_count: 2,
                    read_time_ms: Some(50),
                    source: Some("mock".into()),
                })
            })
            .collect()
    }

    async fn rerank(
        &self,
        _query: &str,
        urls: &[BoostedSearchSnippet],
    ) -> Vec<BoostedSearchSnippet> {
        urls.to_vec()
    }

    async fn read_url_comparative(&self, url: &Url) -> ComparativeReadResult {
        ComparativeReadResult {
            url: url.clone(),
            jina_result: Some(UrlContent {
                title: "Mock Jina".into(),
                text: "Mock jina content".into(),
                url: url.clone(),
                word_count: 3,
                read_time_ms: Some(100),
                source: Some("jina".into()),
            }),
            rust_result: Some(UrlContent {
                title: "Mock Rust".into(),
                text: "Mock rust content".into(),
                url: url.clone(),
                word_count: 3,
                read_time_ms: Some(80),
                source: Some("rust_local".into()),
            }),
            jina_time_ms: 100,
            rust_time_ms: 80,
            speed_diff_ms: 20,
            faster: ReadMethod::RustLocal,
        }
    }

    async fn read_urls_comparative_batch(&self, urls: &[Url]) -> Vec<ComparativeReadResult> {
        urls.iter()
            .map(|url| futures::executor::block_on(self.read_url_comparative(url)))
            .collect()
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
    /// - Busca: `https://svip.jina.ai/` (POST com JSON body)
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
            search_endpoint: "https://svip.jina.ai/".into(),
            reader_endpoint: "https://r.jina.ai".into(),
            rerank_endpoint: "https://api.jina.ai/v1/rerank".into(),
            client: reqwest::Client::new(),
        }
    }
}

// Estruturas para serialização/deserialização da API Jina

/// Request body para busca Jina
#[derive(Serialize)]
struct JinaSearchRequest {
    q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tbs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct JinaSearchResponse {
    results: Vec<JinaSearchResult>,
    meta: JinaSearchMeta,
}

#[derive(Deserialize, Debug)]
struct JinaSearchMeta {
    query: String,
    num_results: u64,
    #[serde(default)]
    latency: f64,
    #[serde(default)]
    credits: u64,
}

#[derive(Deserialize, Debug)]
struct JinaSearchResult {
    title: String,
    url: String,
    snippet: String,
}

/// Request body para leitura de URL Jina
#[derive(Serialize)]
struct JinaReaderRequest {
    url: String,
}

#[derive(Deserialize, Debug)]
struct JinaReaderResponse {
    code: i32,
    status: i32,
    data: Option<JinaReaderData>,
}

#[derive(Deserialize, Debug)]
struct JinaReaderData {
    title: String,
    #[serde(default)]
    description: String,
    url: String,
    content: String,
    #[serde(default)]
    usage: Option<JinaReaderUsage>,
}

#[derive(Deserialize, Debug)]
struct JinaReaderUsage {
    #[serde(default)]
    tokens: u64,
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
        // Construir request body JSON (igual ao TypeScript)
        let request_body = JinaSearchRequest {
            q: query.q.clone(),
            tbs: query.tbs.clone(),
            location: query.location.clone(),
            num: Some(10), // número padrão de resultados
        };

        log::info!("🔍 Jina Search: q={}", query.q);

        let response = self
            .client
            .post(&self.search_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                log::error!("❌ Jina network error: {}", e);
                SearchError::NetworkError(e.to_string())
            })?;

        log::info!("📡 Jina response status: {}", response.status());

        if response.status() == 429 {
            log::warn!("⚠️ Jina rate limit exceeded");
            return Err(SearchError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("❌ Jina API error: {}", error_text);
            return Err(SearchError::ApiError(format!(
                "Jina Search API error: {}",
                error_text
            )));
        }

        let search_response: JinaSearchResponse = response.json().await.map_err(|e| {
            log::error!("❌ Failed to parse Jina response: {}", e);
            SearchError::ParseError(format!("Failed to parse search response: {}", e))
        })?;

        // Usar metadados da resposta para logging detalhado
        log::info!(
            "✅ Jina Search: query='{}' | {} resultados | latency={:.2}ms | credits={}",
            search_response.meta.query,
            search_response.meta.num_results,
            search_response.meta.latency,
            search_response.meta.credits
        );

        let mut snippets: Vec<BoostedSearchSnippet> = Vec::new();
        let mut snippet_strings: Vec<String> = Vec::new();

        for r in search_response.results {
            let hostname = extract_hostname(&r.url);
            let hostname_boost_val = hostname.as_ref().map(|h| hostname_boost(h)).unwrap_or(1.0);
            let path_boost_val = path_boost(&r.url);

            snippets.push(BoostedSearchSnippet {
                url: r.url.clone(),
                title: r.title,
                description: r.snippet.clone(),
                weight: 1.0,
                freq_boost: 1.0,
                hostname_boost: hostname_boost_val,
                path_boost: path_boost_val,
                jina_rerank_boost: 1.0,
                final_score: 1.0,
                score: 1.0,
                merged: r.snippet.clone(),
            });

            snippet_strings.push(r.snippet);
        }

        Ok(SearchResult {
            urls: snippets,
            snippets: snippet_strings,
            total_results: search_response.meta.num_results,
        })
    }

    async fn search_batch(&self, queries: &[SerpQuery]) -> Vec<Result<SearchResult, SearchError>> {
        use futures::future::join_all;

        let futures: Vec<_> = queries.iter().map(|q| self.search(q)).collect();

        join_all(futures).await
    }

    async fn read_url(&self, url: &Url) -> Result<UrlContent, SearchError> {
        // Validar URL
        url::Url::parse(url).map_err(|e| SearchError::InvalidUrl(format!("Invalid URL: {}", e)))?;

        log::info!("📖 Jina Reader: {}", url);

        // Usar POST com JSON body (igual ao TypeScript)
        let request_body = JinaReaderRequest { url: url.clone() };

        let response = self
            .client
            .post(&self.reader_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("X-Md-Link-Style", "discarded")
            .header("X-Retain-Images", "none")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| {
                log::error!("❌ Jina Reader network error: {}", e);
                SearchError::NetworkError(e.to_string())
            })?;

        log::info!("📡 Jina Reader response status: {}", response.status());

        if response.status() == 429 {
            log::warn!("⚠️ Jina Reader rate limit exceeded");
            return Err(SearchError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("❌ Jina Reader API error: {}", error_text);
            return Err(SearchError::FetchError(format!(
                "Jina Reader API error: {}",
                error_text
            )));
        }

        let reader_response: JinaReaderResponse = response.json().await.map_err(|e| {
            log::error!("❌ Failed to parse Jina Reader response: {}", e);
            SearchError::ExtractionError(format!("Failed to parse reader response: {}", e))
        })?;

        // Verificar código de resposta da API
        if reader_response.code != 200 {
            log::error!(
                "❌ Jina Reader API returned code={} status={}",
                reader_response.code,
                reader_response.status
            );
            return Err(SearchError::ApiError(format!(
                "Jina Reader returned code {} status {}",
                reader_response.code, reader_response.status
            )));
        }

        let data = reader_response
            .data
            .ok_or_else(|| SearchError::ExtractionError("No data in response".into()))?;

        let word_count = data.content.split_whitespace().count();
        let tokens_used = data.usage.as_ref().map(|u| u.tokens).unwrap_or(0);

        // Usar todos os campos disponíveis no log
        log::info!(
            "✅ Jina Reader: '{}' | url={} | {} palavras | {} tokens | desc='{}'",
            data.title,
            data.url,
            word_count,
            tokens_used,
            if data.description.len() > 50 {
                format!("{}...", &data.description[..50])
            } else {
                data.description.clone()
            }
        );

        Ok(UrlContent {
            title: data.title,
            text: data.content,
            url: data.url, // Usar URL retornada pela API (pode ter sido normalizada)
            word_count,
            read_time_ms: None, // Será preenchido pelo chamador se necessário
            source: Some("jina".to_string()),
        })
    }

    async fn read_urls_batch(&self, urls: &[Url]) -> Vec<Result<UrlContent, SearchError>> {
        use futures::future::join_all;

        let futures: Vec<_> = urls.iter().map(|url| self.read_url(url)).collect();

        join_all(futures).await
    }

    async fn rerank(
        &self,
        query: &str,
        urls: &[BoostedSearchSnippet],
    ) -> Vec<BoostedSearchSnippet> {
        if urls.is_empty() {
            return vec![];
        }

        let documents: Vec<String> = urls
            .iter()
            .map(|snippet| format!("{} {}", snippet.title, snippet.description))
            .collect();

        let request = JinaRerankRequest {
            model: "jina-reranker-v1-base-en".into(),
            query: query.to_string(),
            documents,
            top_n: Some(urls.len()),
        };

        let response = match self
            .client
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
        let mut reranked: Vec<BoostedSearchSnippet> = urls
            .iter()
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
        reranked.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        reranked
    }

    /// Lê uma URL comparando Jina API vs Rust local em paralelo
    async fn read_url_comparative(&self, url: &Url) -> ComparativeReadResult {
        use crate::utils::FileReader;

        log::info!("🔄 Leitura comparativa: {}", url);

        // Executar ambas as leituras em paralelo
        let jina_future = async {
            let timer = ActionTimer::start("Jina Reader");
            let result = self.read_url(url).await;
            let elapsed = timer.stop();
            (result, elapsed)
        };

        let rust_future = async {
            let timer = ActionTimer::start("Rust Local");
            let reader = FileReader::new();
            let result = reader.read_url(url).await;
            let elapsed = timer.stop();
            (result, elapsed)
        };

        let ((jina_result, jina_time), (rust_result, rust_time)) =
            futures::join!(jina_future, rust_future);

        // Converter resultado Rust local para UrlContent
        let rust_content = rust_result.ok().map(|fc| UrlContent {
            title: fc.title.unwrap_or_default(),
            text: fc.text,
            url: fc.source,
            word_count: fc.word_count,
            read_time_ms: Some(rust_time),
            source: Some("rust_local".to_string()),
        });

        // Adicionar tempo ao resultado Jina
        let jina_content = jina_result.ok().map(|mut uc| {
            uc.read_time_ms = Some(jina_time);
            uc
        });

        let speed_diff = jina_time as i128 - rust_time as i128;
        let faster = if speed_diff > 100 {
            ReadMethod::RustLocal
        } else if speed_diff < -100 {
            ReadMethod::Jina
        } else {
            ReadMethod::Tie
        };

        // Log comparativo
        log::info!(
            "📊 Comparação {} | Jina: {}ms | Rust: {}ms | Diff: {}ms | Mais rápido: {:?}",
            url,
            jina_time,
            rust_time,
            speed_diff,
            faster
        );

        ComparativeReadResult {
            url: url.clone(),
            jina_result: jina_content,
            rust_result: rust_content,
            jina_time_ms: jina_time,
            rust_time_ms: rust_time,
            speed_diff_ms: speed_diff,
            faster,
        }
    }

    /// Lê múltiplas URLs com comparação em paralelo
    async fn read_urls_comparative_batch(&self, urls: &[Url]) -> Vec<ComparativeReadResult> {
        use futures::future::join_all;

        log::info!("🔄 Leitura comparativa em batch: {} URLs", urls.len());

        let futures: Vec<_> = urls
            .iter()
            .map(|url| self.read_url_comparative(url))
            .collect();

        let results = join_all(futures).await;

        // Estatísticas agregadas
        let total_jina: u128 = results.iter().map(|r| r.jina_time_ms).sum();
        let total_rust: u128 = results.iter().map(|r| r.rust_time_ms).sum();
        let jina_wins = results
            .iter()
            .filter(|r| r.faster == ReadMethod::Jina)
            .count();
        let rust_wins = results
            .iter()
            .filter(|r| r.faster == ReadMethod::RustLocal)
            .count();

        log::info!(
            "📊 Batch Summary: Jina total={}ms ({} wins) | Rust total={}ms ({} wins)",
            total_jina,
            jina_wins,
            total_rust,
            rust_wins
        );

        results
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
