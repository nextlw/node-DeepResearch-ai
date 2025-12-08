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

/// Método de leitura utilizado para extração de conteúdo de URLs.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadMethod {
    /// Jina Reader API - extração especializada via API externa
    Jina,
    /// Rust local + processamento LLM - download direto e parsing local
    RustLocal,
    /// Empate - ambos métodos tiveram performance similar
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

    /// Lê uma URL com progresso compartilhado (Rust primeiro, Jina fallback)
    /// Retorna (Result, método_usado, tentativas, bytes_processados)
    async fn read_url_with_fallback_progress(
        &self,
        url: &Url,
        progress: std::sync::Arc<std::sync::atomic::AtomicU8>,
    ) -> (Result<UrlContent, SearchError>, &'static str, u8, usize);
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

    async fn read_url_with_fallback_progress(
        &self,
        url: &Url,
        progress: std::sync::Arc<std::sync::atomic::AtomicU8>,
    ) -> (Result<UrlContent, SearchError>, &'static str, u8, usize) {
        use std::sync::atomic::Ordering;
        progress.store(50, Ordering::Relaxed);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        progress.store(100, Ordering::Relaxed);

        let content = UrlContent {
            title: "Mock Title".into(),
            text: "Mock content from URL".into(),
            url: url.clone(),
            word_count: 4,
            read_time_ms: Some(100),
            source: Some("mock".into()),
        };
        (Ok(content), "mock", 1, 21)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO JINA (STUB)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Preferência de método para leitura de URLs.
///
/// Re-exportado de `crate::config::WebReaderPreference` para conveniência.
pub use crate::config::WebReaderPreference;

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
    /// Endpoint para embeddings
    embeddings_endpoint: String,
    /// Modelo de embeddings (jina-embeddings-v3)
    embeddings_model: String,
    /// Cliente HTTP
    client: reqwest::Client,
    /// Preferência de método de leitura de URLs
    webreader_preference: WebReaderPreference,
}

/// Resultado de embedding Jina
#[derive(Debug, Clone)]
pub struct JinaEmbeddingResult {
    /// Vetor de embedding
    pub vector: Vec<f32>,
    /// Tokens usados
    pub tokens_used: u64,
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
    /// - Embeddings: `https://api.jina.ai/v1/embeddings` (jina-embeddings-v3)
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = JinaClient::new("jina_api_key".into());
    /// let results = client.search(&query).await?;
    /// ```
    pub fn new(api_key: String) -> Self {
        Self::with_preference(api_key, WebReaderPreference::default())
    }

    /// Cria um novo cliente Jina AI com preferência de WebReader.
    ///
    /// # Argumentos
    /// * `api_key` - Sua chave de API Jina AI
    /// * `webreader_preference` - Preferência de método de leitura (Jina, Rust, Compare)
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = JinaClient::with_preference(
    ///     "jina_api_key".into(),
    ///     WebReaderPreference::RustOnly,
    /// );
    /// ```
    pub fn with_preference(api_key: String, webreader_preference: WebReaderPreference) -> Self {
        log::info!("🔧 JinaClient: WebReader preference = {}", webreader_preference);
        Self {
            api_key,
            search_endpoint: "https://svip.jina.ai/".into(),
            reader_endpoint: "https://r.jina.ai".into(),
            rerank_endpoint: "https://api.jina.ai/v1/rerank".into(),
            embeddings_endpoint: "https://api.jina.ai/v1/embeddings".into(),
            embeddings_model: "jina-embeddings-v4".into(),
            client: reqwest::Client::new(),
            webreader_preference,
        }
    }

    /// Retorna a preferência de WebReader configurada.
    pub fn webreader_preference(&self) -> WebReaderPreference {
        self.webreader_preference
    }

    /// Gera embeddings para um único texto usando Jina Embeddings v3
    pub async fn embed(&self, text: &str) -> Result<JinaEmbeddingResult, SearchError> {
        self.embed_batch(&[text.to_string()])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| SearchError::ParseError("No embedding returned".into()))
    }

    /// Gera embeddings em batch usando Jina Embeddings v4
    ///
    /// Jina v4 suporta até 32,768 tokens por input e dimensões de 2048 (single-vector)
    /// É multimodal (texto e imagem) e multilíngue (30+ idiomas)
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<JinaEmbeddingResult>, SearchError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Formatar input conforme API v4 - aceita objetos com "text"
        let input: Vec<serde_json::Value> = texts
            .iter()
            .map(|t| serde_json::json!({"text": t}))
            .collect();

        let request_body = serde_json::json!({
            "model": self.embeddings_model,
            "task": "text-matching",
            "normalized": true,
            "embedding_type": "float",
            "input": input
        });

        log::debug!("🔢 Jina Embeddings v4: {} textos", texts.len());

        let response = self
            .client
            .post(&self.embeddings_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SearchError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SearchError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SearchError::ApiError(format!(
                "Jina Embeddings API error: {}",
                error_text
            )));
        }

        let embedding_response: JinaEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| SearchError::ParseError(format!("Failed to parse Jina embeddings: {}", e)))?;

        let total_tokens = embedding_response.usage.total_tokens;
        let prompt_tokens = embedding_response.usage.prompt_tokens;
        let tokens_per_embedding = prompt_tokens / texts.len().max(1) as u64;

        // Ordenar por index para garantir ordem correta
        let mut data = embedding_response.data;
        data.sort_by_key(|d| d.index);

        let results: Vec<JinaEmbeddingResult> = data
            .into_iter()
            .map(|d| JinaEmbeddingResult {
                vector: d.embedding,
                tokens_used: tokens_per_embedding,
            })
            .collect();

        log::info!(
            "✅ Jina Embeddings v4 ({}): {} vetores | dim={} | {} prompt + {} total tokens",
            embedding_response.model,
            results.len(),
            results.first().map(|r| r.vector.len()).unwrap_or(0),
            prompt_tokens,
            total_tokens
        );

        Ok(results)
    }

    /// Lê uma URL com Jina usando streaming real e callback de progresso
    ///
    /// O callback recebe: (bytes_recebidos, progresso_estimado 0-100)
    pub async fn read_url_with_progress<F>(
        &self,
        url: &Url,
        mut progress_callback: F,
    ) -> Result<UrlContent, SearchError>
    where
        F: FnMut(usize, u8) + Send,
    {
        use futures::StreamExt;

        // Validar URL
        url::Url::parse(url).map_err(|e| SearchError::InvalidUrl(format!("Invalid URL: {}", e)))?;

        log::info!("📖 Jina Reader (streaming com progresso): {}", url);

        // Formato GET com streaming SSE
        // Formato oficial: https://r.jina.ai/https://example.com (SEM encoding)
        let reader_url = format!("{}/{}", self.reader_endpoint, url);

        let response = self
            .client
            .get(&reader_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "text/event-stream")
            .header("X-Return-Format", "markdown")
            .header("X-Md-Link-Style", "discarded")
            .header("X-Retain-Images", "none")
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| {
                log::error!("❌ Jina Reader network error: {}", e);
                SearchError::NetworkError(e.to_string())
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(SearchError::RateLimitError);
        }
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SearchError::FetchError(format!("Jina API error: {}", error_text)));
        }

        // Processar stream com progresso
        let mut content = String::new();
        let mut title = String::new();
        let mut total_bytes: usize = 0;

        // Estimar tamanho total (páginas típicas ~50-200KB de markdown)
        let estimated_total: usize = 100_000; // 100KB como estimativa

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    total_bytes += chunk.len();

                    // Calcular progresso (cap em 95% até finalizar)
                    let progress = ((total_bytes as f64 / estimated_total as f64) * 95.0)
                        .min(95.0) as u8;

                    // Emitir progresso
                    progress_callback(total_bytes, progress);

                    // Processar chunk
                    let text = String::from_utf8_lossy(&chunk);
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data.starts_with('{') {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(t) = json.get("title").and_then(|v| v.as_str()) {
                                        title = t.to_string();
                                    }
                                    if let Some(c) = json.get("content").and_then(|v| v.as_str()) {
                                        content = c.to_string();
                                    }
                                    if let Some(nested) = json.get("data") {
                                        if let Some(t) = nested.get("title").and_then(|v| v.as_str()) {
                                            title = t.to_string();
                                        }
                                        if let Some(c) = nested.get("content").and_then(|v| v.as_str()) {
                                            content = c.to_string();
                                        }
                                    }
                                }
                            } else if !data.is_empty() && data != "[DONE]" {
                                content.push_str(data);
                                content.push('\n');
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("⚠️ Chunk error: {}", e);
                }
            }
        }

        // Progresso final 100%
        progress_callback(total_bytes, 100);

        // Fallback se não parseou SSE
        if content.is_empty() {
            return Err(SearchError::ExtractionError("No content received".into()));
        }

        let word_count = content.split_whitespace().count();

        log::info!(
            "✅ Jina Streaming: '{}' | {} bytes | {} palavras",
            if title.len() > 30 { format!("{}...", &title[..30]) } else { title.clone() },
            total_bytes,
            word_count
        );

        Ok(UrlContent {
            title,
            text: content,
            url: url.clone(),
            word_count,
            read_time_ms: None,
            source: Some("jina".to_string()),
        })
    }

    /// Lê uma URL usando o método configurado por WebReaderPreference.
    ///
    /// - `JinaOnly`: Usa apenas Jina Reader API
    /// - `RustOnly`: Usa apenas Rust local + Readability
    /// - `Compare`: Tenta Rust primeiro, Jina como fallback (padrão)
    ///
    /// Retorna (Result, método_usado, tentativas)
    async fn read_url_with_fallback(
        &self,
        url: &Url,
    ) -> (Result<UrlContent, SearchError>, &'static str, u8) {
        use crate::utils::FileReader;

        const MIN_CONTENT_LENGTH: usize = 100;

        match self.webreader_preference {
            // Usar apenas Jina - sem fallback
            WebReaderPreference::JinaOnly => {
                log::debug!("📖 [JINA-ONLY] Lendo: {}", url);
                let jina_start = std::time::Instant::now();
                let jina_result = self.read_url(url).await;
                let jina_time = jina_start.elapsed().as_millis();

                match jina_result {
                    Ok(mut content) => {
                        if content.text.len() >= MIN_CONTENT_LENGTH {
                            content.read_time_ms = Some(jina_time);
                            log::info!("✅ [JINA-ONLY] {} | {}ms | {} bytes", url, jina_time, content.text.len());
                            (Ok(content), "jina", 1)
                        } else {
                            log::error!("❌ [JINA-ONLY] {} | conteúdo insuficiente (<{} bytes)", url, MIN_CONTENT_LENGTH);
                            (
                                Err(SearchError::ExtractionError(format!(
                                    "Conteúdo muito curto para {}",
                                    url
                                ))),
                                "failed",
                                1,
                            )
                        }
                    }
                    Err(e) => {
                        log::error!("❌ [JINA-ONLY] {} | Erro: {}", url, e);
                        (Err(e), "failed", 1)
                    }
                }
            }

            // Usar apenas Rust local - sem fallback
            WebReaderPreference::RustOnly => {
                log::debug!("📖 [RUST-ONLY] Lendo: {}", url);
                let reader = FileReader::new();
                let rust_start = std::time::Instant::now();
                let rust_result = reader.read_url(url).await;
                let rust_time = rust_start.elapsed().as_millis();

                match rust_result {
                    Ok(file_content) => {
                        if file_content.text.len() >= MIN_CONTENT_LENGTH {
                            log::info!(
                                "✅ [RUST-ONLY] {} | {}ms | {} bytes | {} palavras",
                                url, rust_time, file_content.text.len(), file_content.word_count
                            );
                            (
                                Ok(UrlContent {
                                    title: file_content.title.unwrap_or_default(),
                                    text: file_content.text,
                                    url: file_content.source,
                                    word_count: file_content.word_count,
                                    read_time_ms: Some(rust_time),
                                    source: Some("rust_local".to_string()),
                                }),
                                "rust_local",
                                1,
                            )
                        } else {
                            log::error!("❌ [RUST-ONLY] {} | conteúdo insuficiente (<{} bytes)", url, MIN_CONTENT_LENGTH);
                            (
                                Err(SearchError::ExtractionError(format!(
                                    "Conteúdo muito curto para {}",
                                    url
                                ))),
                                "failed",
                                1,
                            )
                        }
                    }
                    Err(e) => {
                        log::error!("❌ [RUST-ONLY] {} | Erro: {}", url, e);
                        (Err(SearchError::FetchError(e.to_string())), "failed", 1)
                    }
                }
            }

            // Comportamento padrão: Rust primeiro, Jina como fallback
            WebReaderPreference::Compare => {
                log::debug!("📖 [COMPARE] Lendo (Rust→Jina): {}", url);

                // 1. Tentar Rust local primeiro
                let reader = FileReader::new();
                let rust_start = std::time::Instant::now();
                let rust_result = reader.read_url(url).await;
                let rust_time = rust_start.elapsed().as_millis();

                if let Ok(file_content) = rust_result {
                    if file_content.text.len() >= MIN_CONTENT_LENGTH {
                        log::info!(
                            "✅ [RUST+Readability] {} | {}ms | {} bytes | {} palavras",
                            url, rust_time, file_content.text.len(), file_content.word_count
                        );
                        return (
                            Ok(UrlContent {
                                title: file_content.title.unwrap_or_default(),
                                text: file_content.text,
                                url: file_content.source,
                                word_count: file_content.word_count,
                                read_time_ms: Some(rust_time),
                                source: Some("rust_local".to_string()),
                            }),
                            "rust_local",
                            1,
                        );
                    }
                    log::warn!("⚠️ [RUST] {} conteúdo curto ({} bytes)", url, file_content.text.len());
                } else if let Err(ref e) = rust_result {
                    log::warn!("⚠️ [RUST] {} falhou ({}ms): {}", url, rust_time, e);
                }

                // 2. Fallback para Jina
                let jina_start = std::time::Instant::now();
                let jina_result = self.read_url(url).await;
                let jina_time = jina_start.elapsed().as_millis();

                match jina_result {
                    Ok(mut content) => {
                        if content.text.len() >= MIN_CONTENT_LENGTH {
                            content.read_time_ms = Some(rust_time + jina_time);
                            log::info!("✅ [JINA-FALLBACK] {} | {}ms | {} bytes", url, jina_time, content.text.len());
                            (Ok(content), "jina", 2)
                        } else {
                            log::error!("❌ [AMBOS FALHARAM] {} | conteúdo insuficiente (<{} bytes)", url, MIN_CONTENT_LENGTH);
                            (
                                Err(SearchError::ExtractionError(format!(
                                    "Conteúdo muito curto em ambos os métodos para {}",
                                    url
                                ))),
                                "failed",
                                2,
                            )
                        }
                    }
                    Err(e) => {
                        log::error!("❌ [AMBOS FALHARAM] {} | Rust e Jina: {}", url, e);
                        (Err(e), "failed", 2)
                    }
                }
            }
        }
    }
}

// Estruturas para serialização/deserialização da API Jina

/// Response de embeddings Jina v4
#[derive(Deserialize, Debug)]
struct JinaEmbeddingResponse {
    data: Vec<JinaEmbeddingData>,
    usage: JinaEmbeddingUsage,
    model: String,
}

#[derive(Deserialize, Debug)]
struct JinaEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize, Debug)]
struct JinaEmbeddingUsage {
    total_tokens: u64,
    prompt_tokens: u64,
}

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

// Structs para Jina Reader removidas - agora usamos streaming SSE via GET

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

        log::info!("📖 Jina Reader (streaming): {}", url);

        // Usar GET com streaming para receber chunks progressivos
        // Formato oficial: https://r.jina.ai/https://example.com (SEM encoding)
        let reader_url = format!("{}/{}", self.reader_endpoint, url);

        let response = self
            .client
            .get(&reader_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "text/event-stream") // Habilitar streaming SSE
            .header("X-Return-Format", "markdown")
            .header("X-Md-Link-Style", "discarded")
            .header("X-Retain-Images", "none")
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| {
                log::error!("❌ Jina Reader network error: {}", e);
                SearchError::NetworkError(e.to_string())
            })?;

        let status = response.status();
        log::info!("📡 Jina Reader response status: {}", status);

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            log::warn!("⚠️ Jina Reader rate limit exceeded");
            return Err(SearchError::RateLimitError);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("❌ Jina Reader API error: {}", error_text);
            return Err(SearchError::FetchError(format!(
                "Jina Reader API error: {}",
                error_text
            )));
        }

        // Processar streaming SSE - cada chunk contém mais conteúdo
        let mut content = String::new();
        let mut title = String::new();

        // Ler bytes como stream
        let bytes = response.bytes().await.map_err(|e| {
            log::error!("❌ Jina Reader stream error: {}", e);
            SearchError::NetworkError(e.to_string())
        })?;

        let bytes_received = bytes.len();
        let raw_text = String::from_utf8_lossy(&bytes);

        // Processar SSE events - formato: "data: {content}\n\n"
        for line in raw_text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..]; // Remover "data: "

                // Tentar parsear como JSON (último evento contém JSON completo)
                if data.starts_with('{') {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(t) = json.get("title").and_then(|v| v.as_str()) {
                            title = t.to_string();
                        }
                        if let Some(c) = json.get("content").and_then(|v| v.as_str()) {
                            content = c.to_string();
                        }
                        // Se tem 'data' aninhado (formato da API)
                        if let Some(nested) = json.get("data") {
                            if let Some(t) = nested.get("title").and_then(|v| v.as_str()) {
                                title = t.to_string();
                            }
                            if let Some(c) = nested.get("content").and_then(|v| v.as_str()) {
                                content = c.to_string();
                            }
                        }
                    }
                } else if !data.is_empty() && data != "[DONE]" {
                    // Chunk de texto incremental
                    content.push_str(data);
                    content.push('\n');
                }
            }
        }

        // Se não conseguiu parsear SSE, usar conteúdo raw como markdown
        if content.is_empty() {
            content = raw_text.to_string();
            // Tentar extrair título do início do markdown
            if let Some(first_line) = content.lines().next() {
                if first_line.starts_with("# ") {
                    title = first_line[2..].trim().to_string();
                }
            }
        }

        let word_count = content.split_whitespace().count();

        log::info!(
            "✅ Jina Reader: '{}' | {} bytes | {} palavras",
            if title.len() > 40 {
                format!("{}...", &title[..40])
            } else {
                title.clone()
            },
            bytes_received,
            word_count
        );

        Ok(UrlContent {
            title,
            text: content,
            url: url.clone(),
            word_count,
            read_time_ms: None,
            source: Some("jina".to_string()),
        })
    }

    async fn read_urls_batch(&self, urls: &[Url]) -> Vec<Result<UrlContent, SearchError>> {
        use futures::future::join_all;
        use std::time::Instant;

        let start = Instant::now();
        log::info!(
            "⚡ [PARALELO] Iniciando {} leituras (Rust primeiro, Jina fallback)...",
            urls.len()
        );

        // Usar o novo método com fallback
        let futures: Vec<_> = urls
            .iter()
            .map(|url| self.read_url_with_fallback(url))
            .collect();
        let results_with_meta = join_all(futures).await;

        let elapsed = start.elapsed().as_millis();
        let avg_per_url = elapsed as f64 / urls.len().max(1) as f64;

        // Contar métodos usados
        let rust_count = results_with_meta.iter().filter(|(_, m, _)| *m == "rust_local").count();
        let jina_count = results_with_meta.iter().filter(|(_, m, _)| *m == "jina").count();
        let failed_count = results_with_meta.iter().filter(|(_, m, _)| *m == "failed").count();

        log::info!(
            "⚡ [PARALELO] {} URLs em {}ms (média: {:.0}ms) | Rust: {} | Jina: {} | Falhas: {}",
            urls.len(),
            elapsed,
            avg_per_url,
            rust_count,
            jina_count,
            failed_count
        );

        // Extrair apenas os resultados
        results_with_meta.into_iter().map(|(r, _, _)| r).collect()
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

    async fn read_url_with_fallback_progress(
        &self,
        url: &Url,
        progress: std::sync::Arc<std::sync::atomic::AtomicU8>,
    ) -> (Result<UrlContent, SearchError>, &'static str, u8, usize) {
        use crate::utils::FileReader;
        use std::sync::atomic::Ordering;

        const MIN_CONTENT_LENGTH: usize = 100;

        match self.webreader_preference {
            // Usar apenas Jina - sem fallback
            WebReaderPreference::JinaOnly => {
                progress.store(10, Ordering::Relaxed);
                log::debug!("📖 [JINA-ONLY] Lendo com progresso: {}", url);

                let jina_start = std::time::Instant::now();
                progress.store(30, Ordering::Relaxed);

                let jina_result = self.read_url(url).await;
                let jina_time = jina_start.elapsed().as_millis();

                progress.store(90, Ordering::Relaxed);

                match jina_result {
                    Ok(mut content) => {
                        if content.text.len() >= MIN_CONTENT_LENGTH {
                            content.read_time_ms = Some(jina_time);
                            progress.store(100, Ordering::Relaxed);
                            log::info!("✅ [JINA-ONLY] {} | {}ms | {} bytes", url, jina_time, content.text.len());
                            let bytes = content.text.len();
                            (Ok(content), "jina", 1, bytes)
                        } else {
                            progress.store(100, Ordering::Relaxed);
                            log::error!("❌ [JINA-ONLY] {} | conteúdo insuficiente", url);
                            (
                                Err(SearchError::ExtractionError(format!(
                                    "Conteúdo muito curto para {}", url
                                ))),
                                "failed",
                                1,
                                0,
                            )
                        }
                    }
                    Err(e) => {
                        progress.store(100, Ordering::Relaxed);
                        log::error!("❌ [JINA-ONLY] {} | falha: {}", url, e);
                        (Err(e), "failed", 1, 0)
                    }
                }
            }

            // Usar apenas Rust local - sem fallback
            WebReaderPreference::RustOnly => {
                progress.store(10, Ordering::Relaxed);
                log::debug!("📖 [RUST-ONLY] Lendo com progresso: {}", url);

                let reader = FileReader::new();
                let rust_start = std::time::Instant::now();
                progress.store(30, Ordering::Relaxed);

                let rust_result = reader.read_url(url).await;
                let rust_time = rust_start.elapsed().as_millis();

                progress.store(90, Ordering::Relaxed);

                match rust_result {
                    Ok(file_content) => {
                        if file_content.text.len() >= MIN_CONTENT_LENGTH {
                            progress.store(100, Ordering::Relaxed);
                            log::info!(
                                "✅ [RUST-ONLY] {} | {}ms | {} bytes",
                                url, rust_time, file_content.text.len()
                            );
                            let bytes = file_content.text.len();
                            (
                                Ok(UrlContent {
                                    title: file_content.title.unwrap_or_default(),
                                    text: file_content.text,
                                    url: file_content.source,
                                    word_count: file_content.word_count,
                                    read_time_ms: Some(rust_time),
                                    source: Some("rust_local".to_string()),
                                }),
                                "rust_local",
                                1,
                                bytes,
                            )
                        } else {
                            progress.store(100, Ordering::Relaxed);
                            log::error!("❌ [RUST-ONLY] {} | conteúdo insuficiente", url);
                            (
                                Err(SearchError::ExtractionError(format!(
                                    "Conteúdo muito curto para {}", url
                                ))),
                                "failed",
                                1,
                                0,
                            )
                        }
                    }
                    Err(e) => {
                        progress.store(100, Ordering::Relaxed);
                        log::error!("❌ [RUST-ONLY] {} | falha: {}", url, e);
                        (Err(SearchError::FetchError(e.to_string())), "failed", 1, 0)
                    }
                }
            }

            // Comportamento padrão: Rust primeiro, Jina como fallback
            WebReaderPreference::Compare => {
                // Fase 1: Rust local (0-50%)
                progress.store(5, Ordering::Relaxed);

                let reader = FileReader::new();
                let rust_start = std::time::Instant::now();

                progress.store(15, Ordering::Relaxed);
                let rust_result = reader.read_url(url).await;
                let rust_time = rust_start.elapsed().as_millis();

                progress.store(45, Ordering::Relaxed);

                if let Ok(file_content) = rust_result {
                    if file_content.text.len() >= MIN_CONTENT_LENGTH {
                        progress.store(100, Ordering::Relaxed);
                        log::info!(
                            "✅ [RUST+Readability] {} | {}ms | {} bytes",
                            url, rust_time, file_content.text.len()
                        );
                        let bytes = file_content.text.len();
                        return (
                            Ok(UrlContent {
                                title: file_content.title.unwrap_or_default(),
                                text: file_content.text,
                                url: file_content.source,
                                word_count: file_content.word_count,
                                read_time_ms: Some(rust_time),
                                source: Some("rust_local".to_string()),
                            }),
                            "rust_local",
                            1,
                            bytes,
                        );
                    }
                    log::warn!("⚠️ [RUST] {} conteúdo curto ({} bytes)", url, file_content.text.len());
                } else if let Err(ref e) = rust_result {
                    log::warn!("⚠️ [RUST] {} falhou: {}", url, e);
                }

                // Fase 2: Jina fallback (50-100%)
                progress.store(55, Ordering::Relaxed);

                let jina_start = std::time::Instant::now();
                progress.store(65, Ordering::Relaxed);

                let jina_result = self.read_url(url).await;
                let jina_time = jina_start.elapsed().as_millis();

                progress.store(90, Ordering::Relaxed);

                match jina_result {
                    Ok(mut content) => {
                        if content.text.len() >= MIN_CONTENT_LENGTH {
                            content.read_time_ms = Some(rust_time + jina_time);
                            progress.store(100, Ordering::Relaxed);
                            log::info!("✅ [JINA-FALLBACK] {} | {}ms | {} bytes", url, jina_time, content.text.len());
                            let bytes = content.text.len();
                            (Ok(content), "jina", 2, bytes)
                        } else {
                            progress.store(100, Ordering::Relaxed);
                            log::error!("❌ [AMBOS] {} | conteúdo insuficiente", url);
                            (
                                Err(SearchError::ExtractionError(format!(
                                    "Conteúdo muito curto em ambos os métodos para {}", url
                                ))),
                                "failed",
                                2,
                                0,
                            )
                        }
                    }
                    Err(e) => {
                        progress.store(100, Ordering::Relaxed);
                        log::error!("❌ [AMBOS] {} | falha: {}", url, e);
                        (Err(e), "failed", 2, 0)
                    }
                }
            }
        }
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
