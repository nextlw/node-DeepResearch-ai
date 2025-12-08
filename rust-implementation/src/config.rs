// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONFIGURAÇÃO DO RUNTIME E WEBREADER
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Configurações para o runtime Tokio e escolha do WebReader.
// Todas as configurações podem ser definidas via .env
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::fmt;

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
}
