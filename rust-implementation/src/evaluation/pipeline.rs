// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PIPELINE DE AVALIAÇÃO
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::sync::Arc;

use super::{
    EvaluationType, EvaluationResult, EvaluationContext, EvalError, PromptPair,
};
use crate::llm::LlmClient;

/// Resultado do pipeline de avaliação
#[derive(Debug)]
pub struct EvaluationPipelineResult {
    /// Se todas as avaliações passaram
    pub overall_passed: bool,
    /// Resultados individuais de cada avaliação
    pub results: Vec<EvaluationResult>,
    /// Tipo de avaliação onde falhou (se aplicável)
    pub failed_at: Option<EvaluationType>,
}

impl EvaluationPipelineResult {
    /// Cria um resultado de sucesso
    pub fn success(results: Vec<EvaluationResult>) -> Self {
        Self {
            overall_passed: true,
            results,
            failed_at: None,
        }
    }

    /// Cria um resultado de falha
    pub fn failure(results: Vec<EvaluationResult>, failed_at: EvaluationType) -> Self {
        Self {
            overall_passed: false,
            results,
            failed_at: Some(failed_at),
        }
    }

    /// Retorna o motivo da falha formatado
    pub fn failure_reason(&self) -> Option<String> {
        self.results
            .last()
            .filter(|r| !r.passed)
            .map(|r| format!("{}: {}", r.eval_type, r.reasoning))
    }

    /// Retorna sugestões de melhoria
    pub fn all_suggestions(&self) -> Vec<String> {
        self.results
            .iter()
            .flat_map(|r| r.suggestions.clone())
            .collect()
    }
}

/// Pipeline de avaliação multidimensional
///
/// Executa múltiplas avaliações em sequência com falha rápida:
/// se uma avaliação falha, as próximas não são executadas.
///
/// # Exemplo
///
/// ```rust
/// let pipeline = EvaluationPipeline::new(llm_client);
/// let result = pipeline.evaluate_sequential(
///     "What is Rust?",
///     "Rust is a systems programming language...",
///     &context,
///     &[EvaluationType::Definitive, EvaluationType::Completeness],
/// ).await;
///
/// if result.overall_passed {
///     println!("Answer accepted!");
/// } else {
///     println!("Failed at: {:?}", result.failed_at);
/// }
/// ```
pub struct EvaluationPipeline {
    llm: Arc<dyn LlmClient>,
}

impl EvaluationPipeline {
    /// Cria um novo pipeline com o cliente LLM fornecido
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self { llm }
    }

    /// Executa avaliações em sequência - FALHA RÁPIDA
    ///
    /// Retorna no primeiro erro para economizar tokens.
    /// Esta é a estratégia mais eficiente quando qualquer falha
    /// invalida a resposta.
    pub async fn evaluate_sequential(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
        required_types: &[EvaluationType],
    ) -> EvaluationPipelineResult {
        let mut results = Vec::new();

        for &eval_type in required_types {
            let result = self.evaluate_single(eval_type, question, answer, context).await;

            match result {
                Ok(eval_result) => {
                    let passed = eval_result.passed;
                    results.push(eval_result);

                    // FALHA RÁPIDA - retorna imediatamente se falhou
                    if !passed {
                        return EvaluationPipelineResult::failure(results, eval_type);
                    }
                }
                Err(_) => {
                    // Erro de avaliação = falha
                    results.push(EvaluationResult::failure(
                        eval_type,
                        "Evaluation error".into(),
                        vec!["Retry evaluation".into()],
                        0.0,
                    ));
                    return EvaluationPipelineResult::failure(results, eval_type);
                }
            }
        }

        // Todas passaram
        EvaluationPipelineResult::success(results)
    }

    /// Executa uma única avaliação
    async fn evaluate_single(
        &self,
        eval_type: EvaluationType,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvalError> {
        let start = std::time::Instant::now();

        let _prompt = self.generate_prompt(eval_type, question, answer, context);

        // Simula chamada ao LLM (em produção, chamaria a API real)
        let response = self.llm
            .evaluate(question, answer, &eval_type.as_str())
            .await
            .map_err(|e| EvalError::LlmError(e.to_string()))?;

        Ok(EvaluationResult {
            eval_type,
            passed: response.passed,
            confidence: response.confidence,
            reasoning: response.reasoning,
            suggestions: vec![],  // TODO: adicionar sugestões na EvaluationResponse
            duration: start.elapsed(),
        })
    }

    /// Gera o prompt para um tipo específico de avaliação
    fn generate_prompt(
        &self,
        eval_type: EvaluationType,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> PromptPair {
        match eval_type {
            EvaluationType::Definitive => self.definitive_prompt(question, answer),
            EvaluationType::Freshness => self.freshness_prompt(question, answer, context),
            EvaluationType::Plurality => self.plurality_prompt(question, answer),
            EvaluationType::Completeness => self.completeness_prompt(question, answer),
            EvaluationType::Strict => self.strict_prompt(question, answer, context),
        }
    }

    fn definitive_prompt(&self, question: &str, answer: &str) -> PromptPair {
        PromptPair {
            system: r#"
You are an evaluator checking if an answer is DEFINITIVE.
A definitive answer:
- States facts confidently without excessive hedging
- Does not use phrases like "I think", "maybe", "probably", "might be"
- Provides concrete information rather than vague generalities
- Acknowledges uncertainty only when genuinely uncertain, not as a habit

Evaluate the answer and respond with:
- passed: boolean
- confidence: float 0-1
- reasoning: string explaining your evaluation
- suggestions: array of improvement suggestions (if failed)
"#.into(),
            user: format!("Question: {}\n\nAnswer to evaluate:\n{}", question, answer),
        }
    }

    fn freshness_prompt(&self, question: &str, answer: &str, context: &EvaluationContext) -> PromptPair {
        let threshold = EvaluationType::Freshness.freshness_threshold(&context.topic);
        let days = threshold.as_secs() / 86400;

        PromptPair {
            system: format!(r#"
You are evaluating if an answer contains sufficiently RECENT information.
Topic category: {:?}
Required freshness: information should not be older than {} days

Check if:
1. The answer mentions dates/timeframes that are recent enough
2. The information reflects current state (not outdated)
3. For time-sensitive topics, data is from recent sources

Respond with:
- passed: boolean
- confidence: float 0-1
- reasoning: string
- suggestions: array (if failed)
- detected_date: string (if found)
"#, context.topic, days),
            user: format!("Question: {}\n\nAnswer to evaluate:\n{}", question, answer),
        }
    }

    fn plurality_prompt(&self, question: &str, answer: &str) -> PromptPair {
        PromptPair {
            system: r#"
Count the number of distinct items/examples in the answer.
If the question asks for a specific number (e.g., "5 examples", "top 10"),
verify the answer provides at least that many.

Respond with:
- passed: boolean
- confidence: float 0-1
- reasoning: string
- suggestions: array (if failed)
- item_count: integer
- expected_count: integer (if specified in question)
"#.into(),
            user: format!("Question: {}\n\nAnswer to evaluate:\n{}", question, answer),
        }
    }

    fn completeness_prompt(&self, question: &str, answer: &str) -> PromptPair {
        PromptPair {
            system: r#"
Evaluate if the answer addresses ALL aspects of the question.

First, identify the aspects/sub-questions in the question.
Then check if each aspect is adequately addressed.

Respond with:
- passed: boolean (true if >= 80% coverage)
- confidence: float 0-1
- reasoning: string
- suggestions: array (if failed)
- aspects_found: array of strings
- aspects_covered: array of strings
- coverage_ratio: float 0-1
"#.into(),
            user: format!("Question: {}\n\nAnswer to evaluate:\n{}", question, answer),
        }
    }

    fn strict_prompt(&self, question: &str, answer: &str, context: &EvaluationContext) -> PromptPair {
        let knowledge_summary = context.knowledge_items
            .iter()
            .take(5)
            .map(|k| format!("- {}: {}", k.question, k.answer.chars().take(100).collect::<String>()))
            .collect::<Vec<_>>()
            .join("\n");

        PromptPair {
            system: r#"
You are a BRUTAL evaluator. Your job is to REJECT mediocre answers.

An answer ONLY passes if it demonstrates:
1. DEPTH: Goes beyond surface-level information
2. INSIGHT: Provides non-obvious analysis or connections
3. SPECIFICITY: Includes concrete examples, numbers, or evidence
4. COMPLETENESS: Addresses the full scope of the question
5. ACCURACY: No factual errors or misleading statements

Be harsh. Most answers should FAIL.
If the answer is just "good enough", it FAILS.
Only truly excellent, insightful answers should pass.

Respond with:
- passed: boolean
- confidence: float 0-1
- reasoning: string (detailed explanation)
- suggestions: array (specific improvements needed)
- quality_score: float 0-1
"#.into(),
            user: format!(
                "Question: {}\n\nAnswer to evaluate:\n{}\n\nKnowledge base used:\n{}",
                question, answer, knowledge_summary
            ),
        }
    }

    /// Determina quais avaliações são necessárias para uma pergunta
    pub async fn determine_required_evaluations(
        &self,
        question: &str,
        llm: &dyn LlmClient,
    ) -> Vec<EvaluationType> {
        let _prompt = PromptPair {
            system: r#"
Analyze the question and determine which evaluation types are needed:
- definitive: Does this question have a clear factual answer?
- freshness: Is time-sensitive information relevant?
- plurality: Does it ask for multiple items/examples?
- completeness: Does it have multiple sub-questions or aspects?

Respond with:
- needs_definitive: boolean
- needs_freshness: boolean
- needs_plurality: boolean
- needs_completeness: boolean
- reasoning: string
"#.into(),
            user: format!("Question: {}", question),
        };

        // Tenta determinar via LLM, com fallback para default
        match llm.determine_eval_types(question).await {
            Ok(types) => types,
            Err(_) => {
                // Default: apenas definitive
                vec![EvaluationType::Definitive]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_result_success() {
        let results = vec![
            EvaluationResult::success(EvaluationType::Definitive, "Good".into(), 0.9),
            EvaluationResult::success(EvaluationType::Completeness, "Complete".into(), 0.85),
        ];

        let pipeline_result = EvaluationPipelineResult::success(results);
        assert!(pipeline_result.overall_passed);
        assert!(pipeline_result.failed_at.is_none());
        assert!(pipeline_result.failure_reason().is_none());
    }

    #[test]
    fn test_pipeline_result_failure() {
        let results = vec![
            EvaluationResult::success(EvaluationType::Definitive, "Good".into(), 0.9),
            EvaluationResult::failure(
                EvaluationType::Freshness,
                "Outdated info".into(),
                vec!["Update data".into()],
                0.3,
            ),
        ];

        let pipeline_result = EvaluationPipelineResult::failure(results, EvaluationType::Freshness);
        assert!(!pipeline_result.overall_passed);
        assert_eq!(pipeline_result.failed_at, Some(EvaluationType::Freshness));
        assert!(pipeline_result.failure_reason().is_some());
    }

    #[test]
    fn test_all_suggestions() {
        let results = vec![
            EvaluationResult::failure(
                EvaluationType::Definitive,
                "Hedging".into(),
                vec!["Be more confident".into()],
                0.4,
            ),
            EvaluationResult::failure(
                EvaluationType::Completeness,
                "Missing aspects".into(),
                vec!["Address X".into(), "Address Y".into()],
                0.5,
            ),
        ];

        let pipeline_result = EvaluationPipelineResult::failure(results, EvaluationType::Definitive);
        let suggestions = pipeline_result.all_suggestions();

        assert_eq!(suggestions.len(), 3);
        assert!(suggestions.contains(&"Be more confident".to_string()));
    }
}
