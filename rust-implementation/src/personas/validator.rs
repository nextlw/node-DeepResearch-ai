// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// VALIDADOR DE PERSONAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Sistema de validação para garantir que personas seguem o contrato esperado.
// Usado para validar personas antes de registrá-las no registry.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashSet;

use super::{CognitivePersona, QueryContext, PersonaRegistry};
use crate::types::SerpQuery;

/// Resultado de uma validação
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Se a validação passou
    pub valid: bool,
    /// Erros encontrados (vazio se válido)
    pub errors: Vec<ValidationError>,
    /// Avisos (não bloqueantes)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Cria resultado de sucesso
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
        }
    }

    /// Cria resultado de falha com erros
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: vec![],
        }
    }

    /// Adiciona um erro
    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error);
    }

    /// Adiciona um aviso
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Verifica se é válido
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Mescla com outro resultado
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.valid {
            self.valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// Tipos de erro de validação
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Nome está vazio
    EmptyName,
    /// Nome já existe no registry
    DuplicateName(String),
    /// Nome contém caracteres inválidos
    InvalidNameChars(String),
    /// Foco/descrição muito curto (mínimo 10 chars)
    FocusTooShort {
        /// Tamanho atual do foco
        actual: usize,
        /// Tamanho mínimo requerido
        minimum: usize,
    },
    /// Foco está vazio
    EmptyFocus,
    /// Peso fora do range permitido (0.0 - 2.0)
    WeightOutOfRange {
        /// Valor atual do peso
        value: f32,
        /// Valor mínimo permitido
        min: f32,
        /// Valor máximo permitido
        max: f32,
    },
    /// Query de saída está vazia
    EmptyOutputQuery,
    /// Query de saída é igual à entrada (sem expansão)
    NoQueryExpansion,
    /// Query contém apenas stop words
    QueryOnlyStopWords,
    /// Erro customizado
    Custom(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "Persona name cannot be empty"),
            Self::DuplicateName(name) => write!(f, "Persona '{}' already exists", name),
            Self::InvalidNameChars(chars) => write!(f, "Name contains invalid characters: {}", chars),
            Self::FocusTooShort { actual, minimum } => {
                write!(f, "Focus too short: {} chars (minimum {})", actual, minimum)
            }
            Self::EmptyFocus => write!(f, "Persona focus cannot be empty"),
            Self::WeightOutOfRange { value, min, max } => {
                write!(f, "Weight {} out of range [{}, {}]", value, min, max)
            }
            Self::EmptyOutputQuery => write!(f, "Output query cannot be empty"),
            Self::NoQueryExpansion => write!(f, "Query was not expanded (output equals input)"),
            Self::QueryOnlyStopWords => write!(f, "Query contains only stop words"),
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Configuração do validador
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// Tamanho mínimo do foco/descrição
    pub min_focus_length: usize,
    /// Peso mínimo permitido
    pub min_weight: f32,
    /// Peso máximo permitido
    pub max_weight: f32,
    /// Se deve verificar expansão de query
    pub check_query_expansion: bool,
    /// Se deve verificar duplicatas no registry
    pub check_duplicates: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            min_focus_length: 10,
            min_weight: 0.0,
            max_weight: 2.0,
            check_query_expansion: true,
            check_duplicates: true,
        }
    }
}

/// Validador de personas
///
/// Verifica se uma persona segue o contrato esperado:
/// - Nome não vazio e único
/// - Foco com descrição mínima
/// - Peso dentro do range
/// - Query de saída válida
///
/// # Exemplo
///
/// ```rust,ignore
/// let validator = PersonaValidator::new();
/// let result = validator.validate(&my_persona, Some(&registry));
///
/// if !result.is_valid() {
///     for error in &result.errors {
///         println!("Error: {}", error);
///     }
/// }
/// ```
pub struct PersonaValidator {
    config: ValidatorConfig,
}

impl PersonaValidator {
    /// Cria um novo validador com configuração padrão
    pub fn new() -> Self {
        Self {
            config: ValidatorConfig::default(),
        }
    }

    /// Cria um validador com configuração customizada
    pub fn with_config(config: ValidatorConfig) -> Self {
        Self { config }
    }

    /// Valida uma persona completamente
    pub fn validate(
        &self,
        persona: &dyn CognitivePersona,
        registry: Option<&PersonaRegistry>,
    ) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Validar nome
        result.merge(self.validate_name(persona.name(), registry));

        // Validar foco
        result.merge(self.validate_focus(persona.focus()));

        // Validar peso
        result.merge(self.validate_weight(persona.weight()));

        // Validar saída de query
        let ctx = QueryContext::default();
        let test_query = "test query for validation";
        let output = persona.expand_query(test_query, &ctx);
        result.merge(self.validate_output(&output, test_query));

        result
    }

    /// Valida o nome da persona
    pub fn validate_name(
        &self,
        name: &str,
        registry: Option<&PersonaRegistry>,
    ) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Nome não pode ser vazio
        if name.is_empty() {
            result.add_error(ValidationError::EmptyName);
            return result;
        }

        // Nome não pode ter apenas espaços
        if name.trim().is_empty() {
            result.add_error(ValidationError::EmptyName);
            return result;
        }

        // Verificar caracteres inválidos (apenas alfanuméricos, espaços e alguns símbolos)
        let invalid_chars: String = name
            .chars()
            .filter(|c| !c.is_alphanumeric() && !matches!(c, ' ' | '-' | '_'))
            .collect();
        
        if !invalid_chars.is_empty() {
            result.add_error(ValidationError::InvalidNameChars(invalid_chars));
        }

        // Verificar duplicatas no registry
        if self.config.check_duplicates {
            if let Some(reg) = registry {
                if reg.contains(name) {
                    result.add_error(ValidationError::DuplicateName(name.to_string()));
                }
            }
        }

        result
    }

    /// Valida o foco/descrição da persona
    pub fn validate_focus(&self, focus: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Foco não pode ser vazio
        if focus.is_empty() {
            result.add_error(ValidationError::EmptyFocus);
            return result;
        }

        // Foco deve ter tamanho mínimo
        let trimmed_len = focus.trim().len();
        if trimmed_len < self.config.min_focus_length {
            result.add_error(ValidationError::FocusTooShort {
                actual: trimmed_len,
                minimum: self.config.min_focus_length,
            });
        }

        // Aviso se foco for muito genérico
        let generic_terms = ["general", "generic", "default", "test", "testing"];
        if generic_terms.iter().any(|t| focus.to_lowercase().contains(t)) {
            result.add_warning("Focus appears too generic, consider being more specific");
        }

        result
    }

    /// Valida o peso da persona
    pub fn validate_weight(&self, weight: f32) -> ValidationResult {
        let mut result = ValidationResult::success();

        if weight < self.config.min_weight || weight > self.config.max_weight {
            result.add_error(ValidationError::WeightOutOfRange {
                value: weight,
                min: self.config.min_weight,
                max: self.config.max_weight,
            });
        }

        // Aviso se peso for muito extremo
        if weight < 0.5 {
            result.add_warning("Very low weight may cause persona to be rarely used");
        } else if weight > 1.5 {
            result.add_warning("Very high weight may dominate other personas");
        }

        result
    }

    /// Valida a query de saída
    pub fn validate_output(&self, output: &SerpQuery, input: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Query não pode ser vazia
        if output.q.is_empty() {
            result.add_error(ValidationError::EmptyOutputQuery);
            return result;
        }

        // Query não pode ser só espaços
        if output.q.trim().is_empty() {
            result.add_error(ValidationError::EmptyOutputQuery);
            return result;
        }

        // Verificar se houve expansão (se configurado)
        if self.config.check_query_expansion {
            if output.q.trim() == input.trim() {
                result.add_error(ValidationError::NoQueryExpansion);
            }
        }

        // Verificar se query contém apenas stop words
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been",
            "have", "has", "had", "do", "does", "did", "will", "would",
            "could", "should", "may", "might", "can", "to", "of", "in",
            "for", "on", "with", "at", "by", "from", "or", "and",
        ].into_iter().collect();

        let words: Vec<&str> = output.q.split_whitespace().collect();
        let non_stop_words: Vec<_> = words
            .iter()
            .filter(|w| !stop_words.contains(w.to_lowercase().as_str()))
            .collect();

        if non_stop_words.is_empty() && !words.is_empty() {
            result.add_error(ValidationError::QueryOnlyStopWords);
        }

        result
    }

    /// Valida todas as personas do registry
    pub fn validate_registry(&self, registry: &PersonaRegistry) -> Vec<(String, ValidationResult)> {
        registry
            .iter()
            .map(|(name, persona)| {
                // Não verificar duplicatas nem expansão pois personas existentes
                // podem ter comportamentos válidos que não modificam todas as queries
                let config = ValidatorConfig {
                    check_duplicates: false,
                    check_query_expansion: false, // Algumas personas podem não expandir certas queries
                    ..self.config.clone()
                };
                let validator = PersonaValidator::with_config(config);
                
                // PersonaBox implementa Deref<Target = dyn CognitivePersona>
                (name.to_string(), validator.validate(&**persona, None))
            })
            .collect()
    }

    /// Testa conformidade de uma persona com testes extensivos
    pub fn test_conformance(
        &self,
        persona: &dyn CognitivePersona,
    ) -> ConformanceReport {
        let mut report = ConformanceReport::new(persona.name());

        // Teste 1: Validação básica
        let basic_result = self.validate(persona, None);
        report.add_test("basic_validation", basic_result.is_valid(), 
            if basic_result.is_valid() { "Passed".into() } 
            else { format!("Failed: {:?}", basic_result.errors) });

        // Teste 2: Thread safety (Send + Sync)
        report.add_test("thread_safety", true, "CognitivePersona requires Send + Sync".into());

        // Teste 3: Determinismo (mesma entrada = mesma saída)
        let ctx = QueryContext::default();
        let input = "test determinism query";
        let output1 = persona.expand_query(input, &ctx);
        let output2 = persona.expand_query(input, &ctx);
        let deterministic = output1.q == output2.q;
        report.add_test("determinism", deterministic,
            if deterministic { "Same input produces same output".into() }
            else { format!("Non-deterministic: '{}' vs '{}'", output1.q, output2.q) });

        // Teste 4: Não pânico com entrada vazia
        let empty_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            persona.expand_query("", &ctx)
        }));
        let no_panic_empty = empty_result.is_ok();
        report.add_test("no_panic_empty_input", no_panic_empty,
            if no_panic_empty { "Handles empty input".into() }
            else { "Panicked on empty input".into() });

        // Teste 5: Não pânico com entrada muito longa
        let long_input = "a".repeat(10000);
        let long_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            persona.expand_query(&long_input, &ctx)
        }));
        let no_panic_long = long_result.is_ok();
        report.add_test("no_panic_long_input", no_panic_long,
            if no_panic_long { "Handles long input".into() }
            else { "Panicked on long input".into() });

        // Teste 6: Não pânico com caracteres especiais
        let special_input = "测试 тест 🎉 <script>alert('xss')</script>";
        let special_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            persona.expand_query(special_input, &ctx)
        }));
        let no_panic_special = special_result.is_ok();
        report.add_test("no_panic_special_chars", no_panic_special,
            if no_panic_special { "Handles special characters".into() }
            else { "Panicked on special characters".into() });

        // Teste 7: is_applicable retorna sem pânico
        let applicable_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            persona.is_applicable(&ctx)
        }));
        let applicable_ok = applicable_result.is_ok();
        report.add_test("is_applicable_safe", applicable_ok,
            if applicable_ok { "is_applicable works correctly".into() }
            else { "is_applicable panicked".into() });

        // Teste 8: prompt_description não é vazio
        let desc = persona.prompt_description();
        let desc_ok = !desc.is_empty();
        report.add_test("prompt_description_not_empty", desc_ok,
            if desc_ok { format!("Description: '{}'", desc) }
            else { "prompt_description returned empty".into() });

        report.finalize();
        report
    }
}

impl Default for PersonaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Relatório de conformidade de uma persona
#[derive(Debug, Clone)]
pub struct ConformanceReport {
    /// Nome da persona testada
    pub persona_name: String,
    /// Testes executados
    pub tests: Vec<ConformanceTest>,
    /// Total de testes
    pub total_tests: usize,
    /// Testes que passaram
    pub passed_tests: usize,
    /// Taxa de conformidade (0.0 - 1.0)
    pub conformance_rate: f32,
}

impl ConformanceReport {
    /// Cria novo relatório
    pub fn new(persona_name: &str) -> Self {
        Self {
            persona_name: persona_name.to_string(),
            tests: Vec::new(),
            total_tests: 0,
            passed_tests: 0,
            conformance_rate: 0.0,
        }
    }

    /// Adiciona resultado de um teste
    pub fn add_test(&mut self, name: &str, passed: bool, details: String) {
        self.tests.push(ConformanceTest {
            name: name.to_string(),
            passed,
            details,
        });
    }

    /// Finaliza o relatório calculando métricas
    pub fn finalize(&mut self) {
        self.total_tests = self.tests.len();
        self.passed_tests = self.tests.iter().filter(|t| t.passed).count();
        self.conformance_rate = if self.total_tests > 0 {
            self.passed_tests as f32 / self.total_tests as f32
        } else {
            0.0
        };
    }

    /// Verifica se todos os testes passaram
    pub fn all_passed(&self) -> bool {
        self.passed_tests == self.total_tests
    }

    /// Retorna resumo formatado
    pub fn summary(&self) -> String {
        format!(
            "Conformance Report for '{}': {}/{} tests passed ({:.0}%)",
            self.persona_name,
            self.passed_tests,
            self.total_tests,
            self.conformance_rate * 100.0
        )
    }
}

/// Um teste de conformidade individual
#[derive(Debug, Clone)]
pub struct ConformanceTest {
    /// Nome do teste
    pub name: String,
    /// Se passou
    pub passed: bool,
    /// Detalhes/mensagem
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Persona válida para testes
    struct ValidPersona;
    impl CognitivePersona for ValidPersona {
        fn name(&self) -> &'static str { "Valid Test Persona" }
        fn focus(&self) -> &'static str { "testing validation with sufficient description length" }
        fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
            SerpQuery {
                q: format!("{} expanded test query", original),
                tbs: None,
                location: None,
            }
        }
        fn weight(&self) -> f32 { 1.0 }
    }

    // Persona com nome vazio
    struct EmptyNamePersona;
    impl CognitivePersona for EmptyNamePersona {
        fn name(&self) -> &'static str { "" }
        fn focus(&self) -> &'static str { "testing empty name validation" }
        fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
            SerpQuery { q: format!("{} test", original), tbs: None, location: None }
        }
    }

    // Persona com foco muito curto
    struct ShortFocusPersona;
    impl CognitivePersona for ShortFocusPersona {
        fn name(&self) -> &'static str { "Short Focus" }
        fn focus(&self) -> &'static str { "short" } // Menos de 10 chars
        fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
            SerpQuery { q: format!("{} test", original), tbs: None, location: None }
        }
    }

    // Persona com peso inválido
    struct InvalidWeightPersona;
    impl CognitivePersona for InvalidWeightPersona {
        fn name(&self) -> &'static str { "Invalid Weight" }
        fn focus(&self) -> &'static str { "testing weight validation properly" }
        fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
            SerpQuery { q: format!("{} test", original), tbs: None, location: None }
        }
        fn weight(&self) -> f32 { 5.0 } // Acima de 2.0
    }

    // Persona que não expande query
    struct NoExpansionPersona;
    impl CognitivePersona for NoExpansionPersona {
        fn name(&self) -> &'static str { "No Expansion" }
        fn focus(&self) -> &'static str { "testing query expansion validation" }
        fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
            SerpQuery { q: original.to_string(), tbs: None, location: None } // Não expande
        }
    }

    // Persona que retorna query vazia
    struct EmptyQueryPersona;
    impl CognitivePersona for EmptyQueryPersona {
        fn name(&self) -> &'static str { "Empty Query" }
        fn focus(&self) -> &'static str { "testing empty query validation" }
        fn expand_query(&self, _original: &str, _ctx: &QueryContext) -> SerpQuery {
            SerpQuery { q: "".to_string(), tbs: None, location: None }
        }
    }

    #[test]
    fn test_valid_persona() {
        let validator = PersonaValidator::new();
        let result = validator.validate(&ValidPersona, None);
        
        assert!(result.is_valid(), "Valid persona should pass: {:?}", result.errors);
    }

    #[test]
    fn test_empty_name() {
        let validator = PersonaValidator::new();
        let result = validator.validate(&EmptyNamePersona, None);
        
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::EmptyName)));
    }

    #[test]
    fn test_short_focus() {
        let validator = PersonaValidator::new();
        let result = validator.validate(&ShortFocusPersona, None);
        
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::FocusTooShort { .. })));
    }

    #[test]
    fn test_invalid_weight() {
        let validator = PersonaValidator::new();
        let result = validator.validate(&InvalidWeightPersona, None);
        
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::WeightOutOfRange { .. })));
    }

    #[test]
    fn test_no_expansion() {
        let validator = PersonaValidator::new();
        let result = validator.validate(&NoExpansionPersona, None);
        
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::NoQueryExpansion)));
    }

    #[test]
    fn test_empty_query() {
        let validator = PersonaValidator::new();
        let result = validator.validate(&EmptyQueryPersona, None);
        
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::EmptyOutputQuery)));
    }

    #[test]
    fn test_validate_name_with_special_chars() {
        let validator = PersonaValidator::new();
        let result = validator.validate_name("Test@Persona#123", None);
        
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::InvalidNameChars(_))));
    }

    #[test]
    fn test_validate_name_valid_chars() {
        let validator = PersonaValidator::new();
        let result = validator.validate_name("Test Persona-123_abc", None);
        
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_weight_boundaries() {
        let validator = PersonaValidator::new();
        
        // Limite inferior válido
        assert!(validator.validate_weight(0.0).is_valid());
        // Limite superior válido
        assert!(validator.validate_weight(2.0).is_valid());
        // Abaixo do limite
        assert!(!validator.validate_weight(-0.1).is_valid());
        // Acima do limite
        assert!(!validator.validate_weight(2.1).is_valid());
    }

    #[test]
    fn test_validate_focus_minimum_length() {
        let validator = PersonaValidator::new();
        
        // Exatamente 10 chars
        assert!(validator.validate_focus("1234567890").is_valid());
        // Menos de 10 chars
        assert!(!validator.validate_focus("123456789").is_valid());
    }

    #[test]
    fn test_custom_config() {
        let config = ValidatorConfig {
            min_focus_length: 5, // Menos restritivo
            ..Default::default()
        };
        let validator = PersonaValidator::with_config(config);
        
        // Agora "short" (5 chars) deve passar
        let result = validator.validate_focus("short");
        assert!(result.is_valid());
    }

    #[test]
    fn test_conformance_report() {
        let validator = PersonaValidator::new();
        let report = validator.test_conformance(&ValidPersona);
        
        assert!(report.all_passed(), "All tests should pass for valid persona");
        assert!(report.conformance_rate >= 1.0);
        assert!(!report.summary().is_empty());
    }

    #[test]
    fn test_conformance_report_invalid_persona() {
        let validator = PersonaValidator::new();
        let report = validator.test_conformance(&EmptyNamePersona);
        
        assert!(!report.all_passed(), "Some tests should fail for invalid persona");
        assert!(report.conformance_rate < 1.0);
    }

    #[test]
    fn test_validate_registry() {
        let registry = PersonaRegistry::with_defaults();
        let validator = PersonaValidator::new();
        
        let results = validator.validate_registry(&registry);
        
        // Todas as 7 personas padrão devem ser válidas
        assert_eq!(results.len(), 7);
        for (name, result) in &results {
            assert!(result.is_valid(), "Persona '{}' should be valid: {:?}", name, result.errors);
        }
    }

    #[test]
    fn test_duplicate_name_detection() {
        let registry = PersonaRegistry::with_defaults();
        let validator = PersonaValidator::new();
        
        // Criar persona com nome que já existe
        struct DuplicateNamePersona;
        impl CognitivePersona for DuplicateNamePersona {
            fn name(&self) -> &'static str { "Expert Skeptic" } // Já existe!
            fn focus(&self) -> &'static str { "testing duplicate detection properly" }
            fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
                SerpQuery { q: format!("{} test", original), tbs: None, location: None }
            }
        }

        let result = validator.validate(&DuplicateNamePersona, Some(&registry));
        
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::DuplicateName(_))));
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::success();
        result1.add_warning("Warning 1");

        let mut result2 = ValidationResult::success();
        result2.add_error(ValidationError::EmptyName);
        result2.add_warning("Warning 2");

        result1.merge(result2);

        assert!(!result1.is_valid());
        assert_eq!(result1.errors.len(), 1);
        assert_eq!(result1.warnings.len(), 2);
    }

    #[test]
    fn test_warnings_generic_focus() {
        let validator = PersonaValidator::new();
        let result = validator.validate_focus("general testing purpose here");
        
        assert!(result.is_valid()); // Válido mas com warning
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_warnings_extreme_weight() {
        let validator = PersonaValidator::new();
        
        // Peso muito baixo
        let low = validator.validate_weight(0.3);
        assert!(low.is_valid());
        assert!(!low.warnings.is_empty());

        // Peso muito alto
        let high = validator.validate_weight(1.8);
        assert!(high.is_valid());
        assert!(!high.warnings.is_empty());
    }
}

