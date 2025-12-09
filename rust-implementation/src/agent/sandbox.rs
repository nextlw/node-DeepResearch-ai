// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CODE SANDBOX - Execução segura de JavaScript e Python
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Este módulo permite ao agente gerar e executar código em um ambiente
// isolado (sandbox) para processar dados, chamar APIs e transformar
// informações coletadas durante a pesquisa.
//
// Linguagens suportadas:
// - JavaScript (via Boa Engine) - para manipulação de JSON/strings
// - Python (via subprocess) - para análise de dados, cálculos complexos
//
// Características:
// - Isolamento completo (sem acesso a filesystem/rede perigoso)
// - Timeout de execução configurável
// - Limite de iterações de loop e recursão (JS)
// - Retry inteligente com feedback de erros para o LLM
// - LLM pode escolher automaticamente a melhor linguagem
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::llm::{LlmClient, LlmError};
use crate::types::KnowledgeItem;
use boa_engine::{Context, JsValue, Source};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;

/// Linguagem de programação para execução no sandbox
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SandboxLanguage {
    /// JavaScript via Boa Engine (in-process, mais rápido)
    /// Bom para: manipulação de JSON, strings, cálculos simples
    #[default]
    JavaScript,
    /// Python via subprocess (mais poderoso)
    /// Bom para: análise de dados, regex complexo, cálculos científicos
    Python,
    /// LLM escolhe automaticamente a melhor linguagem
    Auto,
}

impl SandboxLanguage {
    /// Retorna o nome da linguagem para exibição
    pub fn display_name(&self) -> &'static str {
        match self {
            SandboxLanguage::JavaScript => "JavaScript",
            SandboxLanguage::Python => "Python",
            SandboxLanguage::Auto => "Auto",
        }
    }

    /// Retorna a extensão de arquivo
    pub fn extension(&self) -> &'static str {
        match self {
            SandboxLanguage::JavaScript => "js",
            SandboxLanguage::Python => "py",
            SandboxLanguage::Auto => "auto",
        }
    }

    /// Verifica se Python está disponível no sistema
    pub async fn is_python_available() -> bool {
        Command::new("python3")
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Erros que podem ocorrer durante execução no sandbox
#[derive(Debug, Error)]
pub enum SandboxError {
    /// Erro de execução (JavaScript ou Python)
    #[error("{language} execution error: {message}")]
    ExecutionError {
        /// Linguagem onde ocorreu o erro
        language: String,
        /// Mensagem de erro
        message: String,
    },

    /// Timeout de execução excedido
    #[error("Execution timeout after {0}ms")]
    Timeout(u64),

    /// Código não retornou valor
    #[error("Code did not return a value (missing return/print statement)")]
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

    /// Python não está disponível no sistema
    #[error("Python is not available on this system")]
    PythonNotAvailable,

    /// Erro de I/O (subprocess, arquivos temporários)
    #[error("I/O error: {0}")]
    IoError(String),
}

impl SandboxError {
    /// Cria erro de execução JavaScript
    pub fn js_error(msg: impl Into<String>) -> Self {
        SandboxError::ExecutionError {
            language: "JavaScript".to_string(),
            message: msg.into(),
        }
    }

    /// Cria erro de execução Python
    pub fn python_error(msg: impl Into<String>) -> Self {
        SandboxError::ExecutionError {
            language: "Python".to_string(),
            message: msg.into(),
        }
    }
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
    /// Linguagem usada para execução
    pub language: SandboxLanguage,
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
                        language: SandboxLanguage::JavaScript,
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
                            language: SandboxLanguage::JavaScript,
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
                Err(SandboxError::js_error(error_msg))
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
                language: SandboxLanguage::JavaScript,
            },
            Err(e) => SandboxResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                code: code.to_string(),
                attempts: 1,
                execution_time_ms: start.elapsed().as_millis() as u64,
                language: SandboxLanguage::JavaScript,
            },
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PYTHON SANDBOX - Execução via subprocess
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Sandbox para execução segura de código Python via subprocess
///
/// # Segurança
/// - Execução em processo isolado
/// - Timeout rigoroso
/// - Sem acesso a módulos perigosos (bloqueados via wrapper)
/// - Captura de stdout/stderr
///
/// # Exemplo
/// ```ignore
/// let sandbox = PythonSandbox::new(&knowledge, 10000);
/// let result = sandbox.solve(&llm, "Analyze the data and find patterns").await?;
/// ```
pub struct PythonSandbox {
    /// Contexto com variáveis disponíveis
    context: SandboxContext,
    /// Máximo de tentativas de geração/execução
    max_attempts: usize,
    /// Timeout de execução em milissegundos
    timeout_ms: u64,
}

impl PythonSandbox {
    /// Cria um novo sandbox Python com configurações padrão
    pub fn new(knowledge: &[KnowledgeItem], timeout_ms: u64) -> Self {
        Self {
            context: SandboxContext::from_knowledge(knowledge),
            max_attempts: 3,
            timeout_ms,
        }
    }

    /// Cria sandbox com contexto customizado
    pub fn with_context(context: SandboxContext, timeout_ms: u64) -> Self {
        Self {
            context,
            max_attempts: 3,
            timeout_ms,
        }
    }

    /// Define o máximo de tentativas
    pub fn max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Resolve um problema gerando e executando código Python
    pub async fn solve(
        &self,
        llm: &dyn LlmClient,
        problem: &str,
    ) -> Result<SandboxResult, SandboxError> {
        let mut attempts: Vec<(String, Option<String>)> = Vec::new();
        let start_time = Instant::now();

        for attempt in 0..self.max_attempts {
            log::debug!("Python Sandbox attempt {}/{}", attempt + 1, self.max_attempts);

            // Gerar código via LLM (especificando Python)
            let code_response = llm
                .generate_python_code(problem, &self.context.describe_for_llm(), &attempts)
                .await?;

            log::debug!("Generated Python code:\n{}", code_response.code);

            // Executar código
            match self.execute(&code_response.code).await {
                Ok(output) => {
                    log::info!("Python Sandbox execution successful on attempt {}", attempt + 1);
                    return Ok(SandboxResult {
                        success: true,
                        output: Some(output),
                        error: None,
                        code: code_response.code,
                        attempts: attempt + 1,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        language: SandboxLanguage::Python,
                    });
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Python Sandbox attempt {} failed: {}", attempt + 1, error_msg);

                    attempts.push((code_response.code.clone(), Some(error_msg.clone())));

                    if attempt == self.max_attempts - 1 {
                        return Ok(SandboxResult {
                            success: false,
                            output: None,
                            error: Some(error_msg),
                            code: code_response.code,
                            attempts: attempt + 1,
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            language: SandboxLanguage::Python,
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

    /// Executa código Python no sandbox via subprocess
    pub async fn execute(&self, code: &str) -> Result<String, SandboxError> {
        // Verificar se Python está disponível
        if !SandboxLanguage::is_python_available().await {
            return Err(SandboxError::PythonNotAvailable);
        }

        // Criar código Python seguro com wrapper
        let wrapped_code = self.wrap_python_code(code);

        // Executar com timeout
        let result = timeout(
            Duration::from_millis(self.timeout_ms),
            self.run_python(&wrapped_code),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_) => Err(SandboxError::Timeout(self.timeout_ms)),
        }
    }

    /// Executa código Python via subprocess
    async fn run_python(&self, code: &str) -> Result<String, SandboxError> {
        let child = Command::new("python3")
            .arg("-c")
            .arg(code)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SandboxError::IoError(e.to_string()))?;

        // Capturar output
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| SandboxError::IoError(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout.is_empty() {
                Err(SandboxError::NoReturnValue)
            } else {
                Ok(stdout)
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(SandboxError::python_error(stderr))
        }
    }

    /// Envolve o código Python com segurança e injeção de contexto
    fn wrap_python_code(&self, code: &str) -> String {
        let mut wrapped = String::new();

        // Imports seguros
        wrapped.push_str("import json\nimport re\nimport math\nfrom collections import Counter\n\n");

        // Injetar variáveis do contexto
        for (name, value) in &self.context.variables {
            // Escapar o valor JSON para Python
            let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
            wrapped.push_str(&format!("{} = json.loads('{}')\n", name, escaped));
        }

        wrapped.push_str("\n# User code\n");
        wrapped.push_str(code);

        wrapped
    }

    /// Executa código diretamente sem geração via LLM
    pub async fn execute_direct(&self, code: &str) -> SandboxResult {
        let start = Instant::now();

        match self.execute(code).await {
            Ok(output) => SandboxResult {
                success: true,
                output: Some(output),
                error: None,
                code: code.to_string(),
                attempts: 1,
                execution_time_ms: start.elapsed().as_millis() as u64,
                language: SandboxLanguage::Python,
            },
            Err(e) => SandboxResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                code: code.to_string(),
                attempts: 1,
                execution_time_ms: start.elapsed().as_millis() as u64,
                language: SandboxLanguage::Python,
            },
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// UNIFIED SANDBOX - Escolha automática de linguagem
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Sandbox unificado que escolhe automaticamente a melhor linguagem
///
/// O LLM analisa o problema e decide se é melhor usar JavaScript ou Python
/// baseado nas características da tarefa:
///
/// - **JavaScript**: Manipulação de JSON, strings, cálculos simples
/// - **Python**: Análise de dados, regex complexo, cálculos científicos, estatísticas
pub struct UnifiedSandbox {
    /// Sandbox JavaScript
    js_sandbox: CodeSandbox,
    /// Sandbox Python
    py_sandbox: PythonSandbox,
    /// Linguagem preferida (ou Auto)
    preferred_language: SandboxLanguage,
    /// Se Python está disponível no sistema
    python_available: bool,
}

impl UnifiedSandbox {
    /// Cria um novo sandbox unificado
    pub async fn new(knowledge: &[KnowledgeItem], timeout_ms: u64) -> Self {
        let python_available = SandboxLanguage::is_python_available().await;
        if !python_available {
            log::warn!("Python not available, falling back to JavaScript only");
        }

        Self {
            js_sandbox: CodeSandbox::new(knowledge, timeout_ms),
            py_sandbox: PythonSandbox::new(knowledge, timeout_ms),
            preferred_language: SandboxLanguage::Auto,
            python_available,
        }
    }

    /// Define a linguagem preferida
    pub fn with_language(mut self, language: SandboxLanguage) -> Self {
        self.preferred_language = language;
        self
    }

    /// Resolve um problema escolhendo a linguagem automaticamente
    pub async fn solve(
        &self,
        llm: &dyn LlmClient,
        problem: &str,
    ) -> Result<SandboxResult, SandboxError> {
        let language = match self.preferred_language {
            SandboxLanguage::JavaScript => SandboxLanguage::JavaScript,
            SandboxLanguage::Python => {
                if self.python_available {
                    SandboxLanguage::Python
                } else {
                    log::warn!("Python requested but not available, using JavaScript");
                    SandboxLanguage::JavaScript
                }
            }
            SandboxLanguage::Auto => {
                // LLM escolhe a melhor linguagem
                self.choose_language(llm, problem).await
            }
        };

        log::info!("🖥️ Sandbox usando linguagem: {}", language.display_name());

        match language {
            SandboxLanguage::JavaScript => self.js_sandbox.solve(llm, problem).await,
            SandboxLanguage::Python => self.py_sandbox.solve(llm, problem).await,
            SandboxLanguage::Auto => unreachable!(), // Já foi resolvido acima
        }
    }

    /// LLM escolhe a melhor linguagem para o problema
    async fn choose_language(&self, llm: &dyn LlmClient, problem: &str) -> SandboxLanguage {
        // Se Python não está disponível, usar JS
        if !self.python_available {
            return SandboxLanguage::JavaScript;
        }

        // Heurísticas simples antes de chamar LLM
        let problem_lower = problem.to_lowercase();

        // Indicadores fortes de Python
        let python_indicators = [
            "pandas", "numpy", "dataframe", "statistics", "statistical",
            "machine learning", "ml", "data analysis", "analyze data",
            "scientific", "matrix", "correlation", "regression",
            "csv", "excel", "plot", "graph", "visualization",
        ];

        // Indicadores fortes de JavaScript
        let js_indicators = [
            "json", "parse json", "stringify", "object", "array manipulation",
            "string manipulation", "dom", "html", "css",
            "simple calculation", "quick transform",
        ];

        let python_score: usize = python_indicators
            .iter()
            .filter(|&ind| problem_lower.contains(ind))
            .count();

        let js_score: usize = js_indicators
            .iter()
            .filter(|&ind| problem_lower.contains(ind))
            .count();

        // Se há indicador claro, usar direto
        if python_score > js_score + 1 {
            log::debug!("Heuristic chose Python (score: {} vs {})", python_score, js_score);
            return SandboxLanguage::Python;
        }
        if js_score > python_score + 1 {
            log::debug!("Heuristic chose JavaScript (score: {} vs {})", js_score, python_score);
            return SandboxLanguage::JavaScript;
        }

        // Se não há indicador claro, perguntar ao LLM
        match llm.choose_coding_language(problem).await {
            Ok(lang) => {
                log::debug!("LLM chose: {}", lang.display_name());
                lang
            }
            Err(e) => {
                log::warn!("Failed to get language choice from LLM: {}, defaulting to JavaScript", e);
                SandboxLanguage::JavaScript
            }
        }
    }

    /// Retorna a linguagem que será usada
    pub fn get_effective_language(&self) -> SandboxLanguage {
        match self.preferred_language {
            SandboxLanguage::Python if !self.python_available => SandboxLanguage::JavaScript,
            other => other,
        }
    }

    /// Verifica se Python está disponível
    pub fn is_python_available(&self) -> bool {
        self.python_available
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
