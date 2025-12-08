// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ESTADOS DO AGENTE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::types::{KnowledgeItem, Reference};

/// Estado do agente - transições explícitas
///
/// A máquina de estados garante que o agente só pode estar em um estado válido.
/// Pattern matching exaustivo força o tratamento de todos os casos.
#[derive(Debug, Clone)]
pub enum AgentState {
    /// Processando normalmente
    ///
    /// O agente está no loop principal, executando ações de pesquisa.
    Processing {
        /// Passo atual dentro da pergunta atual
        step: u32,
        /// Passo total desde o início
        total_step: u32,
        /// Pergunta sendo processada atualmente
        current_question: String,
        /// Porcentagem do budget utilizado (0.0 - 1.0)
        budget_used: f64,
    },

    /// Modo de emergência - forçar resposta
    ///
    /// Ativado quando 85% do budget foi consumido sem resposta aceita.
    /// Desabilita SEARCH e REFLECT, força ANSWER.
    BeastMode {
        /// Número de tentativas em beast mode
        attempts: u32,
        /// Motivo da última falha
        last_failure: String,
    },

    /// Pesquisa concluída com sucesso
    ///
    /// Estado terminal - resposta foi aceita por todas as avaliações.
    Completed {
        /// Resposta final
        answer: String,
        /// Referências utilizadas
        references: Vec<Reference>,
        /// Se foi uma pergunta trivial (respondida no step 1)
        trivial: bool,
    },

    /// Falha - budget esgotado sem resposta
    ///
    /// Estado terminal - não foi possível encontrar uma resposta satisfatória.
    Failed {
        /// Motivo da falha
        reason: String,
        /// Conhecimento parcial acumulado
        partial_knowledge: Vec<KnowledgeItem>,
    },
}

impl AgentState {
    /// Verifica se o estado é terminal (Completed ou Failed)
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Completed { .. } | AgentState::Failed { .. })
    }

    /// Verifica se o estado é de processamento normal
    pub fn is_processing(&self) -> bool {
        matches!(self, AgentState::Processing { .. })
    }

    /// Verifica se o estado é beast mode
    pub fn is_beast_mode(&self) -> bool {
        matches!(self, AgentState::BeastMode { .. })
    }

    /// Verifica se uma transição é válida
    pub fn can_transition_to(&self, target: &AgentState) -> bool {
        matches!(
            (self, target),
            // De Processing pode ir para BeastMode, Completed ou Failed
            (AgentState::Processing { .. }, AgentState::BeastMode { .. }) |
            (AgentState::Processing { .. }, AgentState::Completed { .. }) |
            (AgentState::Processing { .. }, AgentState::Failed { .. }) |
            // De BeastMode pode ir para Completed ou Failed
            (AgentState::BeastMode { .. }, AgentState::Completed { .. }) |
            (AgentState::BeastMode { .. }, AgentState::Failed { .. })
            // Estados terminais não podem transicionar
        )
    }

    /// Retorna o budget usado, se aplicável
    pub fn budget_used(&self) -> Option<f64> {
        match self {
            AgentState::Processing { budget_used, .. } => Some(*budget_used),
            _ => None,
        }
    }

    /// Retorna o step total, se aplicável
    pub fn total_step(&self) -> Option<u32> {
        match self {
            AgentState::Processing { total_step, .. } => Some(*total_step),
            _ => None,
        }
    }
}

/// Resultado de um passo de execução
#[derive(Debug)]
pub enum StepResult {
    /// Continuar processamento
    Continue,
    /// Pesquisa concluída com sucesso
    Completed(AnswerResult),
    /// Erro durante execução
    Error(String),
}

/// Resultado de uma resposta aceita
#[derive(Debug, Clone)]
pub struct AnswerResult {
    pub answer: String,
    pub references: Vec<Reference>,
    pub trivial: bool,
}

/// Resultado final da pesquisa
#[derive(Debug, Clone)]
pub struct ResearchResult {
    pub success: bool,
    pub answer: Option<String>,
    pub references: Vec<Reference>,
    pub trivial: bool,
    pub token_usage: TokenUsage,
    pub visited_urls: Vec<String>,
    pub error: Option<String>,
}

/// Uso de tokens
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Erros do agente
#[derive(Debug, Clone)]
pub enum AgentError {
    LlmError(String),
    SearchError(String),
    TimeoutError,
    BudgetExhausted,
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::LlmError(msg) => write!(f, "LLM error: {}", msg),
            AgentError::SearchError(msg) => write!(f, "Search error: {}", msg),
            AgentError::TimeoutError => write!(f, "Timeout error"),
            AgentError::BudgetExhausted => write!(f, "Token budget exhausted"),
        }
    }
}

impl std::error::Error for AgentError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        let processing = AgentState::Processing {
            step: 0,
            total_step: 0,
            current_question: "test".into(),
            budget_used: 0.0,
        };

        let beast_mode = AgentState::BeastMode {
            attempts: 0,
            last_failure: "test".into(),
        };

        let completed = AgentState::Completed {
            answer: "answer".into(),
            references: vec![],
            trivial: false,
        };

        // Transições válidas
        assert!(processing.can_transition_to(&beast_mode));
        assert!(processing.can_transition_to(&completed));
        assert!(beast_mode.can_transition_to(&completed));

        // Transições inválidas (de estado terminal)
        assert!(!completed.can_transition_to(&processing));
        assert!(!completed.can_transition_to(&beast_mode));
    }

    #[test]
    fn test_is_terminal() {
        let processing = AgentState::Processing {
            step: 0,
            total_step: 0,
            current_question: "test".into(),
            budget_used: 0.0,
        };

        let completed = AgentState::Completed {
            answer: "answer".into(),
            references: vec![],
            trivial: false,
        };

        let failed = AgentState::Failed {
            reason: "test".into(),
            partial_knowledge: vec![],
        };

        assert!(!processing.is_terminal());
        assert!(completed.is_terminal());
        assert!(failed.is_terminal());
    }
}
