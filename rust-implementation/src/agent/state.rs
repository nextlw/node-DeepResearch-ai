// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ESTADOS DO AGENTE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::agent::interaction::QuestionType;
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

    /// Aguardando input do usuário
    ///
    /// Compatível com OpenAI Responses API (input_required state).
    /// O agente pausou a execução e está esperando que o usuário
    /// responda a uma pergunta antes de continuar.
    InputRequired {
        /// ID único da pergunta pendente
        question_id: String,
        /// Texto da pergunta sendo feita
        question: String,
        /// Tipo da pergunta
        question_type: QuestionType,
        /// Opções de resposta (se aplicável)
        options: Option<Vec<String>>,
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
        matches!(
            self,
            AgentState::Completed { .. } | AgentState::Failed { .. }
        )
    }

    /// Verifica se o estado é de processamento normal
    pub fn is_processing(&self) -> bool {
        matches!(self, AgentState::Processing { .. })
    }

    /// Verifica se o estado é beast mode
    pub fn is_beast_mode(&self) -> bool {
        matches!(self, AgentState::BeastMode { .. })
    }

    /// Verifica se o estado é de aguardando input do usuário
    ///
    /// Compatível com OpenAI Responses API (input_required)
    pub fn is_input_required(&self) -> bool {
        matches!(self, AgentState::InputRequired { .. })
    }

    /// Verifica se o agente está pausado esperando algo
    pub fn is_waiting(&self) -> bool {
        self.is_input_required()
    }

    /// Verifica se uma transição é válida
    pub fn can_transition_to(&self, target: &AgentState) -> bool {
        matches!(
            (self, target),
            // De Processing pode ir para BeastMode, Completed, Failed ou InputRequired
            (AgentState::Processing { .. }, AgentState::BeastMode { .. }) |
            (AgentState::Processing { .. }, AgentState::Completed { .. }) |
            (AgentState::Processing { .. }, AgentState::Failed { .. }) |
            (AgentState::Processing { .. }, AgentState::InputRequired { .. }) |
            // De BeastMode pode ir para Completed, Failed ou InputRequired
            (AgentState::BeastMode { .. }, AgentState::Completed { .. }) |
            (AgentState::BeastMode { .. }, AgentState::Failed { .. }) |
            (AgentState::BeastMode { .. }, AgentState::InputRequired { .. }) |
            // De InputRequired pode voltar para Processing (após receber resposta)
            (AgentState::InputRequired { .. }, AgentState::Processing { .. }) |
            (AgentState::InputRequired { .. }, AgentState::BeastMode { .. }) |
            (AgentState::InputRequired { .. }, AgentState::Completed { .. }) |
            (AgentState::InputRequired { .. }, AgentState::Failed { .. })
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
    /// Aguardando input do usuário (blocking)
    ///
    /// Compatível com OpenAI Responses API (input_required).
    /// O agente pausou e está esperando resposta do usuário.
    InputRequired {
        /// ID da pergunta pendente
        question_id: String,
        /// Texto da pergunta
        question: String,
        /// Tipo da pergunta
        question_type: QuestionType,
        /// Opções de resposta (se aplicável)
        options: Option<Vec<String>>,
    },
}

/// Resultado de uma resposta que foi aceita pela avaliação.
///
/// Quando o agente propõe uma resposta (via `AgentAction::Answer`) e ela
/// passa por todas as avaliações configuradas, um `AnswerResult` é criado
/// contendo a resposta final e suas referências.
///
/// # Campos
/// - `answer`: O texto completo da resposta gerada
/// - `references`: Lista de fontes citadas na resposta
/// - `trivial`: Se `true`, a pergunta foi respondida no primeiro passo
#[derive(Debug, Clone)]
pub struct AnswerResult {
    /// Texto completo da resposta gerada pelo agente.
    ///
    /// Esta é a resposta final que será apresentada ao usuário,
    /// já formatada e revisada pelas avaliações.
    pub answer: String,

    /// Lista de referências (fontes) citadas na resposta.
    ///
    /// Cada referência contém URL, título e opcionalmente uma
    /// citação exata do trecho utilizado.
    pub references: Vec<Reference>,

    /// Indica se a pergunta foi considerada trivial.
    ///
    /// Uma pergunta trivial é aquela que o agente conseguiu
    /// responder no primeiro passo, sem precisar de pesquisa
    /// adicional. Exemplo: "Quanto é 2+2?"
    pub trivial: bool,
}

/// Resultado final completo de uma sessão de pesquisa.
///
/// Esta struct contém tudo que aconteceu durante a execução do agente:
/// se teve sucesso, a resposta gerada, referências, uso de tokens, etc.
///
/// É o retorno principal do método `DeepResearchAgent::research`.
///
/// # Exemplo de Uso
/// ```rust,ignore
/// let result = agent.research("Pergunta aqui").await;
///
/// if result.success {
///     println!("Resposta: {}", result.answer.unwrap());
///     println!("Fontes: {:?}", result.references);
/// } else {
///     println!("Erro: {}", result.error.unwrap_or_default());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ResearchResult {
    /// Indica se a pesquisa foi concluída com sucesso.
    ///
    /// `true` = resposta encontrada e aprovada nas avaliações.
    /// `false` = falha por timeout, budget esgotado ou erro.
    pub success: bool,

    /// Resposta final gerada, se houver.
    ///
    /// Será `Some(texto)` quando `success = true`.
    /// Pode ser `None` se a pesquisa falhou.
    pub answer: Option<String>,

    /// Lista de referências citadas na resposta.
    ///
    /// Contém URLs, títulos e citações das fontes utilizadas.
    /// Vazia se não houve resposta ou nenhuma fonte foi citada.
    pub references: Vec<Reference>,

    /// Indica se foi uma pergunta trivial (respondida no step 1).
    ///
    /// Perguntas triviais são aquelas que não precisaram de
    /// pesquisa externa para serem respondidas.
    pub trivial: bool,

    /// Estatísticas de uso de tokens durante a pesquisa.
    ///
    /// Útil para monitorar custos e otimizar prompts.
    pub token_usage: TokenUsage,

    /// Lista de todas as URLs visitadas durante a pesquisa.
    ///
    /// Inclui URLs que foram lidas com sucesso e também
    /// aquelas que falharam (para evitar revisitas).
    pub visited_urls: Vec<String>,

    /// Mensagem de erro, se a pesquisa falhou.
    ///
    /// Será `Some(mensagem)` quando `success = false`.
    /// Descreve o motivo da falha (timeout, budget, etc).
    pub error: Option<String>,

    /// Tempo total da pesquisa em milissegundos.
    pub total_time_ms: u128,

    /// Tempo gasto em buscas em milissegundos.
    pub search_time_ms: u128,

    /// Tempo gasto em leitura de URLs em milissegundos.
    pub read_time_ms: u128,

    /// Tempo gasto em chamadas LLM em milissegundos.
    pub llm_time_ms: u128,
}

/// Estatísticas de uso de tokens durante a pesquisa.
///
/// Tokens são a unidade de medida usada por LLMs para cobrar pelo uso.
/// Grosso modo, 1 token ≈ 4 caracteres em inglês (menos em português).
///
/// # Por que isso importa?
/// - **Custo**: APIs cobram por token (ex: GPT-4 = $0.03/1K tokens)
/// - **Limite**: Cada modelo tem um limite máximo de tokens por chamada
/// - **Otimização**: Monitorar uso ajuda a otimizar prompts
///
/// # Exemplo
/// ```rust
/// use deep_research::agent::TokenUsage;
///
/// let usage = TokenUsage {
///     prompt_tokens: 1500,      // Tokens enviados ao modelo
///     completion_tokens: 500,    // Tokens gerados pelo modelo
///     total_tokens: 2000,        // Soma total
/// };
///
/// let custo_estimado = (usage.total_tokens as f64) * 0.00003; // GPT-4
/// println!("Custo: ${:.4}", custo_estimado);
/// ```
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// Tokens consumidos pelos prompts enviados ao modelo.
    ///
    /// Inclui: system prompt + user prompt + histórico.
    /// Quanto maior o contexto, mais tokens de prompt.
    pub prompt_tokens: u64,

    /// Tokens gerados pelo modelo nas respostas.
    ///
    /// Inclui: respostas, ações decididas, avaliações.
    /// Geralmente menor que prompt_tokens.
    pub completion_tokens: u64,

    /// Total de tokens usados (prompt + completion).
    ///
    /// Este é o valor usado para calcular custos e
    /// verificar se está dentro do budget.
    pub total_tokens: u64,
}

/// Erros que podem ocorrer durante a execução do agente.
///
/// Estes são os tipos de falha que podem interromper uma pesquisa.
/// Cada variante representa uma categoria diferente de problema.
///
/// # Tratamento de Erros
/// O agente tenta ser resiliente a erros temporários:
/// - Rate limits: aguarda e tenta novamente
/// - Erros de rede: retry com backoff exponencial
/// - Budget esgotado: entra em BeastMode antes de falhar
#[derive(Debug, Clone)]
pub enum AgentError {
    /// Erro na comunicação com o LLM (OpenAI, Anthropic, etc).
    ///
    /// Pode ser causado por: API key inválida, rate limit, resposta
    /// malformada, modelo indisponível, etc.
    ///
    /// A string contém detalhes do erro para debugging.
    LlmError(String),

    /// Erro durante operações de busca ou leitura de URLs.
    ///
    /// Pode ser causado por: URL inválida, site bloqueando acesso,
    /// timeout de conexão, conteúdo não extraível, etc.
    ///
    /// A string contém detalhes do erro.
    SearchError(String),

    /// Timeout geral da operação.
    ///
    /// A pesquisa excedeu o tempo máximo configurado.
    /// Diferente de timeouts individuais de requisições.
    TimeoutError,

    /// Budget de tokens foi completamente esgotado.
    ///
    /// Isso significa que mesmo após entrar em BeastMode
    /// (85% do budget), o agente não conseguiu gerar uma
    /// resposta aceitável antes de esgotar os tokens.
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
