// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TOKEN TRACKER
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Gerenciamento de budget de tokens para controle de custos.
// Suporta:
// - Tracking de tokens por operação
// - Budget limits
// - Alertas de threshold
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::agent::TokenUsage;

/// Budget padrão: 1 milhão de tokens
pub const DEFAULT_TOKEN_BUDGET: u64 = 1_000_000;

/// Threshold para beast mode: 85%
pub const BEAST_MODE_THRESHOLD: f64 = 0.85;

/// Tracker de uso de tokens
#[derive(Debug, Clone)]
pub struct TokenTracker {
    /// Budget total disponível
    budget: u64,

    /// Tokens de prompt utilizados
    prompt_tokens: u64,

    /// Tokens de completion utilizados
    completion_tokens: u64,

    /// Histórico de uso por step
    history: Vec<StepUsage>,
}

/// Uso de tokens em um step específico
#[derive(Debug, Clone)]
pub struct StepUsage {
    pub step: usize,
    pub operation: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

impl TokenTracker {
    /// Cria um novo tracker com budget opcional
    pub fn new(budget: Option<u64>) -> Self {
        Self {
            budget: budget.unwrap_or(DEFAULT_TOKEN_BUDGET),
            prompt_tokens: 0,
            completion_tokens: 0,
            history: Vec::new(),
        }
    }

    /// Registra uso de tokens
    pub fn track(&mut self, step: usize, operation: &str, prompt: u64, completion: u64) {
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;

        self.history.push(StepUsage {
            step,
            operation: operation.to_string(),
            prompt_tokens: prompt,
            completion_tokens: completion,
        });

        log::debug!(
            "Token usage: {} + {} = {} total ({:.1}% of budget)",
            prompt,
            completion,
            self.total_tokens(),
            self.budget_used_percentage() * 100.0
        );
    }

    /// Retorna tokens totais utilizados
    pub fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }

    /// Retorna porcentagem do budget utilizado (0.0 - 1.0)
    pub fn budget_used_percentage(&self) -> f64 {
        self.total_tokens() as f64 / self.budget as f64
    }

    /// Verifica se está em território de beast mode (>= 85%)
    pub fn should_enter_beast_mode(&self) -> bool {
        self.budget_used_percentage() >= BEAST_MODE_THRESHOLD
    }

    /// Verifica se ainda há budget disponível
    pub fn has_budget(&self) -> bool {
        self.total_tokens() < self.budget
    }

    /// Retorna tokens restantes
    pub fn remaining_tokens(&self) -> u64 {
        self.budget.saturating_sub(self.total_tokens())
    }

    /// Retorna uso total formatado
    pub fn get_total_usage(&self) -> TokenUsage {
        TokenUsage {
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.total_tokens(),
        }
    }

    /// Retorna estatísticas detalhadas
    pub fn stats(&self) -> TrackerStats {
        let total_steps = self.history.len();
        let avg_per_step = if total_steps > 0 {
            self.total_tokens() / total_steps as u64
        } else {
            0
        };

        TrackerStats {
            total_tokens: self.total_tokens(),
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            budget: self.budget,
            budget_used_percentage: self.budget_used_percentage(),
            total_steps,
            avg_tokens_per_step: avg_per_step,
            remaining_tokens: self.remaining_tokens(),
        }
    }

    /// Reseta o tracker mantendo o budget
    pub fn reset(&mut self) {
        self.prompt_tokens = 0;
        self.completion_tokens = 0;
        self.history.clear();
    }

    /// Ajusta o budget dinamicamente
    pub fn set_budget(&mut self, budget: u64) {
        self.budget = budget;
    }
}

/// Estatísticas detalhadas do tracker
#[derive(Debug, Clone)]
pub struct TrackerStats {
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub budget: u64,
    pub budget_used_percentage: f64,
    pub total_steps: usize,
    pub avg_tokens_per_step: u64,
    pub remaining_tokens: u64,
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let tracker = TokenTracker::new(None);
        assert_eq!(tracker.budget, DEFAULT_TOKEN_BUDGET);
        assert_eq!(tracker.total_tokens(), 0);
    }

    #[test]
    fn test_custom_budget() {
        let tracker = TokenTracker::new(Some(500_000));
        assert_eq!(tracker.budget, 500_000);
    }

    #[test]
    fn test_tracking() {
        let mut tracker = TokenTracker::new(Some(1000));

        tracker.track(1, "search", 100, 50);
        assert_eq!(tracker.total_tokens(), 150);
        assert_eq!(tracker.budget_used_percentage(), 0.15);

        tracker.track(2, "answer", 200, 100);
        assert_eq!(tracker.total_tokens(), 450);
        assert_eq!(tracker.budget_used_percentage(), 0.45);
    }

    #[test]
    fn test_beast_mode_threshold() {
        let mut tracker = TokenTracker::new(Some(1000));

        tracker.track(1, "op", 400, 400);
        assert!(!tracker.should_enter_beast_mode()); // 80%

        tracker.track(2, "op", 50, 0);
        assert!(tracker.should_enter_beast_mode()); // 85%
    }

    #[test]
    fn test_remaining_tokens() {
        let mut tracker = TokenTracker::new(Some(1000));
        tracker.track(1, "op", 300, 200);

        assert_eq!(tracker.remaining_tokens(), 500);
    }

    #[test]
    fn test_stats() {
        let mut tracker = TokenTracker::new(Some(1000));
        tracker.track(1, "op1", 100, 50);
        tracker.track(2, "op2", 100, 50);

        let stats = tracker.stats();
        assert_eq!(stats.total_tokens, 300);
        assert_eq!(stats.total_steps, 2);
        assert_eq!(stats.avg_tokens_per_step, 150);
    }

    #[test]
    fn test_reset() {
        let mut tracker = TokenTracker::new(Some(1000));
        tracker.track(1, "op", 100, 50);
        tracker.reset();

        assert_eq!(tracker.total_tokens(), 0);
        assert!(tracker.history.is_empty());
        assert_eq!(tracker.budget, 1000); // Budget mantido
    }
}
