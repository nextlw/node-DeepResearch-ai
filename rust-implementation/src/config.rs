// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONFIGURAÇÃO DO RUNTIME, WEBREADER E LLM
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Configurações para o runtime Tokio, escolha do WebReader e modelos LLM.
// Todas as configurações podem ser definidas via .env
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::fmt;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONFIGURAÇÃO DO LLM
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Provider de LLM suportado.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LlmProvider {
    /// OpenAI API (padrão)
    #[default]
    OpenAI,
    /// Anthropic Claude API
    Anthropic,
    /// API local (Ollama, etc.)
    Local,
}

impl LlmProvider {
    /// Converte string do .env para LlmProvider.
    pub fn from_env(value: &str) -> Self {
        match value.to_lowercase().trim() {
            "openai" => Self::OpenAI,
            "anthropic" | "claude" => Self::Anthropic,
            "local" | "ollama" => Self::Local,
            _ => Self::OpenAI,
        }
    }

    /// Retorna nome legível.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OpenAI => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::Local => "Local",
        }
    }
}

impl fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Provider de Embeddings suportado.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EmbeddingProvider {
    /// OpenAI Embeddings (text-embedding-3-small, etc.)
    #[default]
    OpenAI,
    /// Jina AI Embeddings (jina-embeddings-v4, etc.)
    Jina,
}

impl EmbeddingProvider {
    /// Converte string do .env para EmbeddingProvider.
    pub fn from_env(value: &str) -> Self {
        match value.to_lowercase().trim() {
            "jina" => Self::Jina,
            _ => Self::OpenAI,
        }
    }

    /// Retorna nome legível.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OpenAI => "OpenAI",
            Self::Jina => "Jina",
        }
    }
}

impl fmt::Display for EmbeddingProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Configuração do LLM.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Provider de LLM (openai, anthropic, local)
    pub provider: LlmProvider,

    /// Modelo principal para geração de texto.
    /// Padrão: "gpt-4.1-mini" para OpenAI
    pub model: String,

    /// Provider de embeddings (pode ser diferente do LLM)
    /// Padrão: OpenAI
    pub embedding_provider: EmbeddingProvider,

    /// Modelo para geração de embeddings OpenAI.
    /// Padrão: "text-embedding-3-small"
    pub embedding_model: String,

    /// Modelo para geração de embeddings Jina.
    /// Padrão: "jina-embeddings-v4"
    pub jina_embedding_model: String,

    /// URL base da API (para providers locais/custom)
    pub api_base_url: Option<String>,

    /// Temperatura padrão para geração (0.0 a 2.0)
    pub default_temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::default(),
            model: "gpt-4.1-mini".to_string(),
            embedding_provider: EmbeddingProvider::default(),
            embedding_model: "text-embedding-3-small".to_string(),
            jina_embedding_model: "jina-embeddings-v4".to_string(),
            api_base_url: None,
            default_temperature: 0.7,
        }
    }
}

impl LlmConfig {
    /// Retorna o modelo de embedding ativo baseado no provider selecionado
    pub fn active_embedding_model(&self) -> &str {
        match self.embedding_provider {
            EmbeddingProvider::OpenAI => &self.embedding_model,
            EmbeddingProvider::Jina => &self.jina_embedding_model,
        }
    }

    /// Verifica se deve usar Jina para embeddings
    pub fn use_jina_embeddings(&self) -> bool {
        self.embedding_provider == EmbeddingProvider::Jina
    }
}

impl LlmConfig {
    /// Retorna URL base da API para o provider.
    pub fn api_url(&self) -> &str {
        if let Some(url) = &self.api_base_url {
            url
        } else {
            match self.provider {
                LlmProvider::OpenAI => "https://api.openai.com/v1",
                LlmProvider::Anthropic => "https://api.anthropic.com/v1",
                LlmProvider::Local => "http://localhost:11434/api",
            }
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONFIGURAÇÃO DO AGENTE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Configuração do comportamento do agente.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Número mínimo de steps antes de permitir ANSWER.
    /// Padrão: 1 (obriga pelo menos uma pesquisa)
    pub min_steps_before_answer: usize,

    /// Se permite resposta direta sem pesquisa (perguntas triviais).
    /// Padrão: false
    pub allow_direct_answer: bool,

    /// Budget máximo de tokens (se não especificado via CLI).
    /// Padrão: 1_000_000
    pub default_token_budget: u64,

    /// Máximo de URLs por step de leitura.
    /// Padrão: 10
    pub max_urls_per_step: usize,

    /// Máximo de queries por step de busca.
    /// Padrão: 5
    pub max_queries_per_step: usize,

    /// Máximo de falhas consecutivas antes de forçar resposta.
    /// Padrão: 3
    pub max_consecutive_failures: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            min_steps_before_answer: 1,
            allow_direct_answer: false,
            default_token_budget: 1_000_000,
            max_urls_per_step: 10,
            max_queries_per_step: 5,
            max_consecutive_failures: 3,
        }
    }
}

/// Preferência de método para leitura de URLs.
///
/// Define qual backend usar para extrair conteúdo de páginas web:
/// - `JinaOnly`: Apenas Jina Reader API (sem fallback)
/// - `RustOnly`: Apenas Rust local + Readability (sem fallback)
/// - `Compare`: Tenta Rust primeiro, Jina como fallback (padrão)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebReaderPreference {
    /// Usar apenas Jina Reader API.
    /// Mais confiável para sites complexos, mas depende de API externa.
    JinaOnly,

    /// Usar apenas Rust local + Readability.
    /// Mais rápido e sem dependência externa, mas pode falhar em sites complexos.
    RustOnly,

    /// Tentar Rust primeiro, Jina como fallback (comportamento padrão).
    /// Melhor dos dois mundos: velocidade quando possível, confiabilidade quando necessário.
    #[default]
    Compare,
}

impl WebReaderPreference {
    /// Converte string do .env para WebReaderPreference.
    ///
    /// Case-insensitive:
    /// - "jina" → JinaOnly
    /// - "rust" → RustOnly
    /// - "compare" ou qualquer outro valor → Compare
    pub fn from_env(value: &str) -> Self {
        match value.to_lowercase().trim() {
            "jina" => Self::JinaOnly,
            "rust" => Self::RustOnly,
            _ => Self::Compare,
        }
    }

    /// Retorna nome legível para logs.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::JinaOnly => "Jina Only",
            Self::RustOnly => "Rust Only",
            Self::Compare => "Compare (Rust → Jina)",
        }
    }
}

impl fmt::Display for WebReaderPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Configuração do runtime Tokio.
///
/// Controla número de threads e comportamento do async runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Número de worker threads do Tokio.
    /// Se None, usa cálculo dinâmico: min(cpu_cores, max_threads).
    pub worker_threads: Option<usize>,

    /// Número máximo de threads (limite superior para cálculo dinâmico).
    /// Padrão: 16
    pub max_threads: usize,

    /// Número máximo de blocking threads.
    /// Padrão: 512 (padrão do Tokio)
    pub max_blocking_threads: usize,

    /// Nome da thread principal.
    pub thread_name: String,

    /// Preferência de WebReader.
    pub webreader: WebReaderPreference,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: None, // Dinâmico
            max_threads: 16,
            max_blocking_threads: 512,
            thread_name: "deep-research".to_string(),
            webreader: WebReaderPreference::default(),
        }
    }
}

impl RuntimeConfig {
    /// Cria configuração padrão.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calcula número efetivo de worker threads.
    ///
    /// Se `worker_threads` está definido, usa esse valor.
    /// Senão, calcula: min(cpu_cores, max_threads)
    pub fn effective_worker_threads(&self) -> usize {
        if let Some(threads) = self.worker_threads {
            threads
        } else {
            let cpu_cores = num_cpus::get();
            std::cmp::min(cpu_cores, self.max_threads)
        }
    }
}

/// Carrega configuração do runtime a partir das variáveis de ambiente.
///
/// Variáveis suportadas:
/// - `TOKIO_THREADS`: Número fixo de threads (opcional)
/// - `TOKIO_MAX_THREADS`: Máximo de threads para cálculo dinâmico (padrão: 16)
/// - `TOKIO_MAX_BLOCKING`: Máximo de blocking threads (padrão: 512)
/// - `WEBREADER`: Preferência de reader ("jina", "rust", "compare")
///
/// # Exemplo
///
/// ```rust,ignore
/// // .env
/// TOKIO_THREADS=4
/// WEBREADER=rust
///
/// // código
/// let config = load_runtime_config();
/// assert_eq!(config.worker_threads, Some(4));
/// assert_eq!(config.webreader, WebReaderPreference::RustOnly);
/// ```
pub fn load_runtime_config() -> RuntimeConfig {
    let mut config = RuntimeConfig::default();

    // TOKIO_THREADS: número fixo de threads
    if let Ok(threads_str) = std::env::var("TOKIO_THREADS") {
        if let Ok(threads) = threads_str.parse::<usize>() {
            if threads > 0 {
                config.worker_threads = Some(threads);
                log::info!("📦 TOKIO_THREADS={} (fixo)", threads);
            }
        }
    }

    // TOKIO_MAX_THREADS: limite superior para cálculo dinâmico
    if let Ok(max_str) = std::env::var("TOKIO_MAX_THREADS") {
        if let Ok(max) = max_str.parse::<usize>() {
            if max > 0 {
                config.max_threads = max;
                log::info!("📦 TOKIO_MAX_THREADS={}", max);
            }
        }
    }

    // TOKIO_MAX_BLOCKING: máximo de blocking threads
    if let Ok(blocking_str) = std::env::var("TOKIO_MAX_BLOCKING") {
        if let Ok(blocking) = blocking_str.parse::<usize>() {
            if blocking > 0 {
                config.max_blocking_threads = blocking;
                log::info!("📦 TOKIO_MAX_BLOCKING={}", blocking);
            }
        }
    }

    // WEBREADER: preferência de método de leitura
    if let Ok(webreader_str) = std::env::var("WEBREADER") {
        config.webreader = WebReaderPreference::from_env(&webreader_str);
        log::info!("📦 WEBREADER={}", config.webreader);
    }

    // Log da configuração efetiva
    let effective_threads = config.effective_worker_threads();
    let cpu_cores = num_cpus::get();

    if config.worker_threads.is_none() {
        log::info!(
            "🔧 Tokio: {} threads (dinâmico: min({} cores, {} max))",
            effective_threads,
            cpu_cores,
            config.max_threads
        );
    }

    config
}

/// Instala panic hook customizado que não envenena outras threads.
///
/// O panic hook padrão do Rust pode causar "poison" em Mutex/RwLock
/// quando uma thread entra em panic enquanto segura um lock.
///
/// Este hook customizado:
/// 1. Loga o panic com informações da thread
/// 2. NÃO propaga o panic para outras threads
/// 3. Permite que o runtime Tokio continue funcionando
///
/// # Uso
///
/// ```rust,ignore
/// install_panic_hook();
/// // Agora panics em threads individuais não afetam outras threads
/// ```
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        let thread = std::thread::current();
        let thread_id = format!("{:?}", thread.id());
        let thread_name = thread.name().unwrap_or("unnamed");

        // Extrair localização do panic
        let location = panic_info.location().map(|loc| {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        }).unwrap_or_else(|| "unknown location".to_string());

        // Extrair mensagem do panic
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        // Log estruturado do panic (não usa eprintln para não corromper TUI)
        log::error!(
            "[PANIC] Thread {} ({}) at {}: {}",
            thread_id,
            thread_name,
            location,
            message
        );

        // Chamar hook original para manter comportamento padrão de logging
        // mas NÃO abortar o processo inteiro
        // O Tokio vai capturar o JoinError e permitir que outras tasks continuem

        // Nota: não chamamos original_hook aqui para evitar abort em release mode
        // Em vez disso, deixamos o Tokio lidar com o panic da task
        let _ = &original_hook; // Evita warning de unused
    }));
}

/// Cria o runtime Tokio com configuração customizada.
///
/// Esta função deve ser chamada no início do programa, antes de qualquer
/// código async. Configura:
/// - Número de worker threads (dinâmico ou fixo)
/// - Número máximo de blocking threads
/// - Panic hook isolado
///
/// # Exemplo
///
/// ```rust,ignore
/// fn main() {
///     let config = load_runtime_config();
///     let runtime = create_tokio_runtime(&config).expect("Failed to create runtime");
///
///     runtime.block_on(async {
///         // código async aqui
///     });
/// }
/// ```
pub fn create_tokio_runtime(config: &RuntimeConfig) -> std::io::Result<tokio::runtime::Runtime> {
    let worker_threads = config.effective_worker_threads();

    log::info!(
        "🚀 Criando runtime Tokio: {} workers, {} blocking max",
        worker_threads,
        config.max_blocking_threads
    );

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(config.max_blocking_threads)
        .thread_name(&config.thread_name)
        .enable_all()
        .build()
}

/// Carrega configuração do LLM a partir das variáveis de ambiente.
///
/// Variáveis suportadas:
/// - `LLM_PROVIDER`: Provider de LLM ("openai", "anthropic", "local") - padrão: "openai"
/// - `LLM_MODEL`: Modelo principal para texto - padrão: "gpt-4.1-mini"
/// - `EMBEDDING_PROVIDER`: Provider de embeddings ("openai", "jina") - padrão: "openai"
/// - `LLM_EMBEDDING_MODEL`: Modelo OpenAI para embeddings - padrão: "text-embedding-3-small"
/// - `JINA_EMBEDDING_MODEL`: Modelo Jina para embeddings - padrão: "jina-embeddings-v4"
/// - `LLM_API_BASE_URL`: URL base customizada (opcional)
/// - `LLM_TEMPERATURE`: Temperatura padrão (0.0 a 2.0) - padrão: 0.7
///
/// # Exemplo
///
/// ```rust,ignore
/// // .env - Usar OpenAI para LLM e Jina para embeddings
/// LLM_PROVIDER=openai
/// LLM_MODEL=gpt-4o
/// EMBEDDING_PROVIDER=jina
/// JINA_EMBEDDING_MODEL=jina-embeddings-v4
///
/// // código
/// let config = load_llm_config();
/// assert_eq!(config.model, "gpt-4o");
/// assert!(config.use_jina_embeddings());
/// ```
pub fn load_llm_config() -> LlmConfig {
    let mut config = LlmConfig::default();

    // LLM_PROVIDER: provider de LLM
    if let Ok(provider_str) = std::env::var("LLM_PROVIDER") {
        config.provider = LlmProvider::from_env(&provider_str);
        log::info!("📦 LLM_PROVIDER={}", config.provider);
    }

    // LLM_MODEL: modelo principal
    if let Ok(model) = std::env::var("LLM_MODEL") {
        let model = model.trim().to_string();
        if !model.is_empty() {
            config.model = model;
            log::info!("📦 LLM_MODEL={}", config.model);
        }
    }

    // EMBEDDING_PROVIDER: provider de embeddings (pode ser diferente do LLM)
    if let Ok(provider_str) = std::env::var("EMBEDDING_PROVIDER") {
        config.embedding_provider = EmbeddingProvider::from_env(&provider_str);
        log::info!("📦 EMBEDDING_PROVIDER={}", config.embedding_provider);
    }

    // LLM_EMBEDDING_MODEL: modelo OpenAI de embeddings
    if let Ok(model) = std::env::var("LLM_EMBEDDING_MODEL") {
        let model = model.trim().to_string();
        if !model.is_empty() {
            config.embedding_model = model;
            log::info!("📦 LLM_EMBEDDING_MODEL={}", config.embedding_model);
        }
    }

    // JINA_EMBEDDING_MODEL: modelo Jina de embeddings
    if let Ok(model) = std::env::var("JINA_EMBEDDING_MODEL") {
        let model = model.trim().to_string();
        if !model.is_empty() {
            config.jina_embedding_model = model;
            log::info!("📦 JINA_EMBEDDING_MODEL={}", config.jina_embedding_model);
        }
    }

    // LLM_API_BASE_URL: URL base customizada
    if let Ok(url) = std::env::var("LLM_API_BASE_URL") {
        let url = url.trim().to_string();
        if !url.is_empty() {
            config.api_base_url = Some(url.clone());
            log::info!("📦 LLM_API_BASE_URL={}", url);
        }
    }

    // LLM_TEMPERATURE: temperatura padrão
    if let Ok(temp_str) = std::env::var("LLM_TEMPERATURE") {
        if let Ok(temp) = temp_str.parse::<f32>() {
            if (0.0..=2.0).contains(&temp) {
                config.default_temperature = temp;
                log::info!("📦 LLM_TEMPERATURE={}", temp);
            }
        }
    }

    // Log do provider de embedding ativo
    log::info!(
        "🔢 Embedding: {} ({})",
        config.embedding_provider,
        config.active_embedding_model()
    );

    config
}

/// Carrega configuração do agente a partir das variáveis de ambiente.
///
/// Variáveis suportadas:
/// - `AGENT_MIN_STEPS`: Mínimo de steps antes de ANSWER - padrão: 1
/// - `AGENT_ALLOW_DIRECT_ANSWER`: Permite resposta direta ("true"/"false") - padrão: false
/// - `AGENT_TOKEN_BUDGET`: Budget de tokens - padrão: 1000000
/// - `AGENT_MAX_URLS_PER_STEP`: Máximo de URLs por step - padrão: 10
/// - `AGENT_MAX_QUERIES_PER_STEP`: Máximo de queries por step - padrão: 5
/// - `AGENT_MAX_FAILURES`: Máximo de falhas consecutivas - padrão: 3
///
/// # Exemplo
///
/// ```rust,ignore
/// // .env
/// AGENT_MIN_STEPS=2
/// AGENT_ALLOW_DIRECT_ANSWER=false
/// AGENT_TOKEN_BUDGET=500000
///
/// // código
/// let config = load_agent_config();
/// assert_eq!(config.min_steps_before_answer, 2);
/// ```
pub fn load_agent_config() -> AgentConfig {
    let mut config = AgentConfig::default();

    // AGENT_MIN_STEPS: mínimo de steps antes de permitir ANSWER
    if let Ok(steps_str) = std::env::var("AGENT_MIN_STEPS") {
        if let Ok(steps) = steps_str.parse::<usize>() {
            config.min_steps_before_answer = steps;
            log::info!("📦 AGENT_MIN_STEPS={}", steps);
        }
    }

    // AGENT_ALLOW_DIRECT_ANSWER: permite resposta direta sem pesquisa
    if let Ok(allow_str) = std::env::var("AGENT_ALLOW_DIRECT_ANSWER") {
        let allow = matches!(allow_str.to_lowercase().trim(), "true" | "1" | "yes" | "sim");
        config.allow_direct_answer = allow;
        log::info!("📦 AGENT_ALLOW_DIRECT_ANSWER={}", allow);
    }

    // AGENT_TOKEN_BUDGET: budget máximo de tokens
    if let Ok(budget_str) = std::env::var("AGENT_TOKEN_BUDGET") {
        if let Ok(budget) = budget_str.parse::<u64>() {
            if budget > 0 {
                config.default_token_budget = budget;
                log::info!("📦 AGENT_TOKEN_BUDGET={}", budget);
            }
        }
    }

    // AGENT_MAX_URLS_PER_STEP: máximo de URLs por step
    if let Ok(max_str) = std::env::var("AGENT_MAX_URLS_PER_STEP") {
        if let Ok(max) = max_str.parse::<usize>() {
            if max > 0 {
                config.max_urls_per_step = max;
                log::info!("📦 AGENT_MAX_URLS_PER_STEP={}", max);
            }
        }
    }

    // AGENT_MAX_QUERIES_PER_STEP: máximo de queries por step
    if let Ok(max_str) = std::env::var("AGENT_MAX_QUERIES_PER_STEP") {
        if let Ok(max) = max_str.parse::<usize>() {
            if max > 0 {
                config.max_queries_per_step = max;
                log::info!("📦 AGENT_MAX_QUERIES_PER_STEP={}", max);
            }
        }
    }

    // AGENT_MAX_FAILURES: máximo de falhas consecutivas
    if let Ok(max_str) = std::env::var("AGENT_MAX_FAILURES") {
        if let Ok(max) = max_str.parse::<usize>() {
            config.max_consecutive_failures = max;
            log::info!("📦 AGENT_MAX_FAILURES={}", max);
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webreader_preference_from_env() {
        assert_eq!(WebReaderPreference::from_env("jina"), WebReaderPreference::JinaOnly);
        assert_eq!(WebReaderPreference::from_env("JINA"), WebReaderPreference::JinaOnly);
        assert_eq!(WebReaderPreference::from_env("rust"), WebReaderPreference::RustOnly);
        assert_eq!(WebReaderPreference::from_env("RUST"), WebReaderPreference::RustOnly);
        assert_eq!(WebReaderPreference::from_env("compare"), WebReaderPreference::Compare);
        assert_eq!(WebReaderPreference::from_env("anything"), WebReaderPreference::Compare);
        assert_eq!(WebReaderPreference::from_env(""), WebReaderPreference::Compare);
    }

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert!(config.worker_threads.is_none());
        assert_eq!(config.max_threads, 16);
        assert_eq!(config.max_blocking_threads, 512);
        assert_eq!(config.webreader, WebReaderPreference::Compare);
    }

    #[test]
    fn test_effective_worker_threads_fixed() {
        let mut config = RuntimeConfig::default();
        config.worker_threads = Some(4);
        assert_eq!(config.effective_worker_threads(), 4);
    }

    #[test]
    fn test_effective_worker_threads_dynamic() {
        let config = RuntimeConfig::default();
        let effective = config.effective_worker_threads();
        let cpu_cores = num_cpus::get();
        assert_eq!(effective, std::cmp::min(cpu_cores, 16));
    }

    #[test]
    fn test_webreader_display() {
        assert_eq!(WebReaderPreference::JinaOnly.display_name(), "Jina Only");
        assert_eq!(WebReaderPreference::RustOnly.display_name(), "Rust Only");
        assert_eq!(WebReaderPreference::Compare.display_name(), "Compare (Rust → Jina)");
    }

    #[test]
    fn test_llm_provider_from_env() {
        assert_eq!(LlmProvider::from_env("openai"), LlmProvider::OpenAI);
        assert_eq!(LlmProvider::from_env("OPENAI"), LlmProvider::OpenAI);
        assert_eq!(LlmProvider::from_env("anthropic"), LlmProvider::Anthropic);
        assert_eq!(LlmProvider::from_env("claude"), LlmProvider::Anthropic);
        assert_eq!(LlmProvider::from_env("local"), LlmProvider::Local);
        assert_eq!(LlmProvider::from_env("ollama"), LlmProvider::Local);
        assert_eq!(LlmProvider::from_env("unknown"), LlmProvider::OpenAI);
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::OpenAI);
        assert_eq!(config.model, "gpt-4.1-mini");
        assert_eq!(config.embedding_model, "text-embedding-3-small");
        assert!(config.api_base_url.is_none());
        assert!((config.default_temperature - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_llm_config_api_url() {
        let mut config = LlmConfig::default();
        assert_eq!(config.api_url(), "https://api.openai.com/v1");

        config.provider = LlmProvider::Anthropic;
        assert_eq!(config.api_url(), "https://api.anthropic.com/v1");

        config.provider = LlmProvider::Local;
        assert_eq!(config.api_url(), "http://localhost:11434/api");

        config.api_base_url = Some("http://custom:8080".to_string());
        assert_eq!(config.api_url(), "http://custom:8080");
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.min_steps_before_answer, 1);
        assert!(!config.allow_direct_answer);
        assert_eq!(config.default_token_budget, 1_000_000);
        assert_eq!(config.max_urls_per_step, 10);
        assert_eq!(config.max_queries_per_step, 5);
        assert_eq!(config.max_consecutive_failures, 3);
    }
}
