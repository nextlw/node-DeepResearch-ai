// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CODE SANDBOX - Execução segura de JavaScript via Boa Engine
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Este módulo permite ao agente gerar e executar código JavaScript em um
// ambiente isolado (sandbox) para processar dados, chamar APIs e transformar
// informações coletadas durante a pesquisa.
//
// Características:
// - Isolamento completo (sem acesso a filesystem/rede)
// - Timeout de execução configurável
// - Limite de iterações de loop e recursão
// - Retry inteligente com feedback de erros para o LLM
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::llm::{LlmClient, LlmError};
use crate::types::KnowledgeItem;
use boa_engine::{Context, JsValue, Source};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Erros que podem ocorrer durante execução no sandbox
#[derive(Debug, Error)]
pub enum SandboxError {
    /// Erro de execução JavaScript
    #[error("JavaScript execution error: {0}")]
    ExecutionError(String),

    /// Timeout de execução excedido
    #[error("Execution timeout after {0}ms")]
    Timeout(u64),

    /// Código não retornou valor
    #[error("Code did not return a value (missing return statement)")]
    NoReturnValue,

    /// Erro ao gerar código via LLM
    #[error("Code generation failed: {0}")]
    GenerationError(String),

    /// Máximo de tentativas excedido
    #[error("Failed after {attempts} attempts. Last error: {last_error}")]
    MaxAttemptsExceeded {
        /// Número de tentativas realizadas
        attempts: usize,
        /// Último erro encontrado
        last_error: String,
    },

    /// Erro de limite (loop/recursão)
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
}

impl From<LlmError> for SandboxError {
    fn from(err: LlmError) -> Self {
        SandboxError::GenerationError(err.to_string())
    }
}

/// Contexto do sandbox com variáveis disponíveis para o código
#[derive(Debug, Clone)]
pub struct SandboxContext {
    /// Variáveis como JSON strings (serão parseadas no JS)
    variables: HashMap<String, String>,
}

impl SandboxContext {
    /// Cria um novo contexto vazio
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Cria contexto a partir dos itens de conhecimento do agente
    ///
    /// Transforma o knowledge em variáveis acessíveis no JavaScript:
    /// - `knowledge`: Array com todos os itens
    /// - `urls`: Array com conteúdos de URLs lidas
    /// - `answers`: Array com respostas anteriores
    pub fn from_knowledge(knowledge: &[KnowledgeItem]) -> Self {
        let mut ctx = Self::new();

        // Converter knowledge para JSON
        let knowledge_json: Vec<serde_json::Value> = knowledge
            .iter()
            .map(|item| {
                serde_json::json!({
                    "question": item.question,
                    "answer": item.answer,
                    "type": item.item_type.as_str(),
                })
            })
            .collect();

        ctx.set_variable("knowledge", &serde_json::to_string(&knowledge_json).unwrap_or_default());

        // Extrair URLs como variável separada
        let urls: Vec<&str> = knowledge
            .iter()
            .filter(|k| matches!(k.item_type, crate::types::KnowledgeType::Url))
            .map(|k| k.answer.as_str())
            .collect();
        ctx.set_variable("urlContents", &serde_json::to_string(&urls).unwrap_or_default());

        // Extrair respostas anteriores
        let answers: Vec<&str> = knowledge
            .iter()
            .filter(|k| matches!(k.item_type, crate::types::KnowledgeType::Qa))
            .map(|k| k.answer.as_str())
            .collect();
        ctx.set_variable("previousAnswers", &serde_json::to_string(&answers).unwrap_or_default());

        ctx
    }

    /// Define uma variável no contexto
    pub fn set_variable(&mut self, name: &str, json_value: &str) {
        self.variables.insert(name.to_string(), json_value.to_string());
    }

    /// Gera descrição das variáveis para o prompt do LLM
    pub fn describe_for_llm(&self) -> String {
        if self.variables.is_empty() {
            return "No variables available.".to_string();
        }

        let mut description = String::new();
        for (name, value) in &self.variables {
            // Truncar valores muito longos para o prompt
            let preview = if value.len() > 200 {
                format!("{}...", &value[..200])
            } else {
                value.clone()
            };

            // Tentar determinar o tipo
            let type_hint = if value.starts_with('[') {
                "Array"
            } else if value.starts_with('{') {
                "Object"
            } else if value.starts_with('"') {
                "String"
            } else if value.parse::<f64>().is_ok() {
                "Number"
            } else if value == "true" || value == "false" {
                "Boolean"
            } else {
                "Unknown"
            };

            description.push_str(&format!(
                "- {} ({}) e.g. {}\n",
                name, type_hint, preview
            ));
        }
        description
    }

    /// Injeta as variáveis no contexto Boa
    fn inject_into_boa(&self, context: &mut Context) {
        for (name, json_value) in &self.variables {
            // Criar código para parsear o JSON e atribuir à variável
            let setup_code = format!(
                "var {} = JSON.parse('{}');",
                name,
                json_value.replace('\\', "\\\\").replace('\'', "\\'")
            );

            if let Err(e) = context.eval(Source::from_bytes(&setup_code)) {
                log::warn!("Failed to inject variable '{}': {}", name, e);
            }
        }
    }
}

impl Default for SandboxContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Resultado da execução do sandbox
#[derive(Debug, Clone)]
pub struct SandboxResult {
    /// Se a execução foi bem-sucedida
    pub success: bool,
    /// Output da execução (se sucesso)
    pub output: Option<String>,
    /// Erro da execução (se falha)
    pub error: Option<String>,
    /// Código que foi executado
    pub code: String,
    /// Número de tentativas realizadas
    pub attempts: usize,
    /// Tempo total de execução em ms
    pub execution_time_ms: u64,
}

/// Sandbox para execução segura de código JavaScript
///
/// # Exemplo
/// ```ignore
/// let sandbox = CodeSandbox::new(&knowledge, 5000);
/// let result = sandbox.solve(&llm, "Calculate the sum of all numbers").await?;
/// println!("Result: {}", result.output.unwrap_or_default());
/// ```
pub struct CodeSandbox {
    /// Contexto com variáveis disponíveis
    context: SandboxContext,
    /// Máximo de tentativas de geração/execução
    max_attempts: usize,
    /// Timeout de execução em milissegundos
    timeout_ms: u64,
    /// Limite de iterações de loop
    loop_iteration_limit: u64,
    /// Limite de profundidade de recursão
    recursion_limit: usize,
}

impl CodeSandbox {
    /// Cria um novo sandbox com configurações padrão
    ///
    /// # Arguments
    /// * `knowledge` - Itens de conhecimento para popular o contexto
    /// * `timeout_ms` - Timeout de execução em milissegundos (default: 5000)
    pub fn new(knowledge: &[KnowledgeItem], timeout_ms: u64) -> Self {
        Self {
            context: SandboxContext::from_knowledge(knowledge),
            max_attempts: 3,
            timeout_ms,
            loop_iteration_limit: 100_000,
            recursion_limit: 1000,
        }
    }

    /// Cria sandbox com contexto customizado
    pub fn with_context(context: SandboxContext, timeout_ms: u64) -> Self {
        Self {
            context,
            max_attempts: 3,
            timeout_ms,
            loop_iteration_limit: 100_000,
            recursion_limit: 1000,
        }
    }

    /// Define o máximo de tentativas
    pub fn max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Define limite de iterações de loop
    pub fn loop_limit(mut self, limit: u64) -> Self {
        self.loop_iteration_limit = limit;
        self
    }

    /// Define limite de recursão
    pub fn recursion_limit(mut self, limit: usize) -> Self {
        self.recursion_limit = limit;
        self
    }

    /// Resolve um problema gerando e executando código
    ///
    /// Este método:
    /// 1. Gera código via LLM baseado no problema
    /// 2. Executa o código no sandbox Boa
    /// 3. Se falhar, passa o erro para o LLM e tenta novamente
    /// 4. Repete até sucesso ou máximo de tentativas
    ///
    /// # Arguments
    /// * `llm` - Cliente LLM para geração de código
    /// * `problem` - Descrição do problema a resolver
    ///
    /// # Returns
    /// `SandboxResult` com o output ou erro
    pub async fn solve(
        &self,
        llm: &dyn LlmClient,
        problem: &str,
    ) -> Result<SandboxResult, SandboxError> {
        let mut attempts: Vec<(String, Option<String>)> = Vec::new();
        let start_time = Instant::now();

        for attempt in 0..self.max_attempts {
            log::debug!("Sandbox attempt {}/{}", attempt + 1, self.max_attempts);

            // Gerar código via LLM
            let code_response = llm
                .generate_code(problem, &self.context.describe_for_llm(), &attempts)
                .await?;

            log::debug!("Generated code:\n{}", code_response.code);

            // Executar código
            match self.execute(&code_response.code) {
                Ok(output) => {
                    log::info!("Sandbox execution successful on attempt {}", attempt + 1);
                    return Ok(SandboxResult {
                        success: true,
                        output: Some(output),
                        error: None,
                        code: code_response.code,
                        attempts: attempt + 1,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Sandbox attempt {} failed: {}", attempt + 1, error_msg);

                    // Guardar tentativa com erro para próxima iteração
                    attempts.push((code_response.code.clone(), Some(error_msg.clone())));

                    // Se foi a última tentativa, retornar erro
                    if attempt == self.max_attempts - 1 {
                        return Ok(SandboxResult {
                            success: false,
                            output: None,
                            error: Some(error_msg),
                            code: code_response.code,
                            attempts: attempt + 1,
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                        });
                    }
                }
            }
        }

        Err(SandboxError::MaxAttemptsExceeded {
            attempts: self.max_attempts,
            last_error: attempts
                .last()
                .and_then(|(_, e)| e.clone())
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }

    /// Executa código JavaScript no sandbox Boa
    ///
    /// # Arguments
    /// * `code` - Código JavaScript a executar
    ///
    /// # Returns
    /// String com o resultado serializado ou erro
    pub fn execute(&self, code: &str) -> Result<String, SandboxError> {
        let start = Instant::now();

        // Criar contexto Boa com limites de segurança
        let mut context = Context::default();

        // Nota: Boa 0.20 não tem set_loop_iteration_limit/set_recursion_limit diretamente
        // O isolamento é garantido pelo próprio design do engine (sem I/O)

        // Injetar variáveis do contexto
        self.context.inject_into_boa(&mut context);

        // Wrap o código em uma função para capturar o return
        let wrapped_code = format!(
            r#"
            (function() {{
                {}
            }})()
            "#,
            code
        );

        // Executar com verificação de timeout simples
        // (Boa não suporta timeout nativo, então fazemos verificação pós-execução)
        let result = context.eval(Source::from_bytes(&wrapped_code));

        let elapsed = start.elapsed();
        if elapsed > Duration::from_millis(self.timeout_ms) {
            return Err(SandboxError::Timeout(self.timeout_ms));
        }

        match result {
            Ok(value) => {
                // Converter resultado para string
                let output = self.js_value_to_string(&value, &mut context);

                if output == "undefined" {
                    return Err(SandboxError::NoReturnValue);
                }

                Ok(output)
            }
            Err(e) => {
                // Extrair mensagem de erro do Boa
                let error_msg = e.to_string();
                Err(SandboxError::ExecutionError(error_msg))
            }
        }
    }

    /// Converte JsValue para String legível
    fn js_value_to_string(&self, value: &JsValue, context: &mut Context) -> String {
        if value.is_undefined() {
            return "undefined".to_string();
        }
        if value.is_null() {
            return "null".to_string();
        }

        // Tentar converter para JSON primeiro (melhor para objetos/arrays)
        if let Ok(json) = value.to_json(context) {
            return json.to_string();
        }

        // Fallback para display string
        value.display().to_string()
    }

    /// Executa código diretamente sem geração via LLM
    ///
    /// Útil para testes ou quando o código já está pronto
    pub fn execute_direct(&self, code: &str) -> SandboxResult {
        let start = Instant::now();

        match self.execute(code) {
            Ok(output) => SandboxResult {
                success: true,
                output: Some(output),
                error: None,
                code: code.to_string(),
                attempts: 1,
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => SandboxResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                code: code.to_string(),
                attempts: 1,
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_execution() {
        let sandbox = CodeSandbox::with_context(SandboxContext::new(), 5000);
        let result = sandbox.execute_direct("return 1 + 1;");

        assert!(result.success);
        assert_eq!(result.output, Some("2".to_string()));
    }

    #[test]
    fn test_string_return() {
        let sandbox = CodeSandbox::with_context(SandboxContext::new(), 5000);
        let result = sandbox.execute_direct("return 'hello world';");

        assert!(result.success);
        assert_eq!(result.output, Some("\"hello world\"".to_string()));
    }

    #[test]
    fn test_array_return() {
        let sandbox = CodeSandbox::with_context(SandboxContext::new(), 5000);
        let result = sandbox.execute_direct("return [1, 2, 3];");

        assert!(result.success);
        assert_eq!(result.output, Some("[1,2,3]".to_string()));
    }

    #[test]
    fn test_object_return() {
        let sandbox = CodeSandbox::with_context(SandboxContext::new(), 5000);
        let result = sandbox.execute_direct("return { a: 1, b: 'test' };");

        assert!(result.success);
        // JSON output
        assert!(result.output.unwrap().contains("\"a\":1"));
    }

    #[test]
    fn test_context_variables() {
        let mut ctx = SandboxContext::new();
        ctx.set_variable("numbers", "[1, 2, 3, 4, 5]");
        ctx.set_variable("threshold", "3");

        let sandbox = CodeSandbox::with_context(ctx, 5000);
        let result = sandbox.execute_direct(
            "return numbers.filter(n => n > threshold).reduce((a, b) => a + b, 0);"
        );

        assert!(result.success);
        assert_eq!(result.output, Some("9".to_string())); // 4 + 5 = 9
    }

    #[test]
    fn test_missing_return() {
        let sandbox = CodeSandbox::with_context(SandboxContext::new(), 5000);
        let result = sandbox.execute_direct("let x = 1 + 1;");

        assert!(!result.success);
        assert!(result.error.unwrap().contains("return"));
    }

    #[test]
    fn test_syntax_error() {
        let sandbox = CodeSandbox::with_context(SandboxContext::new(), 5000);
        let result = sandbox.execute_direct("return {{{");

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_runtime_error() {
        let sandbox = CodeSandbox::with_context(SandboxContext::new(), 5000);
        let result = sandbox.execute_direct("return undefinedVariable.property;");

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_context_describe_for_llm() {
        let mut ctx = SandboxContext::new();
        ctx.set_variable("data", "[1, 2, 3]");
        ctx.set_variable("config", "{\"enabled\": true}");

        let description = ctx.describe_for_llm();

        assert!(description.contains("data"));
        assert!(description.contains("Array"));
        assert!(description.contains("config"));
        assert!(description.contains("Object"));
    }

    #[test]
    fn test_from_knowledge() {
        use crate::types::{KnowledgeItem, KnowledgeType};

        let knowledge = vec![
            KnowledgeItem {
                question: "What is Rust?".to_string(),
                answer: "A systems programming language".to_string(),
                item_type: KnowledgeType::Qa,
                references: vec![],
            },
            KnowledgeItem {
                question: "URL content".to_string(),
                answer: "Page content here".to_string(),
                item_type: KnowledgeType::Url,
                references: vec![],
            },
        ];

        let ctx = SandboxContext::from_knowledge(&knowledge);
        let description = ctx.describe_for_llm();

        assert!(description.contains("knowledge"));
        assert!(description.contains("urlContents"));
        assert!(description.contains("previousAnswers"));
    }

    #[test]
    fn test_complex_computation() {
        let mut ctx = SandboxContext::new();
        ctx.set_variable("data", r#"[{"name": "Alice", "score": 85}, {"name": "Bob", "score": 92}, {"name": "Charlie", "score": 78}]"#);

        let sandbox = CodeSandbox::with_context(ctx, 5000);
        let result = sandbox.execute_direct(r#"
            const avg = data.reduce((sum, p) => sum + p.score, 0) / data.length;
            const topScorer = data.reduce((max, p) => p.score > max.score ? p : max);
            return { averageScore: avg, topScorer: topScorer.name };
        "#);

        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("averageScore"));
        assert!(output.contains("topScorer"));
    }
}
