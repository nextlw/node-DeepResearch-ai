// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CLIENTE LLM
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Trait e implementações para interação com modelos de linguagem.
// Suporta múltiplos provedores: OpenAI, Anthropic, local, etc.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::agent::{ActionPermissions, AgentAction, AgentPrompt};
use crate::types::{Reference, SerpQuery};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Erros que podem ocorrer na comunicação com LLMs.
///
/// Estes são os tipos de falha possíveis ao chamar APIs como OpenAI.
/// Cada variante requer tratamento diferente:
/// - `RateLimitError`: Aguardar e tentar novamente
/// - `NetworkError`: Verificar conectividade
/// - `TokenLimitError`: Reduzir tamanho do prompt
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Erro retornado pela API do provedor.
    ///
    /// Exemplos: API key inválida, modelo não existe, quota excedida.
    #[error("API error: {0}")]
    ApiError(String),

    /// Limite de requisições por minuto excedido.
    ///
    /// A maioria das APIs tem rate limits (ex: OpenAI = 3500 RPM).
    /// Aguarde alguns segundos antes de tentar novamente.
    #[error("Rate limit exceeded")]
    RateLimitError,

    /// Resposta do LLM não está no formato esperado.
    ///
    /// O modelo retornou algo que não conseguimos interpretar.
    /// Pode indicar prompt mal formulado ou modelo instável.
    #[error("Invalid response format: {0}")]
    ParseError(String),

    /// Erro de rede na comunicação com a API.
    ///
    /// Problemas de DNS, timeout, conexão recusada, etc.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Prompt excedeu o limite de tokens do modelo.
    ///
    /// Cada modelo tem um limite (ex: GPT-4 = 128K tokens).
    /// Reduza o contexto ou use um modelo com limite maior.
    #[error("Token limit exceeded: {used} > {limit}")]
    TokenLimitError {
        /// Quantidade de tokens que tentamos usar.
        used: u64,
        /// Limite máximo do modelo.
        limit: u64,
    },
}

/// Resposta gerada pelo LLM para uma pergunta.
///
/// Contém o texto da resposta, referências extraídas,
/// e estatísticas detalhadas de uso de tokens para monitoramento.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Texto completo da resposta gerada.
    pub answer: String,
    /// Referências citadas na resposta (se houver).
    ///
    /// Extraídas automaticamente do contexto ou
    /// geradas pelo modelo se solicitado.
    pub references: Vec<Reference>,
    /// Tokens consumidos pelo prompt (entrada).
    pub prompt_tokens: u64,
    /// Tokens gerados na completion (saída).
    pub completion_tokens: u64,
    /// Total de tokens consumidos (prompt + completion).
    pub total_tokens: u64,
}

/// Resultado de uma operação de embedding.
///
/// Embeddings são representações numéricas de texto que capturam
/// seu significado semântico. Textos similares têm embeddings próximos.
///
/// ## O que é um Embedding?
/// Imagine transformar uma frase em uma lista de 1536 números.
/// Frases com significado similar terão números parecidos.
/// Isso permite comparar textos matematicamente.
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    /// Vetor de embedding (geralmente 1536 dimensões para OpenAI).
    ///
    /// Use `cosine_similarity` para comparar dois vetores.
    pub vector: Vec<f32>,
    /// Tokens consumidos para gerar este embedding.
    ///
    /// Embeddings são mais baratos que completions.
    pub tokens_used: u64,
}

/// Trait principal para clientes LLM
///
/// Esta trait define a interface que qualquer provedor de LLM deve implementar.
/// Permite fácil substituição entre provedores (OpenAI, Anthropic, local).
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Decide a próxima ação baseado no prompt e permissões
    async fn decide_action(
        &self,
        prompt: &AgentPrompt,
        permissions: &ActionPermissions,
    ) -> Result<AgentAction, LlmError>;

    /// Gera uma resposta final para a pergunta
    async fn generate_answer(
        &self,
        prompt: &AgentPrompt,
        temperature: f32,
    ) -> Result<LlmResponse, LlmError>;

    /// Gera embeddings para um texto
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, LlmError>;

    /// Gera embeddings em batch
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError>;

    /// Avalia se uma resposta atende aos critérios
    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        criteria: &str,
    ) -> Result<EvaluationResponse, LlmError>;

    /// Determina os tipos de avaliação necessários
    async fn determine_eval_types(
        &self,
        question: &str,
    ) -> Result<Vec<crate::evaluation::EvaluationType>, LlmError>;

    /// Retorna tokens de prompt acumulados (implementação opcional)
    fn get_prompt_tokens(&self) -> u64 {
        0
    }

    /// Retorna tokens de completion acumulados (implementação opcional)
    fn get_completion_tokens(&self) -> u64 {
        0
    }

    /// Retorna total de tokens acumulados (implementação opcional)
    fn get_total_tokens(&self) -> u64 {
        self.get_prompt_tokens() + self.get_completion_tokens()
    }
}

/// Resposta de uma avaliação feita pelo LLM.
///
/// Quando pedimos ao LLM para avaliar se uma resposta
/// atende a certos critérios, ele retorna esta estrutura.
#[derive(Debug, Clone)]
pub struct EvaluationResponse {
    /// Se a resposta passou na avaliação.
    pub passed: bool,
    /// Explicação do raciocínio usado pelo avaliador.
    ///
    /// Importante para entender por que passou ou não.
    pub reasoning: String,
    /// Nível de confiança na avaliação (0.0 a 1.0).
    ///
    /// Valores baixos indicam que o avaliador não tem certeza.
    pub confidence: f32,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO MOCK PARA TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cliente mock para testes unitários.
///
/// Simula respostas do LLM sem fazer chamadas reais à API.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct MockLlmClient {
    /// Ação padrão a retornar quando `decide_action` é chamado.
    pub default_action: Option<AgentAction>,
}

#[cfg(test)]
impl MockLlmClient {
    /// Cria um novo cliente MockLlmClient com valores padrão.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = MockLlmClient::new();
    /// ```
    pub fn new() -> Self {
        Self {
            default_action: None,
        }
    }

    /// Cria um novo cliente MockLlmClient com uma ação padrão.
    ///
    /// # Argumentos
    /// * `action` - A ação padrão a retornar quando `decide_action` é chamado.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = MockLlmClient::with_action(AgentAction::Search {
    ///     queries: vec![],
    ///     think: "Mock search".into(),
    /// });
    /// ```
    pub fn with_action(action: AgentAction) -> Self {
        Self {
            default_action: Some(action.clone()),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl LlmClient for MockLlmClient {
    async fn decide_action(
        &self,
        _prompt: &AgentPrompt,
        permissions: &ActionPermissions,
    ) -> Result<AgentAction, LlmError> {
        if let Some(action) = &self.default_action {
            return Ok(action.clone());
        }

        // Retorna primeira ação permitida
        if permissions.search {
            Ok(AgentAction::Search {
                queries: vec![],
                think: "Mock search".into(),
            })
        } else if permissions.answer {
            Ok(AgentAction::Answer {
                answer: "Mock answer".into(),
                references: vec![],
                think: "Mock answer".into(),
            })
        } else {
            Err(LlmError::ApiError("No valid action".into()))
        }
    }

    async fn generate_answer(
        &self,
        _prompt: &AgentPrompt,
        _temperature: f32,
    ) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            answer: "Mock generated answer".into(),
            references: vec![],
            prompt_tokens: 80,
            completion_tokens: 20,
            total_tokens: 100,
        })
    }

    async fn embed(&self, _text: &str) -> Result<EmbeddingResult, LlmError> {
        Ok(EmbeddingResult {
            vector: vec![0.0; 1536],
            tokens_used: 10,
        })
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError> {
        Ok(texts
            .iter()
            .map(|_| EmbeddingResult {
                vector: vec![0.0; 1536],
                tokens_used: 10,
            })
            .collect())
    }

    async fn evaluate(
        &self,
        _question: &str,
        _answer: &str,
        _criteria: &str,
    ) -> Result<EvaluationResponse, LlmError> {
        Ok(EvaluationResponse {
            passed: true,
            reasoning: "Mock evaluation passed".into(),
            confidence: 0.95,
        })
    }

    async fn determine_eval_types(
        &self,
        _question: &str,
    ) -> Result<Vec<crate::evaluation::EvaluationType>, LlmError> {
        Ok(vec![crate::evaluation::EvaluationType::Definitive])
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO OPENAI (STUB)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cliente para OpenAI API
pub struct OpenAiClient {
    /// Chave da API OpenAI
    api_key: String,
    /// Modelo principal para geração de texto
    model: String,
    /// Modelo para geração de embeddings
    embedding_model: String,
    /// URL base da API
    api_base_url: String,
    /// Temperatura padrão
    default_temperature: f32,
    /// Cliente HTTP
    client: reqwest::Client,
    /// Contador de tokens de prompt (thread-safe)
    total_prompt_tokens: std::sync::atomic::AtomicU64,
    /// Contador de tokens de completion (thread-safe)
    total_completion_tokens: std::sync::atomic::AtomicU64,
}

impl OpenAiClient {
    /// Cria um novo cliente OpenAI com configurações padrão.
    ///
    /// # Argumentos
    /// * `api_key` - Sua chave de API OpenAI (começa com "sk-")
    ///
    /// # Modelos Padrão
    /// - Texto: `gpt-4.1-mini`
    /// - Embedding: `text-embedding-3-small`
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = OpenAiClient::new("sk-your-api-key".into());
    /// ```
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "gpt-4.1-mini".into(), // 1M tokens context window, mais capaz
            embedding_model: "text-embedding-3-small".into(),
            api_base_url: "https://api.openai.com/v1".into(),
            default_temperature: 0.7,
            client: reqwest::Client::new(),
            total_prompt_tokens: std::sync::atomic::AtomicU64::new(0),
            total_completion_tokens: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Cria um novo cliente OpenAI a partir de LlmConfig.
    ///
    /// # Argumentos
    /// * `api_key` - Sua chave de API OpenAI
    /// * `config` - Configuração do LLM carregada do .env
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let config = load_llm_config();
    /// let client = OpenAiClient::from_config("sk-key".into(), &config);
    /// ```
    pub fn from_config(api_key: String, config: &crate::config::LlmConfig) -> Self {
        Self {
            api_key,
            model: config.model.clone(),
            embedding_model: config.embedding_model.clone(),
            api_base_url: config.api_url().to_string(),
            default_temperature: config.default_temperature,
            client: reqwest::Client::new(),
            total_prompt_tokens: std::sync::atomic::AtomicU64::new(0),
            total_completion_tokens: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Retorna o total de tokens de prompt acumulados
    pub fn get_total_prompt_tokens(&self) -> u64 {
        self.total_prompt_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Retorna o total de tokens de completion acumulados
    pub fn get_total_completion_tokens(&self) -> u64 {
        self.total_completion_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Retorna o total de todos os tokens acumulados
    pub fn get_total_tokens(&self) -> u64 {
        self.get_total_prompt_tokens() + self.get_total_completion_tokens()
    }

    /// Retorna o modelo atual em uso
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Retorna o modelo de embedding atual em uso
    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }

    /// Altera o modelo de texto usado pelo cliente.
    ///
    /// # Argumentos
    /// * `model` - Nome do modelo (ex: "gpt-4", "gpt-3.5-turbo")
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let client = OpenAiClient::new(api_key)
    ///     .with_model("gpt-4");
    /// ```
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.into();
        self
    }

    /// Altera o modelo de embedding usado pelo cliente.
    pub fn with_embedding_model(mut self, model: &str) -> Self {
        self.embedding_model = model.into();
        self
    }

    /// Altera a URL base da API.
    pub fn with_api_base_url(mut self, url: &str) -> Self {
        self.api_base_url = url.into();
        self
    }

    /// Altera a temperatura padrão.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.default_temperature = temp;
        self
    }

    /// Retorna a URL completa para um endpoint.
    fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.api_base_url.trim_end_matches('/'), path.trim_start_matches('/'))
    }
}

// Estruturas para serialização/deserialização da API OpenAI
#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Usage,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: serde_json::Value,
}

/// Usage específico para embeddings - OpenAI não retorna completion_tokens
#[derive(Deserialize, Debug)]
struct EmbeddingUsage {
    prompt_tokens: u64,
    total_tokens: u64,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    usage: EmbeddingUsage,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct ActionJson {
    action: String,
    queries: Option<Vec<ActionQuery>>,
    urls: Option<Vec<String>>,
    gap_questions: Option<Vec<String>>,
    answer: Option<String>,
    references: Option<Vec<ActionReference>>,
    code: Option<String>,
    think: String,
}

#[derive(Deserialize)]
struct ActionQuery {
    q: String,
    tbs: Option<String>,
    location: Option<String>,
}

#[derive(Deserialize)]
struct ActionReference {
    url: String,
    title: String,
    #[serde(rename = "exactQuote")]
    exact_quote: Option<String>,
    #[serde(rename = "relevanceScore")]
    relevance_score: Option<f32>,
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn decide_action(
        &self,
        prompt: &AgentPrompt,
        permissions: &ActionPermissions,
    ) -> Result<AgentAction, LlmError> {
        let mut system_prompt = prompt.system.clone();
        system_prompt.push_str("\n\nYou must respond with a valid JSON object containing the action. Available actions:\n");

        if permissions.search {
            system_prompt.push_str("- search: {\"action\": \"search\", \"queries\": [{\"q\": \"query text\", \"tbs\": \"optional\", \"location\": \"optional\"}], \"think\": \"reasoning\"}\n");
        }
        if permissions.read {
            system_prompt.push_str("- read: {\"action\": \"read\", \"urls\": [\"url1\", \"url2\"], \"think\": \"reasoning\"}\n");
        }
        if permissions.reflect {
            system_prompt.push_str("- reflect: {\"action\": \"reflect\", \"gap_questions\": [\"question1\"], \"think\": \"reasoning\"}\n");
        }
        if permissions.answer {
            system_prompt.push_str("- answer: {\"action\": \"answer\", \"answer\": \"response text\", \"references\": [{\"url\": \"...\", \"title\": \"...\"}], \"think\": \"reasoning\"}\n");
        }
        if permissions.coding {
            system_prompt.push_str("- coding: {\"action\": \"coding\", \"code\": \"code to execute\", \"think\": \"reasoning\"}\n");
        }

        system_prompt.push_str("\nRespond ONLY with valid JSON, no other text.");

        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".into(),
                content: format!(
                    "{}\n\nDiary:\n{}",
                    prompt.user,
                    prompt
                        .diary
                        .iter()
                        .map(|e| e.format())
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            },
        ];

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.7),
            response_format: Some(serde_json::json!({"type": "json_object"})),
        };

        let response = self
            .client
            .post(self.endpoint("chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(LlmError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))?;

        // Acumular tokens
        self.total_prompt_tokens.fetch_add(
            chat_response.usage.prompt_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.total_completion_tokens.fetch_add(
            chat_response.usage.completion_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );

        // Log token usage
        log::info!(
            "🎫 Tokens: prompt={}, completion={}, total={} | Acumulado: {} | Model: {}",
            chat_response.usage.prompt_tokens,
            chat_response.usage.completion_tokens,
            chat_response.usage.total_tokens,
            self.get_total_tokens(),
            self.model
        );

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ParseError("No choices in response".into()))?
            .message
            .content
            .clone();

        let action_json: ActionJson = serde_json::from_str(&content)
            .map_err(|e| LlmError::ParseError(format!("Failed to parse action JSON: {}", e)))?;

        match action_json.action.as_str() {
            "search" => {
                let queries = action_json
                    .queries
                    .unwrap_or_default()
                    .into_iter()
                    .map(|q| SerpQuery {
                        q: q.q,
                        tbs: q.tbs,
                        location: q.location,
                    })
                    .collect();
                Ok(AgentAction::Search {
                    queries,
                    think: action_json.think,
                })
            }
            "read" => Ok(AgentAction::Read {
                urls: action_json.urls.unwrap_or_default(),
                think: action_json.think,
            }),
            "reflect" => Ok(AgentAction::Reflect {
                gap_questions: action_json.gap_questions.unwrap_or_default(),
                think: action_json.think,
            }),
            "answer" => {
                let references = action_json
                    .references
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| Reference {
                        url: r.url,
                        title: r.title,
                        exact_quote: r.exact_quote,
                        relevance_score: r.relevance_score,
                        answer_chunk: None,
                        answer_position: None,
                    })
                    .collect();
                Ok(AgentAction::Answer {
                    answer: action_json.answer.unwrap_or_default(),
                    references,
                    think: action_json.think,
                })
            }
            "coding" => Ok(AgentAction::Coding {
                code: action_json.code.unwrap_or_default(),
                think: action_json.think,
            }),
            _ => Err(LlmError::ParseError(format!(
                "Unknown action: {}",
                action_json.action
            ))),
        }
    }

    async fn generate_answer(
        &self,
        prompt: &AgentPrompt,
        temperature: f32,
    ) -> Result<LlmResponse, LlmError> {
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: prompt.system.clone(),
            },
            ChatMessage {
                role: "user".into(),
                content: format!(
                    "{}\n\nDiary:\n{}",
                    prompt.user,
                    prompt
                        .diary
                        .iter()
                        .map(|e| e.format())
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            },
        ];

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(temperature),
            response_format: None,
        };

        let response = self
            .client
            .post(self.endpoint("chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(LlmError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))?;

        // Acumular tokens
        self.total_prompt_tokens.fetch_add(
            chat_response.usage.prompt_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.total_completion_tokens.fetch_add(
            chat_response.usage.completion_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        log::debug!(
            "🎫 generate_answer tokens: {} | Acumulado: {} | Model: {}",
            chat_response.usage.total_tokens,
            self.get_total_tokens(),
            self.model
        );

        let answer = chat_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ParseError("No choices in response".into()))?
            .message
            .content
            .clone();

        Ok(LlmResponse {
            answer,
            references: vec![], // References devem ser extraídas do contexto
            prompt_tokens: chat_response.usage.prompt_tokens,
            completion_tokens: chat_response.usage.completion_tokens,
            total_tokens: chat_response.usage.total_tokens,
        })
    }

    async fn embed(&self, text: &str) -> Result<EmbeddingResult, LlmError> {
        let request = EmbeddingRequest {
            model: self.embedding_model.clone(),
            input: serde_json::Value::String(text.to_string()),
        };

        let response = self
            .client
            .post(self.endpoint("embeddings"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(LlmError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))?;

        let embedding_data = embedding_response
            .data
            .first()
            .ok_or_else(|| LlmError::ParseError("No embedding data in response".into()))?;

        log::debug!(
            "🔢 Embedding: dim={} | {} tokens | Model: {}",
            embedding_data.embedding.len(),
            embedding_response.usage.prompt_tokens,
            self.embedding_model
        );

        Ok(EmbeddingResult {
            vector: embedding_data.embedding.clone(),
            tokens_used: embedding_response.usage.prompt_tokens, // Usar prompt_tokens para embeddings
        })
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError> {
        let input: Vec<serde_json::Value> = texts
            .iter()
            .map(|t| serde_json::Value::String(t.clone()))
            .collect();

        let request = EmbeddingRequest {
            model: self.embedding_model.clone(),
            input: serde_json::Value::Array(input),
        };

        let response = self
            .client
            .post(self.endpoint("embeddings"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(LlmError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))?;

        let data_len = embedding_response.data.len() as u64;
        let tokens_per_embedding = embedding_response.usage.prompt_tokens / data_len.max(1);

        log::info!(
            "🔢 Embeddings: {} vetores | dim={} | {} tokens | Model: {}",
            data_len,
            embedding_response.data.first().map(|d| d.embedding.len()).unwrap_or(0),
            embedding_response.usage.prompt_tokens,
            self.embedding_model
        );

        let results: Vec<EmbeddingResult> = embedding_response
            .data
            .into_iter()
            .map(|data| EmbeddingResult {
                vector: data.embedding,
                tokens_used: tokens_per_embedding,
            })
            .collect();

        Ok(results)
    }

    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        criteria: &str,
    ) -> Result<EvaluationResponse, LlmError> {
        let system_prompt = format!(
            "You are an evaluator. Evaluate if the answer meets the criteria: {}\n\nRespond with JSON: {{\"passed\": true/false, \"reasoning\": \"explanation\", \"confidence\": 0.0-1.0}}",
            criteria
        );

        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".into(),
                content: format!("Question: {}\n\nAnswer: {}", question, answer),
            },
        ];

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.3),
            response_format: Some(serde_json::json!({"type": "json_object"})),
        };

        let response = self
            .client
            .post(self.endpoint("chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(LlmError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))?;

        // Acumular tokens
        self.total_prompt_tokens.fetch_add(
            chat_response.usage.prompt_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.total_completion_tokens.fetch_add(
            chat_response.usage.completion_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        log::debug!(
            "🎫 evaluate tokens: {} | Acumulado: {}",
            chat_response.usage.total_tokens,
            self.get_total_tokens()
        );

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ParseError("No choices in response".into()))?
            .message
            .content
            .clone();

        #[derive(Deserialize)]
        struct EvalJson {
            passed: bool,
            reasoning: String,
            confidence: f32,
        }

        let eval_json: EvalJson = serde_json::from_str(&content)
            .map_err(|e| LlmError::ParseError(format!("Failed to parse evaluation JSON: {}", e)))?;

        Ok(EvaluationResponse {
            passed: eval_json.passed,
            reasoning: eval_json.reasoning,
            confidence: eval_json.confidence,
        })
    }

    async fn determine_eval_types(
        &self,
        question: &str,
    ) -> Result<Vec<crate::evaluation::EvaluationType>, LlmError> {
        let system_prompt = r#"You are an evaluator selector. Determine which evaluation types are needed for this question.
Respond with JSON: {"needs_definitive": true/false, "needs_freshness": true/false, "needs_plurality": true/false, "needs_completeness": true/false}
- definitive: Does the question need a clear, confident answer?
- freshness: Does the question require recent/current information?
- plurality: Does the question ask for multiple items/examples?
- completeness: Does the question have multiple aspects that need coverage?"#;

        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system_prompt.into(),
            },
            ChatMessage {
                role: "user".into(),
                content: format!("Question: {}", question),
            },
        ];

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.3),
            response_format: Some(serde_json::json!({"type": "json_object"})),
        };

        let response = self
            .client
            .post(self.endpoint("chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(LlmError::RateLimitError);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(format!("Failed to parse response: {}", e)))?;

        // Acumular tokens
        self.total_prompt_tokens.fetch_add(
            chat_response.usage.prompt_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.total_completion_tokens.fetch_add(
            chat_response.usage.completion_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        log::debug!(
            "🎫 determine_eval_types tokens: {} | Acumulado: {}",
            chat_response.usage.total_tokens,
            self.get_total_tokens()
        );

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ParseError("No choices in response".into()))?
            .message
            .content
            .clone();

        #[derive(Deserialize)]
        struct EvalTypesJson {
            needs_definitive: bool,
            needs_freshness: bool,
            needs_plurality: bool,
            needs_completeness: bool,
        }

        let eval_types_json: EvalTypesJson = serde_json::from_str(&content)
            .map_err(|e| LlmError::ParseError(format!("Failed to parse eval types JSON: {}", e)))?;

        let mut types = Vec::new();
        if eval_types_json.needs_definitive {
            types.push(crate::evaluation::EvaluationType::Definitive);
        }
        if eval_types_json.needs_freshness {
            types.push(crate::evaluation::EvaluationType::Freshness);
        }
        if eval_types_json.needs_plurality {
            types.push(crate::evaluation::EvaluationType::Plurality);
        }
        if eval_types_json.needs_completeness {
            types.push(crate::evaluation::EvaluationType::Completeness);
        }

        // Sempre adiciona Strict se houver outros tipos
        if !types.is_empty() {
            types.push(crate::evaluation::EvaluationType::Strict);
        }

        Ok(types)
    }

    fn get_prompt_tokens(&self) -> u64 {
        self.total_prompt_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    fn get_completion_tokens(&self) -> u64 {
        self.total_completion_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client() {
        let client = MockLlmClient::new();
        let prompt = AgentPrompt {
            system: "test".into(),
            user: "test".into(),
            diary: vec![],
        };
        let permissions = ActionPermissions::all_enabled();

        let action = client.decide_action(&prompt, &permissions).await.unwrap();
        assert!(action.is_search());
    }
}
