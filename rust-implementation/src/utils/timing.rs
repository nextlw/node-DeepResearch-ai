// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TIMING UTILITIES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Utilitários para medir tempo de execução de operações.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::time::{Duration, Instant};

/// Timer para medir duração de operações
pub struct ActionTimer {
    start: Instant,
    action_name: String,
}

impl ActionTimer {
    /// Inicia um novo timer para uma ação
    pub fn start(action_name: &str) -> Self {
        Self {
            start: Instant::now(),
            action_name: action_name.to_string(),
        }
    }

    /// Retorna o tempo decorrido em milissegundos
    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    /// Retorna o tempo decorrido como Duration
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Para o timer e loga o tempo decorrido
    pub fn stop_and_log(self) -> u128 {
        let elapsed = self.elapsed_ms();
        log::info!("⏱️  {} completado em {}ms", self.action_name, elapsed);
        elapsed
    }

    /// Para o timer e retorna o tempo sem logar
    pub fn stop(self) -> u128 {
        self.elapsed_ms()
    }
}

/// Macro para medir tempo de execução de um bloco
#[macro_export]
macro_rules! timed {
    ($name:expr, $block:expr) => {{
        let timer = $crate::utils::ActionTimer::start($name);
        let result = $block;
        timer.stop_and_log();
        result
    }};
}

/// Estatísticas agregadas de tempo
#[derive(Debug, Clone, Default)]
pub struct TimingStats {
    /// Tempos de busca (ms)
    pub search_times: Vec<u128>,
    /// Tempos de leitura (ms)
    pub read_times: Vec<u128>,
    /// Tempos de LLM (ms)
    pub llm_times: Vec<u128>,
    /// Tempos de avaliação (ms)
    pub eval_times: Vec<u128>,
}

impl TimingStats {
    /// Cria uma nova instância de `TimingStats` com todas as métricas zeradas.
    ///
    /// Equivalente a [`TimingStats::default()`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adiciona um tempo de busca
    pub fn add_search_time(&mut self, ms: u128) {
        self.search_times.push(ms);
    }

    /// Adiciona um tempo de leitura
    pub fn add_read_time(&mut self, ms: u128) {
        self.read_times.push(ms);
    }

    /// Adiciona um tempo de LLM
    pub fn add_llm_time(&mut self, ms: u128) {
        self.llm_times.push(ms);
    }

    /// Adiciona um tempo de avaliação
    pub fn add_eval_time(&mut self, ms: u128) {
        self.eval_times.push(ms);
    }

    /// Calcula média de uma lista de tempos
    fn avg(times: &[u128]) -> f64 {
        if times.is_empty() {
            0.0
        } else {
            times.iter().sum::<u128>() as f64 / times.len() as f64
        }
    }

    /// Retorna média de tempo de busca
    pub fn avg_search_time(&self) -> f64 {
        Self::avg(&self.search_times)
    }

    /// Retorna média de tempo de leitura
    pub fn avg_read_time(&self) -> f64 {
        Self::avg(&self.read_times)
    }

    /// Retorna média de tempo de LLM
    pub fn avg_llm_time(&self) -> f64 {
        Self::avg(&self.llm_times)
    }

    /// Retorna tempo total
    pub fn total_time(&self) -> u128 {
        self.search_times.iter().sum::<u128>()
            + self.read_times.iter().sum::<u128>()
            + self.llm_times.iter().sum::<u128>()
            + self.eval_times.iter().sum::<u128>()
    }

    /// Formata um resumo das estatísticas
    pub fn summary(&self) -> String {
        format!(
            "Timing Stats:\n\
             - Search: {} calls, avg {:.1}ms, total {}ms\n\
             - Read: {} calls, avg {:.1}ms, total {}ms\n\
             - LLM: {} calls, avg {:.1}ms, total {}ms\n\
             - Eval: {} calls, avg {:.1}ms, total {}ms\n\
             - Total: {}ms",
            self.search_times.len(),
            self.avg_search_time(),
            self.search_times.iter().sum::<u128>(),
            self.read_times.len(),
            self.avg_read_time(),
            self.read_times.iter().sum::<u128>(),
            self.llm_times.len(),
            self.avg_llm_time(),
            self.llm_times.iter().sum::<u128>(),
            self.eval_times.len(),
            Self::avg(&self.eval_times),
            self.eval_times.iter().sum::<u128>(),
            self.total_time()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_action_timer() {
        let timer = ActionTimer::start("test");
        sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_timing_stats() {
        let mut stats = TimingStats::new();
        stats.add_search_time(100);
        stats.add_search_time(200);
        stats.add_read_time(50);

        assert_eq!(stats.avg_search_time(), 150.0);
        assert_eq!(stats.avg_read_time(), 50.0);
        assert_eq!(stats.total_time(), 350);
    }
}
