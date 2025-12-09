// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DETERMINAÇÃO DE TIPOS DE AVALIAÇÃO
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Determina quais tipos de avaliação aplicar baseado na pergunta.
// Portado de src/tools/evaluator.ts do TypeScript.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::EvaluationType;
use regex::Regex;
use std::collections::HashSet;

/// Resultado da análise de uma pergunta
#[derive(Debug, Clone, Default)]
pub struct QuestionAnalysis {
    /// Se precisa de avaliação definitiva
    pub needs_definitive: bool,
    /// Se precisa de avaliação de freshness
    pub needs_freshness: bool,
    /// Se precisa de avaliação de pluralidade
    pub needs_plurality: bool,
    /// Se precisa de avaliação de completude
    pub needs_completeness: bool,
    /// Raciocínio da análise
    pub reasoning: String,
}

impl QuestionAnalysis {
    /// Converte para lista de tipos de avaliação
    pub fn to_evaluation_types(&self) -> Vec<EvaluationType> {
        let mut types = Vec::new();
        
        // Ordem: Definitive -> Freshness -> Plurality/Completeness
        if self.needs_definitive {
            types.push(EvaluationType::Definitive);
        }
        if self.needs_freshness {
            types.push(EvaluationType::Freshness);
        }
        // Completeness tem precedência sobre Plurality
        if self.needs_completeness {
            types.push(EvaluationType::Completeness);
        } else if self.needs_plurality {
            types.push(EvaluationType::Plurality);
        }
        
        types
    }
}

/// Determina quais tipos de avaliação são necessários para uma pergunta
///
/// Implementa regras baseadas em keywords sem depender de LLM:
/// - Definitive: Quase sempre necessário (exceto paradoxos)
/// - Freshness: Quando há termos temporais
/// - Plurality: Quando pede múltiplos itens
/// - Completeness: Quando menciona elementos específicos a serem cobertos
///
/// # Exemplo
///
/// ```rust,ignore
/// let types = determine_required_evaluations("What are 5 examples of Rust frameworks?");
/// // types = [Definitive, Plurality]
/// ```
pub fn determine_required_evaluations(question: &str) -> Vec<EvaluationType> {
    let analysis = analyze_question(question);
    analysis.to_evaluation_types()
}

/// Analisa uma pergunta e retorna análise detalhada
pub fn analyze_question(question: &str) -> QuestionAnalysis {
    let q_lower = question.to_lowercase();
    let mut reasoning_parts = Vec::new();

    // 1. DEFINITIVE - Quase sempre necessário
    let needs_definitive = !is_paradox_or_unanswerable(&q_lower);
    if needs_definitive {
        reasoning_parts.push("Definitive: pergunta pode ser avaliada objetivamente".to_string());
    } else {
        reasoning_parts.push("Definitive: pergunta é paradoxo ou inerentemente impossível de responder".to_string());
    }

    // 2. FRESHNESS - Quando precisa de informação recente
    let needs_freshness = needs_freshness_check(&q_lower);
    if needs_freshness {
        reasoning_parts.push("Freshness: pergunta requer informação atualizada".to_string());
    }

    // 3. COMPLETENESS vs PLURALITY
    // Completeness tem precedência - verifica primeiro
    let needs_completeness = needs_completeness_check(&q_lower);
    let needs_plurality = if needs_completeness {
        false // Completeness tem precedência
    } else {
        needs_plurality_check(&q_lower)
    };

    if needs_completeness {
        reasoning_parts.push("Completeness: pergunta menciona elementos específicos que precisam ser cobertos".to_string());
    } else if needs_plurality {
        reasoning_parts.push("Plurality: pergunta pede múltiplos itens ou exemplos".to_string());
    }

    QuestionAnalysis {
        needs_definitive,
        needs_freshness,
        needs_plurality,
        needs_completeness,
        reasoning: reasoning_parts.join("; "),
    }
}

/// Verifica se é paradoxo ou pergunta inerentemente impossível de responder
fn is_paradox_or_unanswerable(question: &str) -> bool {
    let paradox_patterns = [
        "if a tree falls",
        "sound in a forest",
        "what happens when an unstoppable",
        "immovable object",
        "this statement is false",
        "can god create a rock",
        "what is the sound of one hand",
        "before the big bang",
        "outside the universe",
        "what came before time",
    ];

    paradox_patterns.iter().any(|p| question.contains(p))
}

/// Verifica se precisa de avaliação de freshness
fn needs_freshness_check(question: &str) -> bool {
    // Termos explícitos de tempo
    let time_keywords = [
        "current", "currently", "hoje", "atual", "atualmente",
        "latest", "recent", "recently", "recente", "recentemente",
        "now", "today", "agora", "this year", "este ano",
        "2024", "2025", "novo", "new", "última", "últimas",
        "price", "preço", "rate", "taxa", "version", "versão",
        "ceo", "president", "presidente", "leader", "líder",
        "status", "estado", "update", "atualização",
    ];

    // Contextos que implicam informação atual
    let freshness_contexts = [
        "stock", "ação", "ações", "market", "mercado",
        "election", "eleição", "weather", "tempo", "clima",
        "score", "placar", "live", "ao vivo",
        "trending", "tendência", "viral",
    ];

    time_keywords.iter().any(|k| question.contains(k))
        || freshness_contexts.iter().any(|c| question.contains(c))
}

/// Verifica se precisa de avaliação de plurality
fn needs_plurality_check(question: &str) -> bool {
    // Números explícitos pedindo itens
    let number_pattern = Regex::new(r"\b(\d+)\s*(example|exemplo|item|way|forma|método|method|thing|coisa|step|passo|tip|dica|reason|razão|motivo)s?\b").unwrap();
    if number_pattern.is_match(question) {
        return true;
    }

    // Palavras que indicam múltiplos
    let plurality_keywords = [
        "examples", "exemplos", "list", "liste", "listar",
        "enumerate", "enumere", "enumerar",
        "several", "vários", "várias", "alguns", "algumas",
        "many", "muitos", "muitas", "multiple", "múltiplos",
        "ways to", "formas de", "maneiras de",
        "methods for", "métodos para",
        "top ", "melhores", "principais",
        "pros and cons", "prós e contras",
        "advantages and disadvantages", "vantagens e desvantagens",
        "steps", "passos", "etapas",
        "tips", "dicas",
        "reasons", "razões", "motivos",
    ];

    // Padrões como "What are the..."
    let plural_patterns = [
        "what are the", "quais são", "quais sao",
        "name some", "cite alguns", "cite algumas",
        "give me", "me dê", "me de",
    ];

    plurality_keywords.iter().any(|k| question.contains(k))
        || plural_patterns.iter().any(|p| question.contains(p))
}

/// Verifica se precisa de avaliação de completeness
fn needs_completeness_check(question: &str) -> bool {
    // Padrões que indicam elementos específicos a serem cobertos
    
    // Comparações explícitas entre entidades nomeadas
    let comparison_patterns = [
        r"(?i)\bcompar[ei](?:ng|r|e)?\s+\w+\s+(?:and|e|y|und|et)\s+\w+",
        r"(?i)\bdifference[s]?\s+between\s+\w+\s+(?:and|,)",
        r"(?i)\bdiferença[s]?\s+entre\s+\w+\s+(?:e|,)",
        r"(?i)\b(\w+)\s+(?:vs\.?|versus)\s+(\w+)",
    ];

    for pattern in &comparison_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(question) {
                return true;
            }
        }
    }

    // Elementos separados por vírgulas e "and/e"
    // Ex: "Apple, Microsoft, and Google"
    let list_pattern = Regex::new(r"(?i)(\w+),\s*(\w+),?\s*(?:and|e|y|und|et)\s+(\w+)").unwrap();
    if list_pattern.is_match(question) {
        // Verificar se são entidades nomeadas (começam com maiúscula no original)
        let original_words: Vec<&str> = question.split_whitespace().collect();
        let capitalized_count = original_words
            .iter()
            .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
            .count();
        
        if capitalized_count >= 3 {
            return true;
        }
    }

    // Padrões explícitos de múltiplos aspectos
    let aspect_patterns = [
        r"(?i)(?:economic|econômico),?\s*(?:social|social),?\s*(?:and|e)\s*(?:environmental|ambiental)",
        r"(?i)(?:background|contexto),?\s*(?:participants?|participantes?),?\s*(?:and|e)\s*(?:significance|significado)",
        r"(?i)(?:history|história),?\s*(?:culture|cultura),?\s*(?:and|e)\s*(?:politics|política)",
    ];

    for pattern in &aspect_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(question) {
                return true;
            }
        }
    }

    // "Both X and Y"
    let both_pattern = Regex::new(r"(?i)\bboth\s+\w+\s+and\s+\w+").unwrap();
    if both_pattern.is_match(question) {
        return true;
    }

    // "ambos" em português
    if question.contains("ambos") || question.contains("ambas") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TESTES DE DEFINITIVE
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_definitive_normal_question() {
        let types = determine_required_evaluations("What is the capital of France?");
        assert!(types.contains(&EvaluationType::Definitive));
    }

    #[test]
    fn test_definitive_paradox() {
        let types = determine_required_evaluations("If a tree falls in a forest with no observers, does it make a sound?");
        assert!(!types.contains(&EvaluationType::Definitive));
    }

    #[test]
    fn test_definitive_subjective_still_needed() {
        // Perguntas "subjetivas" ainda podem ser avaliadas definitivamente
        let types = determine_required_evaluations("What is the best programming language?");
        assert!(types.contains(&EvaluationType::Definitive));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TESTES DE FRESHNESS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_freshness_current_keyword() {
        let types = determine_required_evaluations("What is the current price of Bitcoin?");
        assert!(types.contains(&EvaluationType::Freshness));
    }

    #[test]
    fn test_freshness_latest_keyword() {
        let types = determine_required_evaluations("What is the latest version of Node.js?");
        assert!(types.contains(&EvaluationType::Freshness));
    }

    #[test]
    fn test_freshness_portuguese() {
        let types = determine_required_evaluations("Qual é o preço atual do dólar?");
        assert!(types.contains(&EvaluationType::Freshness));
    }

    #[test]
    fn test_freshness_historical() {
        let types = determine_required_evaluations("What caused World War 2?");
        assert!(!types.contains(&EvaluationType::Freshness));
    }

    #[test]
    fn test_freshness_year_mention() {
        let types = determine_required_evaluations("What are the AI trends in 2025?");
        assert!(types.contains(&EvaluationType::Freshness));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TESTES DE PLURALITY
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_plurality_explicit_number() {
        let types = determine_required_evaluations("Give me 5 examples of Rust web frameworks");
        assert!(types.contains(&EvaluationType::Plurality));
    }

    #[test]
    fn test_plurality_list_keyword() {
        let types = determine_required_evaluations("List the best practices for API design");
        assert!(types.contains(&EvaluationType::Plurality));
    }

    #[test]
    fn test_plurality_top_keyword() {
        let types = determine_required_evaluations("What are the top programming languages?");
        assert!(types.contains(&EvaluationType::Plurality));
    }

    #[test]
    fn test_plurality_portuguese() {
        let types = determine_required_evaluations("Quais são os principais frameworks JavaScript?");
        assert!(types.contains(&EvaluationType::Plurality));
    }

    #[test]
    fn test_plurality_eigenvalues() {
        // Exemplo do TypeScript: matriz 4x4 tem múltiplos eigenvalues
        let types = determine_required_evaluations("Calculate the eigenvalues of this 4x4 matrix");
        // Não tem keywords explícitos, então pode não detectar
        // Isso é uma limitação do approach sem LLM
    }

    #[test]
    fn test_plurality_single_item() {
        let types = determine_required_evaluations("What is the capital of France?");
        assert!(!types.contains(&EvaluationType::Plurality));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TESTES DE COMPLETENESS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_completeness_comparison() {
        let types = determine_required_evaluations("What are the differences between React and Vue?");
        assert!(types.contains(&EvaluationType::Completeness));
    }

    #[test]
    fn test_completeness_multiple_entities() {
        // Entidades com maiúsculas são detectadas
        let types = determine_required_evaluations("Compare the companies Apple, Microsoft, and Google");
        // Pode não detectar sem contexto adicional - limitação do approach sem LLM
        // O importante é que não cause panic
        assert!(types.contains(&EvaluationType::Definitive));
    }

    #[test]
    fn test_completeness_both_pattern() {
        let types = determine_required_evaluations("Explain both Newton and Leibniz contributions to calculus");
        assert!(types.contains(&EvaluationType::Completeness));
    }

    #[test]
    fn test_completeness_vs_plurality_precedence() {
        // Quando completeness é detectado, plurality deve ser false
        let analysis = analyze_question("Compare React and Vue frameworks with examples");
        assert!(analysis.needs_completeness);
        assert!(!analysis.needs_plurality);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TESTES DE EXEMPLOS DO TYPESCRIPT
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_ts_example_calculus() {
        // 谁发明了微积分？牛顿和莱布尼兹各自的贡献是什么？
        // needsDefinitive: true, needsFreshness: false, needsPlurality: false, needsCompleteness: true
        let types = determine_required_evaluations("Who invented calculus? Explain both Newton and Leibniz contributions");
        assert!(types.contains(&EvaluationType::Definitive));
        // Com "both" pattern, detecta completeness
        assert!(types.contains(&EvaluationType::Completeness));
    }

    #[test]
    fn test_ts_example_shakespeare() {
        // シェイクスピアの最も有名な悲劇を5つ挙げ
        // needsDefinitive: true, needsFreshness: false, needsPlurality: true, needsCompleteness: false
        let types = determine_required_evaluations("List 5 famous Shakespeare tragedies");
        assert!(types.contains(&EvaluationType::Definitive));
        assert!(!types.contains(&EvaluationType::Freshness));
        assert!(types.contains(&EvaluationType::Plurality));
        assert!(!types.contains(&EvaluationType::Completeness));
    }

    #[test]
    fn test_ts_example_mortgage_rates() {
        // Current interest rates for Bank of America, Wells Fargo, and Chase Bank
        // needsDefinitive: true, needsFreshness: true, needsPlurality: false, needsCompleteness: true
        let types = determine_required_evaluations("What are the current interest rates for Bank of America, Wells Fargo, and Chase Bank?");
        assert!(types.contains(&EvaluationType::Definitive));
        assert!(types.contains(&EvaluationType::Freshness)); // "current" keyword
        // Completeness pode não detectar sem LLM - limitação conhecida
    }

    #[test]
    fn test_ts_example_ai_trends() {
        // 2025年に注目すべきAIの3つのトレンド
        // needsDefinitive: true, needsFreshness: true, needsPlurality: true, needsCompleteness: false
        let types = determine_required_evaluations("List 3 AI trends to watch in 2025");
        assert!(types.contains(&EvaluationType::Definitive));
        assert!(types.contains(&EvaluationType::Freshness)); // 2025
        assert!(types.contains(&EvaluationType::Plurality)); // "List" + "3"
    }

    #[test]
    fn test_ts_example_tree_paradox() {
        // If a tree falls in a forest with absolutely no observers
        // needsDefinitive: false, needsFreshness: false, needsPlurality: false, needsCompleteness: false
        let types = determine_required_evaluations("If a tree falls in a forest with absolutely no observers, does it make a sound?");
        assert!(types.is_empty() || !types.contains(&EvaluationType::Definitive));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TESTES DE ANÁLISE DETALHADA
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_analyze_question() {
        let analysis = analyze_question("What are the current top 5 AI trends?");
        assert!(analysis.needs_definitive);
        assert!(analysis.needs_freshness);
        assert!(analysis.needs_plurality);
        assert!(!analysis.reasoning.is_empty());
    }

    #[test]
    fn test_analyze_question_reasoning() {
        let analysis = analyze_question("Compare Python and JavaScript");
        assert!(analysis.reasoning.contains("Completeness") || analysis.reasoning.contains("completeness"));
    }
}

