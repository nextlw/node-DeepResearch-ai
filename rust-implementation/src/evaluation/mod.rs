// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SISTEMA DE AVALIAÇÃO MULTIDIMENSIONAL
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod pipeline;

pub use pipeline::*;

use std::time::Duration;

use crate::types::{KnowledgeItem, TopicCategory};

/// Contexto para avaliação
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Categoria do tópico
    pub topic: TopicCategory,
    /// Itens de conhecimento acumulados
    pub knowledge_items: Vec<KnowledgeItem>,
}

/// Par de prompts para enviar ao LLM (sistema + usuário).
///
/// Em APIs de LLM como OpenAI, as mensagens são divididas em:
/// - **System**: Define o comportamento e contexto do modelo
/// - **User**: A pergunta ou tarefa específica
///
/// Este struct agrupa ambos para facilitar o gerenciamento.
#[derive(Debug, Clone)]
pub struct PromptPair {
    /// Prompt de sistema que define o comportamento do LLM.
    ///
    /// Exemplo: "Você é um avaliador rigoroso de respostas..."
    pub system: String,
    /// Prompt do usuário com a tarefa específica.
    ///
    /// Exemplo: "Avalie se esta resposta está completa: ..."
    pub user: String,
}

/// Tipos de avaliação - enum garante que não existem tipos "inventados"
///
/// Cada tipo representa uma dimensão diferente de qualidade da resposta:
/// - Definitive: A resposta é confiante, sem hedging excessivo?
/// - Freshness: A informação é recente o suficiente para o tópico?
/// - Plurality: Se pediu N exemplos, tem N exemplos?
/// - Completeness: Todos os aspectos da pergunta foram cobertos?
/// - Strict: Avaliação brutal - tem insights reais e profundos?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvaluationType {
    /// Verifica se a resposta é confiante e definitiva
    Definitive,
    /// Verifica se a informação é recente o suficiente
    Freshness,
    /// Verifica se a quantidade de itens está correta
    Plurality,
    /// Verifica se todos os aspectos foram cobertos
    Completeness,
    /// Avaliação brutal - rejeita respostas mediocres
    Strict,
}

impl EvaluationType {
    /// Retorna o nome do tipo como string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Definitive => "definitive",
            Self::Freshness => "freshness",
            Self::Plurality => "plurality",
            Self::Completeness => "completeness",
            Self::Strict => "strict",
        }
    }

    /// Retorna configuração padrão para cada tipo
    pub fn default_config(self) -> EvaluationConfig {
        match self {
            Self::Definitive => EvaluationConfig {
                eval_type: self,
                max_retries: 2,
                timeout: Duration::from_secs(30),
                weight: 1.0,
            },
            Self::Freshness => EvaluationConfig {
                eval_type: self,
                max_retries: 1,
                timeout: Duration::from_secs(20),
                weight: 0.8,
            },
            Self::Plurality => EvaluationConfig {
                eval_type: self,
                max_retries: 1,
                timeout: Duration::from_secs(15),
                weight: 0.6,
            },
            Self::Completeness => EvaluationConfig {
                eval_type: self,
                max_retries: 2,
                timeout: Duration::from_secs(25),
                weight: 0.9,
            },
            Self::Strict => EvaluationConfig {
                eval_type: self,
                max_retries: 3,
                timeout: Duration::from_secs(45),
                weight: 1.5, // Mais importante
            },
        }
    }

    /// Determina freshness threshold baseado no tópico
    pub fn freshness_threshold(&self, topic: &TopicCategory) -> Duration {
        match topic {
            TopicCategory::Finance => Duration::from_secs(60 * 60 * 2), // 2 horas
            TopicCategory::News => Duration::from_secs(60 * 60 * 24),   // 1 dia
            TopicCategory::Technology => Duration::from_secs(60 * 60 * 24 * 30), // 30 dias
            TopicCategory::Science => Duration::from_secs(60 * 60 * 24 * 365), // 1 ano
            TopicCategory::History => Duration::MAX,                    // Sem limite
            _ => Duration::from_secs(60 * 60 * 24 * 7),                 // 7 dias padrão
        }
    }
}

impl std::fmt::Display for EvaluationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuração específica para cada tipo de avaliação.
///
/// Permite ajustar o comportamento de cada avaliação:
/// - Quantas vezes tentar em caso de falha
/// - Quanto tempo esperar antes de timeout
/// - Qual o peso relativo desta avaliação
///
/// Use [`EvaluationType::default_config`] para valores padrão.
#[derive(Debug, Clone)]
pub struct EvaluationConfig {
    /// Tipo de avaliação que esta configuração controla.
    pub eval_type: EvaluationType,
    /// Número máximo de tentativas em caso de erro.
    ///
    /// Se a avaliação falhar por erro de rede ou parsing,
    /// será reexecutada até `max_retries` vezes.
    pub max_retries: u8,
    /// Tempo máximo para aguardar resposta do LLM.
    ///
    /// Após este tempo, a avaliação é considerada falha por timeout.
    pub timeout: Duration,
    /// Peso relativo desta avaliação (0.0 a 2.0).
    ///
    /// Avaliações com peso maior têm mais influência na
    /// decisão final. Ex: Strict tem peso 1.5 (mais importante).
    pub weight: f32,
}

/// Resultado de uma avaliação individual.
///
/// Contém não apenas se passou ou não, mas também:
/// - Nível de confiança do avaliador
/// - Explicação do raciocínio
/// - Sugestões de melhoria (se falhou)
/// - Tempo que levou para avaliar
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// Tipo de avaliação que gerou este resultado.
    pub eval_type: EvaluationType,
    /// Se a resposta passou nesta avaliação.
    ///
    /// `true` = aprovado, `false` = reprovado.
    pub passed: bool,
    /// Nível de confiança do avaliador (0.0 a 1.0).
    ///
    /// - 0.9-1.0: Muito confiante
    /// - 0.7-0.9: Confiante
    /// - 0.5-0.7: Incerto
    /// - <0.5: Pouco confiante
    pub confidence: f32,
    /// Explicação do raciocínio usado na avaliação.
    ///
    /// Útil para debugging e para mostrar ao usuário
    /// por que uma resposta foi aceita ou rejeitada.
    pub reasoning: String,
    /// Sugestões de como melhorar a resposta.
    ///
    /// Preenchido apenas quando `passed = false`.
    /// O agente pode usar estas sugestões para ajustar.
    pub suggestions: Vec<String>,
    /// Tempo que a avaliação levou para executar.
    ///
    /// Útil para monitorar performance e identificar
    /// avaliações que estão demorando muito.
    pub duration: Duration,
}

impl EvaluationResult {
    /// Cria um resultado de sucesso
    pub fn success(eval_type: EvaluationType, reasoning: String, confidence: f32) -> Self {
        Self {
            eval_type,
            passed: true,
            confidence,
            reasoning,
            suggestions: vec![],
            duration: Duration::ZERO,
        }
    }

    /// Cria um resultado de falha
    pub fn failure(
        eval_type: EvaluationType,
        reasoning: String,
        suggestions: Vec<String>,
        confidence: f32,
    ) -> Self {
        Self {
            eval_type,
            passed: false,
            confidence,
            reasoning,
            suggestions,
            duration: Duration::ZERO,
        }
    }

    /// Define a duração da avaliação
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

/// Erro que pode ocorrer durante uma avaliação.
///
/// Diferente de uma avaliação que retorna `passed = false`
/// (a avaliação funcionou, mas a resposta não passou),
/// estes são erros técnicos na execução da avaliação.
#[derive(Debug, Clone)]
pub enum EvalError {
    /// Erro na comunicação com o LLM.
    ///
    /// Pode ser: API indisponível, rate limit, resposta inválida, etc.
    /// A string contém detalhes do erro.
    LlmError(String),
    /// A avaliação excedeu o tempo limite configurado.
    ///
    /// O LLM demorou demais para responder.
    /// Considere aumentar o timeout em [`EvaluationConfig`].
    Timeout,
    /// Erro ao interpretar a resposta do LLM.
    ///
    /// O LLM retornou algo que não conseguimos parsear
    /// para o formato esperado de avaliação.
    ParseError(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LlmError(msg) => write!(f, "LLM error: {}", msg),
            Self::Timeout => write!(f, "Evaluation timeout"),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for EvalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluation_type_as_str() {
        assert_eq!(EvaluationType::Definitive.as_str(), "definitive");
        assert_eq!(EvaluationType::Freshness.as_str(), "freshness");
        assert_eq!(EvaluationType::Plurality.as_str(), "plurality");
        assert_eq!(EvaluationType::Completeness.as_str(), "completeness");
        assert_eq!(EvaluationType::Strict.as_str(), "strict");
    }

    #[test]
    fn test_default_configs() {
        let definitive = EvaluationType::Definitive.default_config();
        assert_eq!(definitive.weight, 1.0);
        assert_eq!(definitive.max_retries, 2);

        let strict = EvaluationType::Strict.default_config();
        assert_eq!(strict.weight, 1.5);
        assert_eq!(strict.max_retries, 3);
    }

    #[test]
    fn test_freshness_threshold() {
        let eval = EvaluationType::Freshness;

        let finance = eval.freshness_threshold(&TopicCategory::Finance);
        assert_eq!(finance.as_secs(), 60 * 60 * 2); // 2 horas

        let history = eval.freshness_threshold(&TopicCategory::History);
        assert_eq!(history, Duration::MAX);
    }

    #[test]
    fn test_evaluation_result() {
        let success =
            EvaluationResult::success(EvaluationType::Definitive, "Good answer".into(), 0.9);
        assert!(success.passed);
        assert_eq!(success.confidence, 0.9);

        let failure = EvaluationResult::failure(
            EvaluationType::Freshness,
            "Outdated".into(),
            vec!["Update data".into()],
            0.3,
        );
        assert!(!failure.passed);
        assert_eq!(failure.suggestions.len(), 1);
    }
}
