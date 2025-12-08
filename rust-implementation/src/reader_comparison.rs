// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// READER COMPARISON - JINA vs RUST + OPENAI
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Compara diferentes métodos de leitura de URLs:
// 1. Jina Reader API - extração especializada
// 2. Rust local + OpenAI gpt-4o-mini - download HTML + processamento LLM
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Resultado de uma leitura comparativa entre Jina e Rust+OpenAI.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// URL que foi processada
    pub url: String,
    /// Resultado do Jina Reader
    pub jina: Option<ReaderResult>,
    /// Resultado do Rust + OpenAI
    pub rust_openai: Option<ReaderResult>,
    /// Qual foi mais rápido
    pub faster: String,
    /// Diferença de tempo em ms
    pub time_diff_ms: i128,
}

/// Resultado individual de um reader.
#[derive(Debug, Clone)]
pub struct ReaderResult {
    /// Título extraído da página
    pub title: String,
    /// Conteúdo textual extraído
    pub content: String,
    /// Contagem de palavras no conteúdo
    pub word_count: usize,
    /// Tempo de processamento em milissegundos
    pub time_ms: u128,
    /// Identificador do método usado (ex: "jina", "rust+openai")
    pub source: String,
    /// Mensagem de erro, se ocorreu falha
    pub error: Option<String>,
}

/// Cliente para comparação de readers
pub struct ReaderComparison {
    jina_api_key: String,
    openai_api_key: String,
    client: reqwest::Client,
}

impl ReaderComparison {
    /// Cria uma nova instância do comparador de readers.
    ///
    /// # Argumentos
    ///
    /// * `jina_api_key` - Chave da API Jina para extração de conteúdo
    /// * `openai_api_key` - Chave da API OpenAI para processamento LLM
    pub fn new(jina_api_key: String, openai_api_key: String) -> Self {
        Self {
            jina_api_key,
            openai_api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Compara leitura de uma URL usando Jina e Rust+OpenAI em paralelo
    pub async fn compare_read(&self, url: &str) -> ComparisonResult {
        log::info!("🔄 Comparando leitura: {}", url);

        // Executar ambas em paralelo
        let jina_future = self.read_with_jina(url);
        let openai_future = self.read_with_rust_openai(url);

        let (jina_result, openai_result) = futures::join!(jina_future, openai_future);

        let jina_time = jina_result.as_ref().map(|r| r.time_ms).unwrap_or(u128::MAX);
        let openai_time = openai_result
            .as_ref()
            .map(|r| r.time_ms)
            .unwrap_or(u128::MAX);

        let time_diff = jina_time as i128 - openai_time as i128;
        let faster = if time_diff > 0 {
            "rust_openai".to_string()
        } else if time_diff < 0 {
            "jina".to_string()
        } else {
            "tie".to_string()
        };

        log::info!(
            "📊 Comparação concluída | Jina: {}ms | Rust+OpenAI: {}ms | Mais rápido: {} (diff: {}ms)",
            jina_time,
            openai_time,
            faster,
            time_diff.abs()
        );

        ComparisonResult {
            url: url.to_string(),
            jina: jina_result,
            rust_openai: openai_result,
            faster,
            time_diff_ms: time_diff,
        }
    }

    /// Lê usando Jina Reader API
    async fn read_with_jina(&self, url: &str) -> Option<ReaderResult> {
        let start = Instant::now();

        #[derive(Serialize)]
        struct JinaRequest {
            url: String,
        }

        #[derive(Deserialize)]
        struct JinaResponse {
            #[allow(dead_code)]
            code: i32,
            data: Option<JinaData>,
        }

        #[derive(Deserialize)]
        struct JinaData {
            title: String,
            content: String,
        }

        let result = self
            .client
            .post("https://r.jina.ai/")
            .header("Authorization", format!("Bearer {}", self.jina_api_key))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&JinaRequest {
                url: url.to_string(),
            })
            .send()
            .await;

        let elapsed = start.elapsed().as_millis();

        match result {
            Ok(response) => {
                if !response.status().is_success() {
                    return Some(ReaderResult {
                        title: String::new(),
                        content: String::new(),
                        word_count: 0,
                        time_ms: elapsed,
                        source: "jina".to_string(),
                        error: Some(format!("HTTP {}", response.status())),
                    });
                }

                match response.json::<JinaResponse>().await {
                    Ok(data) => {
                        let content = data
                            .data
                            .as_ref()
                            .map(|d| d.content.clone())
                            .unwrap_or_default();
                        let word_count = content.split_whitespace().count();

                        Some(ReaderResult {
                            title: data
                                .data
                                .as_ref()
                                .map(|d| d.title.clone())
                                .unwrap_or_default(),
                            content,
                            word_count,
                            time_ms: elapsed,
                            source: "jina".to_string(),
                            error: None,
                        })
                    }
                    Err(e) => Some(ReaderResult {
                        title: String::new(),
                        content: String::new(),
                        word_count: 0,
                        time_ms: elapsed,
                        source: "jina".to_string(),
                        error: Some(e.to_string()),
                    }),
                }
            }
            Err(e) => Some(ReaderResult {
                title: String::new(),
                content: String::new(),
                word_count: 0,
                time_ms: elapsed,
                source: "jina".to_string(),
                error: Some(e.to_string()),
            }),
        }
    }

    /// Lê usando Rust (download) + OpenAI gpt-4o-mini (extração)
    async fn read_with_rust_openai(&self, url: &str) -> Option<ReaderResult> {
        let start = Instant::now();

        // 1. Baixar HTML
        let html = match self.download_html(url).await {
            Ok(h) => h,
            Err(e) => {
                return Some(ReaderResult {
                    title: String::new(),
                    content: String::new(),
                    word_count: 0,
                    time_ms: start.elapsed().as_millis(),
                    source: "rust_openai".to_string(),
                    error: Some(format!("Download error: {}", e)),
                });
            }
        };

        // 2. Extrair conteúdo com OpenAI gpt-4o-mini
        match self.extract_with_openai(&html, url).await {
            Ok((title, content)) => {
                let word_count = content.split_whitespace().count();
                Some(ReaderResult {
                    title,
                    content,
                    word_count,
                    time_ms: start.elapsed().as_millis(),
                    source: "rust_openai".to_string(),
                    error: None,
                })
            }
            Err(e) => Some(ReaderResult {
                title: String::new(),
                content: String::new(),
                word_count: 0,
                time_ms: start.elapsed().as_millis(),
                source: "rust_openai".to_string(),
                error: Some(e),
            }),
        }
    }

    /// Baixa HTML de uma URL
    async fn download_html(&self, url: &str) -> Result<String, String> {
        let response = self
            .client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; DeepResearch/1.0)")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        response.text().await.map_err(|e| e.to_string())
    }

    /// Extrai conteúdo usando OpenAI gpt-4o-mini
    async fn extract_with_openai(&self, html: &str, url: &str) -> Result<(String, String), String> {
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            temperature: f32,
            max_tokens: u32,
        }

        #[derive(Serialize)]
        struct ChatMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<ChatChoice>,
        }

        #[derive(Deserialize)]
        struct ChatChoice {
            message: ChatResponseMessage,
        }

        #[derive(Deserialize)]
        struct ChatResponseMessage {
            content: String,
        }

        // Limitar HTML para não exceder tokens
        let html_truncated = if html.len() > 50000 {
            &html[..50000]
        } else {
            html
        };

        let system_prompt = r#"You are a web content extractor. Given HTML content, extract:
1. The main title of the page
2. The main text content, removing navigation, ads, scripts, and other non-content elements

Respond in this exact format:
TITLE: <extracted title>
CONTENT:
<extracted main content>"#;

        let request = ChatRequest {
            model: "gpt-4.1-mini".to_string(), // 1M tokens context window
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!("URL: {}\n\nHTML:\n{}", url, html_truncated),
                },
            ],
            temperature: 0.1,
            max_tokens: 4000,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.openai_api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error: {}", error_text));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse response
        let title = content
            .lines()
            .find(|l| l.starts_with("TITLE:"))
            .map(|l| l.trim_start_matches("TITLE:").trim().to_string())
            .unwrap_or_default();

        let main_content = content
            .split("CONTENT:")
            .nth(1)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        Ok((title, main_content))
    }

    /// Compara múltiplas URLs em paralelo
    pub async fn compare_batch(&self, urls: &[&str]) -> Vec<ComparisonResult> {
        use futures::future::join_all;

        log::info!("🔄 Comparação em batch: {} URLs", urls.len());

        let futures: Vec<_> = urls.iter().map(|url| self.compare_read(url)).collect();

        let results = join_all(futures).await;

        // Estatísticas
        let jina_wins = results.iter().filter(|r| r.faster == "jina").count();
        let openai_wins = results.iter().filter(|r| r.faster == "rust_openai").count();
        let jina_total: u128 = results
            .iter()
            .filter_map(|r| r.jina.as_ref())
            .map(|j| j.time_ms)
            .sum();
        let openai_total: u128 = results
            .iter()
            .filter_map(|r| r.rust_openai.as_ref())
            .map(|o| o.time_ms)
            .sum();

        log::info!(
            "📊 Batch Summary:\n\
             - Jina: {} wins, total {}ms\n\
             - Rust+OpenAI: {} wins, total {}ms",
            jina_wins,
            jina_total,
            openai_wins,
            openai_total
        );

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requer API keys
    async fn test_comparison() {
        let jina_key = std::env::var("JINA_API_KEY").unwrap();
        let openai_key = std::env::var("OPENAI_API_KEY").unwrap();

        let comparison = ReaderComparison::new(jina_key, openai_key);
        let result = comparison.compare_read("https://example.com").await;

        println!("{:?}", result);
    }
}
