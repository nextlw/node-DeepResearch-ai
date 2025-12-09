//! Estado da aplicação TUI

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

/// Nível de severidade do log
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    /// Informação geral
    Info,
    /// Operação bem sucedida
    Success,
    /// Aviso
    Warning,
    /// Erro
    Error,
    /// Debug
    Debug,
}

impl LogLevel {
    /// Retorna o símbolo emoji do nível
    pub fn symbol(&self) -> &'static str {
        match self {
            LogLevel::Info => "ℹ️ ",
            LogLevel::Success => "✅",
            LogLevel::Warning => "⚠️ ",
            LogLevel::Error => "❌",
            LogLevel::Debug => "🔍",
        }
    }
}

/// Entrada de log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp formatado
    pub timestamp: String,
    /// Nível do log
    pub level: LogLevel,
    /// Mensagem
    pub message: String,
}

impl LogEntry {
    /// Cria nova entrada de log
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let now = chrono::Local::now();
        Self {
            timestamp: now.format("%H:%M:%S").to_string(),
            level,
            message: message.into(),
        }
    }

    /// Log de informação
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }

    /// Log de sucesso
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Success, message)
    }

    /// Log de aviso
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warning, message)
    }

    /// Log de erro
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }
}

/// Estatísticas de uma persona
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonaStats {
    /// Nome da persona
    pub name: String,
    /// Número de buscas
    pub searches: usize,
    /// Número de leituras
    pub reads: usize,
    /// Número de respostas geradas
    pub answers: usize,
    /// Tokens consumidos
    pub tokens: u64,
    /// Se está ativa agora
    #[serde(skip)]
    pub is_active: bool,
}

/// Métricas do sistema
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Threads ativas
    pub threads: usize,
    /// Uso de memória em MB
    pub memory_mb: f64,
    /// CPU %
    pub cpu_percent: f32,
}

/// Estado do AgentAnalyzer (análise de erros em background)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentAnalyzerState {
    /// Se está ativo (análise em andamento)
    pub is_active: bool,
    /// Número de falhas que dispararam a análise
    pub failures_count: usize,
    /// Entradas do diário sendo analisadas
    pub diary_entries: usize,
    /// Timestamp de início
    pub started_at: Option<String>,
    /// Último recap (resumo)
    pub last_recap: Option<String>,
    /// Última blame (culpa)
    pub last_blame: Option<String>,
    /// Última melhoria sugerida
    pub last_improvement: Option<String>,
    /// Tempo de execução em ms
    pub duration_ms: Option<u128>,
    /// Logs específicos do analyzer
    pub logs: Vec<LogEntry>,
}

/// Execução de sandbox completada (para histórico)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecution {
    /// Problema/tarefa resolvido
    pub problem: String,
    /// Linguagem de programação usada
    pub language: String,
    /// Se foi bem-sucedido
    pub success: bool,
    /// Número de tentativas
    pub attempts: usize,
    /// Tempo de execução em ms
    pub execution_time_ms: u64,
    /// Output da execução (se sucesso)
    pub output: Option<String>,
    /// Erro da execução (se falha)
    pub error: Option<String>,
    /// Preview do código final
    pub code_preview: String,
    /// Timestamp de conclusão
    pub completed_at: String,
}

/// Estado do Sandbox de execução de código
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxState {
    /// Se está ativo (execução em andamento)
    pub is_active: bool,
    /// Problema/tarefa sendo resolvido
    pub problem: String,
    /// Tentativa atual (1-based)
    pub current_attempt: usize,
    /// Máximo de tentativas
    pub max_attempts: usize,
    /// Status atual: "idle", "generating", "executing", "success", "error"
    pub status: String,
    /// Preview do código sendo executado
    pub code_preview: String,
    /// Output da execução (se sucesso)
    pub output: Option<String>,
    /// Erro da execução (se falha)
    pub error: Option<String>,
    /// Tempo de execução em ms
    pub execution_time_ms: u64,
    /// Timeout configurado em ms
    pub timeout_ms: u64,
    /// Timestamp de início
    pub started_at: Option<String>,
    /// Linguagem de programação (JavaScript, Python)
    pub language: String,
    /// Logs específicos do sandbox
    pub logs: Vec<LogEntry>,
    /// Histórico de execuções completadas
    #[serde(default)]
    pub executions: Vec<SandboxExecution>,
}

/// Estado dos Benchmarks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarksState {
    /// Benchmarks disponíveis
    pub available: Vec<BenchmarkInfo>,
    /// Índice do benchmark selecionado
    pub selected: Option<usize>,
    /// Benchmark em execução
    pub running: Option<String>,
    /// Resultado do último benchmark executado
    pub last_result: Option<BenchmarkResult>,
    /// Logs da execução atual
    pub execution_logs: VecDeque<LogEntry>,
    /// Scroll position nos logs
    pub log_scroll: usize,
    /// Resultados dinâmicos do benchmark atual
    #[serde(default)]
    pub dynamic_results: BenchmarkDynamicResults,
    /// Scroll position nos resultados dinâmicos
    #[serde(default)]
    pub results_scroll: usize,
}

/// Informação sobre um benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkInfo {
    /// Nome do benchmark
    pub name: String,
    /// Descrição
    pub description: String,
    /// Nome do arquivo de benchmark
    pub bench_file: String,
}

/// Resultado de um benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Nome do benchmark
    pub name: String,
    /// Timestamp de início
    pub started_at: String,
    /// Timestamp de fim
    pub finished_at: String,
    /// Duração em segundos
    pub duration_secs: f64,
    /// Saída do benchmark
    pub output: String,
    /// Se foi bem-sucedido
    pub success: bool,
    /// Erro (se houver)
    pub error: Option<String>,
}

/// Status de um campo de resultado dinâmico
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldStatus {
    /// Aguardando valor
    Pending,
    /// Processando/coletando dados
    Running,
    /// Valor obtido com sucesso
    Success,
    /// Falhou ao obter valor
    Failed,
    /// Valor é informativo (sem status de sucesso/falha)
    Info,
}

impl Default for FieldStatus {
    fn default() -> Self {
        FieldStatus::Pending
    }
}

/// Campo de resultado dinâmico do benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkDynamicField {
    /// ID único do campo (para atualização)
    pub id: String,
    /// Label/nome do campo a ser exibido
    pub label: String,
    /// Valor atual (None se ainda não obtido)
    pub value: Option<String>,
    /// Status do campo
    pub status: FieldStatus,
    /// Ícone/emoji para o campo (opcional)
    pub icon: Option<String>,
    /// Ordem de exibição (menor = mais acima)
    pub order: usize,
    /// Grupo/categoria do campo (para agrupar visualmente)
    pub group: Option<String>,
}

impl BenchmarkDynamicField {
    /// Cria um novo campo pendente
    pub fn new(id: impl Into<String>, label: impl Into<String>, order: usize) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: None,
            status: FieldStatus::Pending,
            icon: None,
            order,
            group: None,
        }
    }

    /// Define um ícone para o campo
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Define o grupo do campo
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// Resultados dinâmicos de um benchmark
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarkDynamicResults {
    /// Nome do benchmark
    pub bench_name: String,
    /// Campos de resultado
    pub fields: Vec<BenchmarkDynamicField>,
    /// Se todos os campos foram preenchidos
    pub is_complete: bool,
    /// Timestamp de início
    pub started_at: Option<String>,
    /// Timestamp de última atualização
    pub last_update: Option<String>,
}

impl BenchmarkDynamicResults {
    /// Cria novos resultados dinâmicos
    pub fn new(bench_name: impl Into<String>) -> Self {
        Self {
            bench_name: bench_name.into(),
            fields: Vec::new(),
            is_complete: false,
            started_at: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
            last_update: None,
        }
    }

    /// Define o schema de campos esperados
    pub fn set_schema(&mut self, fields: Vec<BenchmarkDynamicField>) {
        self.fields = fields;
        self.is_complete = false;
    }

    /// Atualiza um campo específico
    pub fn update_field(&mut self, field_id: &str, value: String, status: FieldStatus) {
        if let Some(field) = self.fields.iter_mut().find(|f| f.id == field_id) {
            field.value = Some(value);
            field.status = status;
            self.last_update = Some(chrono::Local::now().format("%H:%M:%S").to_string());
        }
        // Verificar se todos os campos foram preenchidos
        self.is_complete = self.fields.iter().all(|f| {
            f.value.is_some() && !matches!(f.status, FieldStatus::Pending | FieldStatus::Running)
        });
    }

    /// Marca um campo como em execução
    pub fn set_field_running(&mut self, field_id: &str) {
        if let Some(field) = self.fields.iter_mut().find(|f| f.id == field_id) {
            field.status = FieldStatus::Running;
            self.last_update = Some(chrono::Local::now().format("%H:%M:%S").to_string());
        }
    }

    /// Retorna os campos ordenados
    pub fn sorted_fields(&self) -> Vec<&BenchmarkDynamicField> {
        let mut fields: Vec<_> = self.fields.iter().collect();
        fields.sort_by_key(|f| f.order);
        fields
    }

    /// Limpa os resultados
    pub fn clear(&mut self) {
        self.fields.clear();
        self.is_complete = false;
        self.started_at = None;
        self.last_update = None;
    }
}

impl BenchmarksState {
    /// Cria novo estado de benchmarks
    pub fn new() -> Self {
        let mut state = Self {
            available: Vec::new(),
            selected: None,
            running: None,
            last_result: None,
            execution_logs: VecDeque::new(),
            log_scroll: 0,
            dynamic_results: BenchmarkDynamicResults::default(),
            results_scroll: 0,
        };
        state.load_available_benchmarks();
        state
    }

    /// Carrega lista de benchmarks disponíveis
    fn load_available_benchmarks(&mut self) {
        self.available = vec![
            BenchmarkInfo {
                name: "Personas".to_string(),
                description: "Performance de criação e expansão de queries pelas personas".to_string(),
                bench_file: "personas_bench".to_string(),
            },
            BenchmarkInfo {
                name: "Search".to_string(),
                description: "Performance de buscas e cache de resultados".to_string(),
                bench_file: "search_bench".to_string(),
            },
            BenchmarkInfo {
                name: "Evaluation".to_string(),
                description: "Performance de avaliação de respostas".to_string(),
                bench_file: "evaluation_bench".to_string(),
            },
            BenchmarkInfo {
                name: "Agent".to_string(),
                description: "Performance do agente e gerenciamento de estado".to_string(),
                bench_file: "agent_bench".to_string(),
            },
            BenchmarkInfo {
                name: "E2E".to_string(),
                description: "Benchmark end-to-end completo do sistema".to_string(),
                bench_file: "e2e_bench".to_string(),
            },
            BenchmarkInfo {
                name: "SIMD".to_string(),
                description: "Performance de similaridade cosseno com SIMD".to_string(),
                bench_file: "simd_bench".to_string(),
            },
        ];
    }

    /// Seleciona próximo benchmark
    pub fn select_next(&mut self) {
        if self.available.is_empty() {
            return;
        }
        let new_idx = match self.selected {
            Some(idx) if idx < self.available.len() - 1 => idx + 1,
            Some(_) => 0,
            None => 0,
        };
        self.selected = Some(new_idx);
    }

    /// Seleciona benchmark anterior
    pub fn select_prev(&mut self) {
        if self.available.is_empty() {
            return;
        }
        let new_idx = match self.selected {
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => self.available.len() - 1,
            None => 0,
        };
        self.selected = Some(new_idx);
    }

    /// Retorna o benchmark selecionado
    pub fn get_selected(&self) -> Option<&BenchmarkInfo> {
        self.selected.and_then(|idx| self.available.get(idx))
    }

    /// Scroll up nos logs
    pub fn scroll_up(&mut self) {
        self.log_scroll = self.log_scroll.saturating_sub(1);
    }

    /// Scroll down nos logs
    pub fn scroll_down(&mut self) {
        let max_scroll = self.execution_logs.len().saturating_sub(10);
        if self.log_scroll < max_scroll {
            self.log_scroll += 1;
        }
    }

    /// Inicia execução de um benchmark
    pub fn start_benchmark(&mut self, bench_file: &str, bench_name: &str) {
        self.running = Some(bench_file.to_string());
        self.execution_logs.clear();
        self.log_scroll = 0;
        self.dynamic_results = BenchmarkDynamicResults::new(bench_name);
        self.results_scroll = 0;
        self.execution_logs.push_back(LogEntry::info(
            format!("Iniciando benchmark: {}", bench_file)
        ));
    }

    /// Define o schema de resultados dinâmicos
    pub fn set_results_schema(&mut self, fields: Vec<BenchmarkDynamicField>) {
        self.dynamic_results.set_schema(fields);
    }

    /// Atualiza um campo de resultado dinâmico
    pub fn update_result_field(&mut self, field_id: &str, value: String, status: FieldStatus) {
        self.dynamic_results.update_field(field_id, value, status);
    }

    /// Marca um campo como em execução
    pub fn set_result_field_running(&mut self, field_id: &str) {
        self.dynamic_results.set_field_running(field_id);
    }

    /// Scroll up nos resultados dinâmicos
    pub fn results_scroll_up(&mut self) {
        self.results_scroll = self.results_scroll.saturating_sub(1);
    }

    /// Scroll down nos resultados dinâmicos
    pub fn results_scroll_down(&mut self) {
        let max_scroll = self.dynamic_results.fields.len().saturating_sub(5);
        if self.results_scroll < max_scroll {
            self.results_scroll += 1;
        }
    }

    /// Finaliza execução de um benchmark com resultado
    pub fn finish_benchmark(&mut self, result: BenchmarkResult) {
        self.running = None;
        self.last_result = Some(result);
    }

    /// Adiciona log de execução
    pub fn add_execution_log(&mut self, level: LogLevel, message: String) {
        self.execution_logs.push_back(LogEntry::new(level, message));
        // Auto-scroll para o final
        if self.execution_logs.len() > 10 {
            self.log_scroll = self.execution_logs.len().saturating_sub(10);
        }
    }
}

impl SandboxState {
    /// Inicia uma nova execução de sandbox
    pub fn start(&mut self, problem: String, max_attempts: usize, timeout_ms: u64, language: String) {
        self.is_active = true;
        self.problem = problem;
        self.current_attempt = 0;
        self.max_attempts = max_attempts;
        self.status = "generating".to_string();
        self.code_preview.clear();
        self.output = None;
        self.error = None;
        self.execution_time_ms = 0;
        self.timeout_ms = timeout_ms;
        self.language = language.clone();
        self.started_at = Some(chrono::Local::now().format("%H:%M:%S").to_string());
        self.logs.clear();

        let lang_emoji = if language == "Python" { "🐍" } else { "📜" };
        self.logs.push(LogEntry::new(
            LogLevel::Info,
            format!("{} Sandbox {} iniciado: {}", lang_emoji, language, if self.problem.len() > 50 {
                format!("{}...", &self.problem[..50])
            } else {
                self.problem.clone()
            }),
        ));
    }

    /// Atualiza uma tentativa
    pub fn update_attempt(&mut self, attempt: usize, code_preview: String, status: String, error: Option<String>) {
        self.current_attempt = attempt;
        self.code_preview = code_preview;
        self.status = status.clone();
        if let Some(e) = &error {
            self.logs.push(LogEntry::new(LogLevel::Warning, format!("❌ Tentativa {}: {}", attempt, e)));
        } else if status == "executing" {
            self.logs.push(LogEntry::new(LogLevel::Info, format!("🔄 Tentativa {}/{}: Executando {}...", attempt, self.max_attempts, self.language)));
        }
    }

    /// Completa a execução
    pub fn complete(&mut self, success: bool, output: Option<String>, error: Option<String>, attempts: usize, execution_time_ms: u64, code_preview: String, language: String) {
        self.is_active = false;
        self.current_attempt = attempts;
        self.status = if success { "success".to_string() } else { "error".to_string() };
        self.output = output.clone();
        self.error = error.clone();
        self.execution_time_ms = execution_time_ms;
        self.code_preview = code_preview.clone();
        self.language = language.clone();

        let lang_emoji = if language == "Python" { "🐍" } else { "📜" };
        if success {
            self.logs.push(LogEntry::new(
                LogLevel::Success,
                format!("✅ {} Sucesso em {} tentativa(s), {}ms", lang_emoji, attempts, execution_time_ms),
            ));
            if let Some(out) = &output {
                let preview = if out.len() > 100 { format!("{}...", &out[..100]) } else { out.clone() };
                self.logs.push(LogEntry::new(LogLevel::Info, format!("📤 Output: {}", preview)));
            }
        } else if let Some(e) = &error {
            self.logs.push(LogEntry::new(LogLevel::Error, format!("❌ {} Falhou após {} tentativa(s): {}", lang_emoji, attempts, e)));
        }

        // Salvar no histórico de execuções
        self.executions.push(SandboxExecution {
            problem: self.problem.clone(),
            language,
            success,
            attempts,
            execution_time_ms,
            output,
            error,
            code_preview,
            completed_at: chrono::Local::now().format("%H:%M:%S").to_string(),
        });
    }

    /// Reseta o estado
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Estado de uma tarefa paralela
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Aguardando início
    Pending,
    /// Em execução
    Running,
    /// Concluída com sucesso
    Completed,
    /// Falhou
    Failed(String),
}

/// Método de leitura usado
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReadMethod {
    /// Jina API Reader
    Jina,
    /// Sistema local (Rust + LLM)
    RustLocal,
    /// Leitura de arquivo local
    FileRead,
    /// Não especificado
    Unknown,
}

impl std::fmt::Display for ReadMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadMethod::Jina => write!(f, "Jina API"),
            ReadMethod::RustLocal => write!(f, "Rust+LLM"),
            ReadMethod::FileRead => write!(f, "File"),
            ReadMethod::Unknown => write!(f, "???"),
        }
    }
}

/// Tarefa paralela sendo monitorada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTask {
    /// ID único da tarefa
    pub id: String,
    /// ID do batch (agrupa tarefas paralelas)
    pub batch_id: String,
    /// Tipo da tarefa (Read, Search, etc)
    pub task_type: String,
    /// Descrição/URL sendo processada
    pub description: String,
    /// Dados/variáveis alocados
    pub data_info: String,
    /// Status atual
    pub status: TaskStatus,
    /// Timestamp de início (ms desde epoch)
    pub started_at: u128,
    /// Tempo de execução em ms
    pub elapsed_ms: u128,
    /// Thread ID (se disponível)
    pub thread_id: Option<String>,
    /// Progresso em porcentagem (0-100)
    #[serde(default)]
    pub progress: u8,
    /// Método de leitura usado
    #[serde(default)]
    pub read_method: ReadMethod,
    /// Bytes processados
    #[serde(default)]
    pub bytes_processed: usize,
    /// Total de bytes esperado
    #[serde(default)]
    pub bytes_total: usize,
}

impl Default for ReadMethod {
    fn default() -> Self {
        ReadMethod::Unknown
    }
}

/// Batch de tarefas paralelas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelBatch {
    /// ID do batch
    pub id: String,
    /// Tipo do batch
    pub batch_type: String,
    /// Tarefas no batch
    pub tasks: Vec<ParallelTask>,
    /// Timestamp de início
    pub started_at: u128,
    /// Tempo total do batch
    pub total_elapsed_ms: u128,
    /// Quantas tarefas completaram
    pub completed: usize,
    /// Quantas falharam
    pub failed: usize,
}

/// Sessão de pesquisa completa (para salvar em JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSession {
    /// UUID único da sessão
    pub id: String,
    /// Timestamp de início (ISO 8601)
    pub started_at: String,
    /// Timestamp de fim (ISO 8601)
    pub finished_at: Option<String>,
    /// Pergunta pesquisada
    pub question: String,
    /// Resposta final
    pub answer: Option<String>,
    /// Referências encontradas
    pub references: Vec<String>,
    /// URLs visitadas
    pub visited_urls: Vec<String>,
    /// Logs da sessão
    pub logs: Vec<LogEntry>,
    /// Estatísticas por persona
    pub personas: HashMap<String, PersonaStats>,
    /// Estatísticas de tempo
    pub timing: SessionTiming,
    /// Estatísticas gerais
    pub stats: SessionStats,
    /// Se teve sucesso
    pub success: bool,
    /// Mensagem de erro (se houver)
    pub error: Option<String>,
    /// Batches de tarefas paralelas executados
    #[serde(default)]
    pub parallel_batches: Vec<ParallelBatch>,
    /// Todas as tarefas paralelas
    #[serde(default)]
    pub all_tasks: Vec<ParallelTask>,
    /// Steps completados
    #[serde(default)]
    pub completed_steps: Vec<CompletedStep>,
    /// Execuções de sandbox (código executado)
    #[serde(default)]
    pub sandbox_executions: Vec<SandboxExecution>,
}

/// Estatísticas de tempo da sessão
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionTiming {
    /// Tempo total em ms
    pub total_ms: u128,
    /// Tempo de busca em ms
    pub search_ms: u128,
    /// Tempo de leitura em ms
    pub read_ms: u128,
    /// Tempo de LLM em ms
    pub llm_ms: u128,
}

/// Estatísticas gerais da sessão
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    /// Número de steps
    pub steps: usize,
    /// URLs encontradas
    pub urls_found: usize,
    /// URLs visitadas
    pub urls_visited: usize,
    /// Tokens utilizados
    pub tokens_used: u64,
}

/// Tab ativa na navegação
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    /// Tab de pesquisa (Input/Research/Result)
    #[default]
    Search,
    /// Tab de configurações
    Config,
    /// Tab de benchmarks
    Benchmarks,
}

impl ActiveTab {
    /// Retorna o índice da tab (para widget Tabs)
    pub fn index(&self) -> usize {
        match self {
            ActiveTab::Search => 0,
            ActiveTab::Config => 1,
            ActiveTab::Benchmarks => 2,
        }
    }

    /// Cria tab a partir do índice
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => ActiveTab::Search,
            1 => ActiveTab::Config,
            2 => ActiveTab::Benchmarks,
            _ => ActiveTab::Search,
        }
    }

    /// Próxima tab (cyclic)
    pub fn next(&self) -> Self {
        match self {
            ActiveTab::Search => ActiveTab::Config,
            ActiveTab::Config => ActiveTab::Benchmarks,
            ActiveTab::Benchmarks => ActiveTab::Search,
        }
    }

    /// Tab anterior (cyclic)
    pub fn prev(&self) -> Self {
        match self {
            ActiveTab::Search => ActiveTab::Benchmarks,
            ActiveTab::Config => ActiveTab::Search,
            ActiveTab::Benchmarks => ActiveTab::Config,
        }
    }
}

/// Configurações carregadas (snapshot para exibição na TUI)
///
/// Esta estrutura representa um snapshot das configurações atuais do sistema,
/// formatadas como strings para exibição na interface de usuário textual (TUI).
/// Os valores são calculados a partir das configurações reais e convertidos
/// para representações legíveis pelo usuário.
#[derive(Debug, Clone, Default)]
pub struct LoadedConfig {
    // Runtime
    /// Número de threads de trabalho configuradas, formatado como string.
    ///
    /// Pode ser um número fixo (ex: "4") ou "auto (N)" quando o cálculo
    /// é dinâmico baseado no número de cores da CPU e `max_threads`.
    /// Este valor é usado apenas para exibição na TUI.
    pub worker_threads: String,

    /// Número máximo de threads permitidas para o runtime Tokio.
    ///
    /// Este valor define o limite superior quando o cálculo de threads
    /// é dinâmico. O valor efetivo será o mínimo entre o número de cores
    /// da CPU e este valor.
    pub max_threads: usize,

    /// Número máximo de threads bloqueantes permitidas.
    ///
    /// Threads bloqueantes são usadas para operações que podem bloquear
    /// a thread atual, como I/O síncrono ou operações de CPU intensivas.
    /// Este valor controla o tamanho do pool de threads bloqueantes do Tokio.
    pub max_blocking_threads: usize,

    /// Provedor de leitura web configurado, formatado como string.
    ///
    /// Indica qual biblioteca ou serviço está sendo usado para fazer
    /// requisições HTTP e extrair conteúdo de páginas web durante a pesquisa.
    pub webreader: String,

    // LLM
    /// Provedor do modelo de linguagem (LLM) configurado.
    ///
    /// Representa qual serviço de LLM está sendo usado, como "openai",
    /// "anthropic", "local", etc. Este valor é formatado como string
    /// para exibição na interface.
    pub llm_provider: String,

    /// Nome do modelo de linguagem específico sendo utilizado.
    ///
    /// Exemplos: "gpt-4", "gpt-3.5-turbo", "claude-3-opus", etc.
    /// Este é o modelo que será usado para gerar respostas e análises.
    pub llm_model: String,

    /// Provedor de embeddings configurado.
    ///
    /// Pode ser diferente do `llm_provider`, permitindo usar um serviço
    /// específico para gerar embeddings vetoriais. Exemplos: "openai", "jina".
    pub embedding_provider: String,

    /// Nome do modelo de embeddings específico sendo utilizado.
    ///
    /// Este modelo é usado para converter texto em vetores numéricos
    /// que permitem busca semântica e comparação de similaridade.
    pub embedding_model: String,

    /// Temperatura do modelo LLM para geração de texto.
    ///
    /// Controla a aleatoriedade das respostas geradas:
    /// - Valores baixos (0.0-0.3): respostas mais determinísticas e focadas
    /// - Valores médios (0.5-0.7): equilíbrio entre criatividade e precisão
    /// - Valores altos (0.8-1.0): respostas mais criativas e variadas
    pub temperature: f32,

    /// URL base da API customizada, se configurada.
    ///
    /// Quando `Some`, indica que está sendo usada uma URL customizada
    /// para a API do LLM, permitindo usar proxies ou servidores alternativos.
    /// Quando `None`, usa a URL padrão do provedor.
    pub api_base_url: Option<String>,

    // Agent
    /// Número mínimo de passos que o agente deve executar antes de
    /// considerar fornecer uma resposta final.
    ///
    /// Este valor garante que o agente realize uma pesquisa adequada
    /// antes de concluir, evitando respostas prematuras baseadas em
    /// informações insuficientes.
    pub min_steps_before_answer: usize,

    /// Se o agente pode fornecer uma resposta direta sem pesquisa.
    ///
    /// Quando `true`, o agente pode responder imediatamente se tiver
    /// confiança suficiente na resposta. Quando `false`, sempre realiza
    /// pesquisa mesmo para perguntas simples.
    pub allow_direct_answer: bool,

    /// Orçamento padrão de tokens para uma sessão de pesquisa.
    ///
    /// Define o limite máximo de tokens que podem ser consumidos durante
    /// uma pesquisa completa. Quando este limite é atingido, o agente
    /// deve finalizar a pesquisa e fornecer a melhor resposta possível
    /// com os recursos disponíveis.
    pub default_token_budget: u64,

    /// Número máximo de URLs que podem ser processadas em um único passo.
    ///
    /// Limita quantas páginas web podem ser acessadas e analisadas
    /// simultaneamente em cada iteração do processo de pesquisa,
    /// controlando o uso de recursos e tempo de resposta.
    pub max_urls_per_step: usize,

    /// Número máximo de consultas de busca que podem ser feitas por passo.
    ///
    /// Limita quantas consultas podem ser enviadas aos mecanismos de busca
    /// (como Google, Bing, etc.) em cada iteração, controlando custos
    /// e tempo de processamento.
    pub max_queries_per_step: usize,

    /// Número máximo de falhas consecutivas permitidas antes de abortar.
    ///
    /// Se o agente falhar (erro de API, timeout, etc.) este número
    /// de vezes seguidas, a pesquisa será interrompida para evitar
    /// loops infinitos ou consumo excessivo de recursos.
    pub max_consecutive_failures: usize,

    // API Keys (mascaradas)
    /// Indica se a chave da API OpenAI está presente no ambiente.
    ///
    /// Este valor é `true` se a variável de ambiente `OPENAI_API_KEY`
    /// está definida, independentemente do valor. Usado apenas para
    /// indicar na interface se as credenciais necessárias estão
    /// configuradas, sem expor os valores reais das chaves.
    pub openai_key_present: bool,

    /// Indica se a chave da API Jina está presente no ambiente.
    ///
    /// Este valor é `true` se a variável de ambiente `JINA_API_KEY`
    /// está definida. Jina é usado como provedor alternativo de embeddings,
    /// e esta flag indica se está disponível para uso.
    pub jina_key_present: bool,
}

/// Estado da tela
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppScreen {
    /// Tela de input da pergunta
    Input,
    /// Tela de pesquisa em andamento
    Research,
    /// Tela de resultado
    Result,
    /// Tela de configurações
    Config,
    /// Tela de benchmarks
    Benchmarks,
    /// Aguardando input do usuário (pergunta do agente)
    ///
    /// Compatível com OpenAI Responses API (input_required).
    InputRequired {
        /// ID da pergunta pendente
        question_id: String,
        /// Tipo da pergunta
        question_type: String,
        /// Texto da pergunta
        question: String,
        /// Opções de resposta (se aplicável)
        options: Option<Vec<String>>,
    },
}

/// Eventos que podem ser enviados para a TUI
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Novo log
    Log(LogEntry),
    /// Atualiza step atual
    SetStep(usize),
    /// Atualiza ação atual
    SetAction(String),
    /// Atualiza think atual
    SetThink(String),
    /// Atualiza contagem de URLs
    SetUrlCount(usize),
    /// Atualiza URLs visitadas
    SetVisitedCount(usize),
    /// Atualiza tokens
    SetTokens(u64),
    /// Define resposta final
    SetAnswer(String),
    /// Define referências
    SetReferences(Vec<String>),
    /// Atualiza métricas do sistema
    UpdateMetrics(SystemMetrics),
    /// Atualiza stats de persona
    UpdatePersona(PersonaStats),
    /// Define tempos detalhados (total, search, read, llm) em ms
    SetTimes {
        /// Tempo total de execução em milissegundos
        total_ms: u128,
        /// Tempo gasto em buscas em milissegundos
        search_ms: u128,
        /// Tempo gasto em leituras em milissegundos
        read_ms: u128,
        /// Tempo gasto em chamadas LLM em milissegundos
        llm_ms: u128,
    },
    /// Pesquisa concluída
    Complete,
    /// Erro fatal
    Error(String),
    /// Adiciona URL visitada
    AddVisitedUrl(String),
    /// Inicia um novo batch de tarefas paralelas.
    ///
    /// Usado para agrupar múltiplas operações assíncronas que serão
    /// executadas em paralelo, como buscas em múltiplas URLs ou
    /// processamento de múltiplos documentos.
    StartBatch {
        /// Identificador único do batch
        batch_id: String,
        /// Tipo do batch (ex: "search", "read", "process")
        batch_type: String,
        /// Número total de tarefas no batch
        task_count: usize,
    },
    /// Atualiza o estado de uma tarefa específica no batch atual.
    ///
    /// Permite rastrear o progresso individual de cada tarefa paralela,
    /// mostrando status como "pending", "running", "completed" ou "failed".
    UpdateTask(ParallelTask),
    /// Finaliza um batch de tarefas paralelas.
    ///
    /// Marca o término de todas as tarefas do batch, registrando
    /// estatísticas de execução como tempo total e contagem de
    /// sucessos/falhas.
    EndBatch {
        /// Identificador único do batch sendo finalizado
        batch_id: String,
        /// Tempo total de execução do batch em milissegundos
        total_ms: u128,
        /// Número de tarefas concluídas com sucesso
        success_count: usize,
        /// Número de tarefas que falharam
        fail_count: usize,
    },
    /// AgentAnalyzer iniciou análise em background
    AgentAnalyzerStarted {
        /// Número de falhas consecutivas que dispararam a análise
        failures_count: usize,
        /// Número de entradas do diário sendo analisadas
        diary_entries: usize,
    },
    /// AgentAnalyzer concluiu análise
    AgentAnalyzerCompleted {
        /// Resumo cronológico
        recap: String,
        /// Identificação do problema
        blame: String,
        /// Sugestões de melhoria
        improvement: String,
        /// Tempo de execução em ms
        duration_ms: u128,
    },
    /// Agente fez uma pergunta ao usuário (interação)
    ///
    /// Compatível com OpenAI Responses API (input_required).
    AgentQuestion {
        /// ID único da pergunta
        question_id: String,
        /// Tipo da pergunta (clarification, confirmation, preference, suggestion)
        question_type: String,
        /// Texto da pergunta
        question: String,
        /// Opções de resposta (se aplicável)
        options: Option<Vec<String>>,
        /// Se é blocking (agente pausado aguardando)
        is_blocking: bool,
    },
    /// Resposta do usuário enviada ao agente
    UserResponse {
        /// ID da pergunta respondida (None se espontânea)
        question_id: Option<String>,
        /// Conteúdo da resposta
        response: String,
    },
    /// Sandbox iniciou execução de código
    SandboxStart {
        /// Problema/tarefa sendo resolvido
        problem: String,
        /// Máximo de tentativas configurado
        max_attempts: usize,
        /// Timeout em ms
        timeout_ms: u64,
        /// Linguagem de programação (JavaScript, Python, Auto)
        language: String,
    },
    /// Sandbox - atualização de tentativa
    SandboxAttempt {
        /// Número da tentativa atual (1-based)
        attempt: usize,
        /// Máximo de tentativas
        max_attempts: usize,
        /// Preview do código sendo executado
        code_preview: String,
        /// Status: "generating", "executing", "success", "error"
        status: String,
        /// Mensagem de erro (se aplicável)
        error: Option<String>,
    },
    /// Sandbox concluiu execução
    SandboxComplete {
        /// Se foi bem-sucedido
        success: bool,
        /// Output da execução (se sucesso)
        output: Option<String>,
        /// Erro (se falha)
        error: Option<String>,
        /// Número total de tentativas
        attempts: usize,
        /// Tempo total de execução em ms
        execution_time_ms: u64,
        /// Preview do código final
        code_preview: String,
        /// Linguagem de programação usada
        language: String,
    },
    /// Benchmark iniciou execução
    BenchmarkStarted {
        /// Nome do arquivo de benchmark
        bench_file: String,
        /// Nome do benchmark
        bench_name: String,
    },
    /// Benchmark atualizou log
    BenchmarkLog {
        /// Mensagem do log
        message: String,
        /// Nível do log
        level: LogLevel,
    },
    /// Benchmark concluiu execução
    BenchmarkComplete {
        /// Nome do arquivo de benchmark
        bench_file: String,
        /// Nome do benchmark
        bench_name: String,
        /// Se foi bem-sucedido
        success: bool,
        /// Output completo
        output: String,
        /// Erro (se houver)
        error: Option<String>,
        /// Duração em segundos
        duration_secs: f64,
    },
    /// Define o schema de campos esperados para resultados dinâmicos
    BenchmarkSetSchema {
        /// Nome do benchmark
        bench_name: String,
        /// Campos esperados (JSON serializado)
        fields: Vec<BenchmarkDynamicField>,
    },
    /// Atualiza um campo de resultado dinâmico
    BenchmarkUpdateField {
        /// ID do campo
        field_id: String,
        /// Valor do campo
        value: String,
        /// Status do campo
        status: FieldStatus,
    },
    /// Marca um campo como em execução
    BenchmarkFieldRunning {
        /// ID do campo
        field_id: String,
    },
}

/// Estado da aplicação
pub struct App {
    /// UUID único da sessão atual
    pub session_id: String,
    /// Timestamp de início (ISO 8601)
    pub started_at: String,
    /// Tela atual
    pub screen: AppScreen,
    /// Tab ativa (para navegação entre seções)
    pub active_tab: ActiveTab,
    /// Tela anterior (para navegação entre Result <-> Research)
    pub previous_screen: Option<AppScreen>,
    /// Configurações carregadas (para exibição)
    pub loaded_config: LoadedConfig,
    /// Texto sendo digitado
    pub input_text: String,
    /// Posição do cursor no input
    pub cursor_pos: usize,
    /// Pergunta sendo pesquisada
    pub question: String,
    /// Step atual
    pub current_step: usize,
    /// Ação atual sendo executada
    pub current_action: String,
    /// Raciocínio atual do agente
    pub current_think: String,
    /// Logs da sessão (todos, sem limite)
    pub logs: VecDeque<LogEntry>,
    /// URLs encontradas
    pub url_count: usize,
    /// URLs visitadas (contagem)
    pub visited_count: usize,
    /// Lista de URLs visitadas
    pub visited_urls: Vec<String>,
    /// Tokens utilizados
    pub tokens_used: u64,
    /// Resposta final
    pub answer: Option<String>,
    /// Referências
    pub references: Vec<String>,
    /// Status de conclusão
    pub is_complete: bool,
    /// Mensagem de erro
    pub error: Option<String>,
    /// Tempo de início
    pub start_time: Option<Instant>,
    /// Tempo final (congelado quando completa)
    pub final_elapsed_secs: Option<f64>,
    /// Tempo total em ms
    pub total_time_ms: u128,
    /// Tempo de busca em ms
    pub search_time_ms: u128,
    /// Tempo de leitura em ms
    pub read_time_ms: u128,
    /// Tempo de LLM em ms
    pub llm_time_ms: u128,
    /// Scroll position dos logs
    pub log_scroll: usize,
    /// Se deve sair
    pub should_quit: bool,
    /// Métricas do sistema
    pub metrics: SystemMetrics,
    /// Stats por persona
    pub personas: HashMap<String, PersonaStats>,
    /// Histórico de perguntas
    pub history: Vec<String>,
    /// Índice no histórico (para input)
    pub history_index: Option<usize>,
    /// Scroll position na resposta final
    pub result_scroll: usize,
    /// Índice selecionado no histórico (para visualização)
    pub history_selected: Option<usize>,
    /// Sessões anteriores carregadas
    pub saved_sessions: Vec<ResearchSession>,
    /// Batches de tarefas paralelas em andamento
    pub active_batches: HashMap<String, ParallelBatch>,
    /// Histórico de batches completados
    pub completed_batches: Vec<ParallelBatch>,
    /// Todas as tarefas (para visualização)
    pub all_tasks: Vec<ParallelTask>,
    /// Histórico de steps completados
    pub completed_steps: Vec<CompletedStep>,
    /// Estado do AgentAnalyzer (análise de erros em background)
    pub agent_analyzer: AgentAnalyzerState,
    /// Estado do Sandbox de execução de código
    pub sandbox: SandboxState,
    /// Mensagem temporária do clipboard (feedback ao usuário)
    pub clipboard_message: Option<String>,
    /// Se o campo de input está focado (durante pesquisa)
    pub input_focused: bool,
    /// Contador de mensagens pendentes na fila para o agente
    pub pending_user_messages: usize,
    /// Fila de mensagens do usuário para enviar ao agente
    pub user_message_queue: VecDeque<String>,
    /// Estado dos benchmarks
    pub benchmarks: BenchmarksState,
}

/// Step completado para histórico
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStep {
    /// Número do step
    pub step_num: usize,
    /// Ação executada
    pub action: String,
    /// Raciocínio do agente
    pub think: String,
    /// Timestamp de conclusão
    pub completed_at: String,
    /// Status de sucesso
    pub success: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Cria nova instância da aplicação
    pub fn new() -> Self {
        let mut app = Self {
            session_id: Uuid::new_v4().to_string(),
            started_at: chrono::Local::now().to_rfc3339(),
            screen: AppScreen::Input,
            active_tab: ActiveTab::Search,
            previous_screen: None,
            loaded_config: LoadedConfig::default(),
            input_text: String::new(),
            cursor_pos: 0,
            question: String::new(),
            current_step: 0,
            current_action: "Aguardando...".into(),
            current_think: String::new(),
            logs: VecDeque::with_capacity(500),
            url_count: 0,
            visited_count: 0,
            visited_urls: Vec::new(),
            tokens_used: 0,
            answer: None,
            references: Vec::new(),
            is_complete: false,
            error: None,
            start_time: None,
            final_elapsed_secs: None,
            total_time_ms: 0,
            search_time_ms: 0,
            read_time_ms: 0,
            llm_time_ms: 0,
            log_scroll: 0,
            should_quit: false,
            metrics: SystemMetrics::default(),
            personas: HashMap::new(),
            history: Vec::new(),
            history_index: None,
            result_scroll: 0,
            history_selected: None,
            saved_sessions: Vec::new(),
            active_batches: HashMap::new(),
            completed_batches: Vec::new(),
            all_tasks: Vec::new(),
            completed_steps: Vec::new(),
            agent_analyzer: AgentAnalyzerState::default(),
            sandbox: SandboxState::default(),
            clipboard_message: None,
            input_focused: false,
            pending_user_messages: 0,
            user_message_queue: VecDeque::new(),
            benchmarks: BenchmarksState::new(),
        };
        // Carregar sessões anteriores
        app.load_sessions();
        app
    }

    /// Define as configurações carregadas (chamado pelo main)
    pub fn set_loaded_config(&mut self, config: LoadedConfig) {
        self.loaded_config = config;
    }

    /// Navega para a próxima tab
    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.sync_screen_with_tab();
    }

    /// Navega para a tab anterior
    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
        self.sync_screen_with_tab();
    }

    /// Vai para tab específica
    pub fn go_to_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
        self.sync_screen_with_tab();
    }

    /// Sincroniza a tela com a tab ativa
    fn sync_screen_with_tab(&mut self) {
        match self.active_tab {
            ActiveTab::Search => {
                // Voltar para tela de pesquisa apropriada
                if let Some(prev) = &self.previous_screen {
                    match prev {
                        AppScreen::Result => {
                            if self.is_complete {
                                self.screen = AppScreen::Result;
                            } else {
                                self.screen = AppScreen::Research;
                            }
                        }
                        AppScreen::Research => self.screen = AppScreen::Research,
                        _ => {
                            // Se estava em Input ou Config, decide baseado no estado
                            if self.is_complete && self.answer.is_some() {
                                self.screen = AppScreen::Result;
                            } else if self.start_time.is_some() {
                                self.screen = AppScreen::Research;
                            } else {
                                self.screen = AppScreen::Input;
                            }
                        }
                    }
                } else {
                    // Sem tela anterior, decide baseado no estado
                    if self.is_complete && self.answer.is_some() {
                        self.screen = AppScreen::Result;
                    } else if self.start_time.is_some() {
                        self.screen = AppScreen::Research;
                    } else {
                        self.screen = AppScreen::Input;
                    }
                }
            }
            ActiveTab::Config => {
                // Salvar tela atual antes de ir para Config
                if self.screen != AppScreen::Config {
                    self.previous_screen = Some(self.screen.clone());
                }
                self.screen = AppScreen::Config;
            }
            ActiveTab::Benchmarks => {
                // Salvar tela atual antes de ir para Benchmarks
                if self.screen != AppScreen::Benchmarks {
                    self.previous_screen = Some(self.screen.clone());
                }
                self.screen = AppScreen::Benchmarks;
            }
        }
    }

    /// Alterna entre Result e Research (quando pesquisa completa)
    pub fn toggle_result_research(&mut self) {
        match self.screen {
            AppScreen::Result => {
                self.screen = AppScreen::Research;
            }
            AppScreen::Research if self.is_complete => {
                self.screen = AppScreen::Result;
            }
            _ => {}
        }
    }

    /// Cria app com pergunta pré-definida
    pub fn with_question(question: String) -> Self {
        let mut app = Self::new();
        app.session_id = Uuid::new_v4().to_string();
        app.started_at = chrono::Local::now().to_rfc3339();
        app.question = question;
        app.screen = AppScreen::Research;
        app.active_tab = ActiveTab::Search;
        app.start_time = Some(Instant::now());
        app
    }

    /// Inicia a pesquisa com o texto atual
    pub fn start_research(&mut self) {
        if !self.input_text.is_empty() {
            // Gerar novo UUID para esta sessão
            self.session_id = Uuid::new_v4().to_string();
            self.started_at = chrono::Local::now().to_rfc3339();
            self.question = self.input_text.clone();
            self.history.push(self.input_text.clone());
            self.input_text.clear();
            self.cursor_pos = 0;
            self.screen = AppScreen::Research;
            self.start_time = Some(Instant::now());
            self.visited_urls.clear();
            self.completed_steps.clear();
            self.active_batches.clear();
            self.completed_batches.clear();
            self.all_tasks.clear();
            self.logs.push_back(LogEntry::info(format!(
                "Pesquisa iniciada (ID: {})",
                &self.session_id[..8]
            )));
        }
    }

    /// Processa um evento
    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Log(entry) => {
                self.logs.push_back(entry);
                if self.logs.len() > 100 {
                    self.logs.pop_front();
                }
                // Auto-scroll para o final
                if self.logs.len() > 10 {
                    self.log_scroll = self.logs.len().saturating_sub(10);
                }
            }
            AppEvent::SetStep(step) => {
                // Se mudou de step, salvar o anterior como completado
                if step > self.current_step && self.current_step > 0 {
                    self.completed_steps.push(CompletedStep {
                        step_num: self.current_step,
                        action: self.current_action.clone(),
                        think: self.current_think.clone(),
                        completed_at: chrono::Local::now().format("%H:%M:%S").to_string(),
                        success: true,
                    });
                }
                self.current_step = step;
            }
            AppEvent::SetAction(action) => {
                self.current_action = action;
            }
            AppEvent::SetThink(think) => {
                self.current_think = think;
            }
            AppEvent::SetUrlCount(count) => {
                self.url_count = count;
            }
            AppEvent::SetVisitedCount(count) => {
                self.visited_count = count;
            }
            AppEvent::SetTokens(tokens) => {
                self.tokens_used = tokens;
            }
            AppEvent::SetAnswer(answer) => {
                self.answer = Some(answer);
            }
            AppEvent::SetReferences(refs) => {
                self.references = refs;
            }
            AppEvent::UpdateMetrics(metrics) => {
                self.metrics = metrics;
            }
            AppEvent::UpdatePersona(stats) => {
                self.personas.insert(stats.name.clone(), stats);
            }
            AppEvent::SetTimes { total_ms, search_ms, read_ms, llm_ms } => {
                self.total_time_ms = total_ms;
                self.search_time_ms = search_ms;
                self.read_time_ms = read_ms;
                self.llm_time_ms = llm_ms;
            }
            AppEvent::Complete => {
                self.is_complete = true;
                self.screen = AppScreen::Result;
                // Congelar o tempo final
                self.final_elapsed_secs = self.start_time.map(|t| t.elapsed().as_secs_f64());
                // Salvar sessão em JSON
                self.save_session();
            }
            AppEvent::Error(msg) => {
                self.error = Some(msg.clone());
                self.logs.push_back(LogEntry::error(msg));
                // Congelar o tempo em caso de erro também
                self.final_elapsed_secs = self.start_time.map(|t| t.elapsed().as_secs_f64());
                // Salvar sessão mesmo com erro
                self.save_session();
            }
            AppEvent::AddVisitedUrl(url) => {
                if !self.visited_urls.contains(&url) {
                    self.visited_urls.push(url);
        }
    }
            AppEvent::StartBatch { batch_id, batch_type, task_count } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let batch = ParallelBatch {
                    id: batch_id.clone(),
                    batch_type: batch_type.clone(),
                    tasks: Vec::with_capacity(task_count),
                    started_at: now,
                    total_elapsed_ms: 0,
                    completed: 0,
                    failed: 0,
                };
                self.active_batches.insert(batch_id.clone(), batch);
                self.logs.push_back(LogEntry::info(format!(
                    "⚡ Batch {} iniciado: {} tarefas {}",
                    &batch_id[..8], task_count, batch_type
                )));
            }
            AppEvent::UpdateTask(task) => {
                // Atualizar no batch ativo
                if let Some(batch) = self.active_batches.get_mut(&task.batch_id) {
                    // Encontrar ou adicionar tarefa
                    if let Some(existing) = batch.tasks.iter_mut().find(|t| t.id == task.id) {
                        *existing = task.clone();
                    } else {
                        batch.tasks.push(task.clone());
                    }
                    // Atualizar contadores
                    batch.completed = batch.tasks.iter()
                        .filter(|t| matches!(t.status, TaskStatus::Completed))
                        .count();
                    batch.failed = batch.tasks.iter()
                        .filter(|t| matches!(t.status, TaskStatus::Failed(_)))
                        .count();
                }
                // Atualizar lista geral
                if let Some(existing) = self.all_tasks.iter_mut().find(|t| t.id == task.id) {
                    *existing = task;
                } else {
                    self.all_tasks.push(task);
                }
            }
            AppEvent::EndBatch { batch_id, total_ms, success_count, fail_count } => {
                if let Some(mut batch) = self.active_batches.remove(&batch_id) {
                    batch.total_elapsed_ms = total_ms;
                    batch.completed = success_count;
                    batch.failed = fail_count;
                    self.logs.push_back(LogEntry::success(format!(
                        "⚡ Batch {} completo: {}ms | ✅{} ❌{}",
                        &batch_id[..8], total_ms, success_count, fail_count
                    )));
                    self.completed_batches.push(batch);
                }
            }
            AppEvent::AgentAnalyzerStarted { failures_count, diary_entries } => {
                self.agent_analyzer.is_active = true;
                self.agent_analyzer.failures_count = failures_count;
                self.agent_analyzer.diary_entries = diary_entries;
                self.agent_analyzer.started_at = Some(chrono::Local::now().format("%H:%M:%S").to_string());
                self.agent_analyzer.logs.push(LogEntry::info(format!(
                    "Iniciando análise de {} falhas ({} entradas)",
                    failures_count, diary_entries
                )));
            }
            AppEvent::AgentAnalyzerCompleted { recap, blame, improvement, duration_ms } => {
                self.agent_analyzer.is_active = false;
                self.agent_analyzer.last_recap = Some(recap.clone());
                self.agent_analyzer.last_blame = Some(blame.clone());
                self.agent_analyzer.last_improvement = Some(improvement.clone());
                self.agent_analyzer.duration_ms = Some(duration_ms);
                self.agent_analyzer.logs.push(LogEntry::success(format!(
                    "Análise concluída em {}ms",
                    duration_ms
                )));
                self.agent_analyzer.logs.push(LogEntry::warning(format!(
                    "📊 {}",
                    recap
                )));
                self.agent_analyzer.logs.push(LogEntry::error(format!(
                    "🎯 {}",
                    blame
                )));
                self.agent_analyzer.logs.push(LogEntry::success(format!(
                    "💡 {}",
                    improvement
                )));
            }
            // Eventos de interação com usuário
            AppEvent::AgentQuestion { question_id, question_type, question, options, is_blocking } => {
                self.logs.push_back(LogEntry::info(format!(
                    "❓ [{}] {}",
                    question_type, question
                )));

                if is_blocking {
                    // Mudar para tela de input necessário
                    self.screen = AppScreen::InputRequired {
                        question_id,
                        question_type,
                        question,
                        options,
                    };
                    // Limpar input para a resposta
                    self.input_text.clear();
                    self.cursor_pos = 0;
                }
            }
            AppEvent::UserResponse { question_id, response } => {
                self.logs.push_back(LogEntry::success(format!(
                    "✅ Resposta enviada: {}",
                    response
                )));
                // Se estava aguardando input, voltar para tela de pesquisa
                if matches!(self.screen, AppScreen::InputRequired { .. }) {
                    self.screen = AppScreen::Research;
                }
                // A resposta será processada pelo agente via canal
                let _ = question_id; // Usar se necessário para rastrear
            }
            // Eventos de Sandbox
            AppEvent::SandboxStart { problem, max_attempts, timeout_ms, language } => {
                self.sandbox.start(problem, max_attempts, timeout_ms, language);
            }
            AppEvent::SandboxAttempt { attempt, max_attempts: _, code_preview, status, error } => {
                self.sandbox.update_attempt(attempt, code_preview, status, error);
            }
            AppEvent::SandboxComplete { success, output, error, attempts, execution_time_ms, code_preview, language } => {
                self.sandbox.complete(success, output, error, attempts, execution_time_ms, code_preview, language);
            }
            AppEvent::BenchmarkStarted { bench_file, bench_name } => {
                self.benchmarks.start_benchmark(&bench_file, &bench_name);
                self.benchmarks.add_execution_log(LogLevel::Info, format!("🚀 Iniciando benchmark: {}", bench_name));
            }
            AppEvent::BenchmarkLog { message, level } => {
                self.benchmarks.add_execution_log(level, message);
            }
            AppEvent::BenchmarkComplete { bench_file: _, bench_name, success, output, error, duration_secs } => {
                let result = BenchmarkResult {
                    name: bench_name.clone(),
                    started_at: chrono::Local::now().to_rfc3339(),
                    finished_at: chrono::Local::now().to_rfc3339(),
                    duration_secs,
                    output,
                    success,
                    error,
                };
                self.benchmarks.finish_benchmark(result);
                let status = if success { "✅" } else { "❌" };
                self.benchmarks.add_execution_log(
                    if success { LogLevel::Success } else { LogLevel::Error },
                    format!("{} Benchmark {} concluído em {:.2}s", status, bench_name, duration_secs)
                );
            }
            AppEvent::BenchmarkSetSchema { bench_name: _, fields } => {
                self.benchmarks.set_results_schema(fields);
            }
            AppEvent::BenchmarkUpdateField { field_id, value, status } => {
                self.benchmarks.update_result_field(&field_id, value, status);
            }
            AppEvent::BenchmarkFieldRunning { field_id } => {
                self.benchmarks.set_result_field_running(&field_id);
            }
        }
    }

    /// Tempo decorrido em segundos (congelado quando completo)
    pub fn elapsed_secs(&self) -> f64 {
        // Se já completou, retorna o tempo congelado
        if let Some(final_time) = self.final_elapsed_secs {
            return final_time;
        }
        // Caso contrário, calcula em tempo real
        self.start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Calcula progresso (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        if self.is_complete {
            1.0
        } else {
            // Estima progresso baseado no step (máximo ~10 steps típicos)
            (self.current_step as f64 / 10.0).min(0.95)
        }
    }

    /// Scroll up nos logs
    pub fn scroll_up(&mut self) {
        self.log_scroll = self.log_scroll.saturating_sub(1);
    }

    /// Scroll down nos logs
    pub fn scroll_down(&mut self) {
        let max_scroll = self.logs.len().saturating_sub(10);
        if self.log_scroll < max_scroll {
            self.log_scroll += 1;
        }
    }

    /// Scroll up na resposta final
    pub fn result_scroll_up(&mut self) {
        self.result_scroll = self.result_scroll.saturating_sub(1);
    }

    /// Scroll down na resposta final
    pub fn result_scroll_down(&mut self) {
        self.result_scroll += 1;
    }

    /// Page up na resposta final
    pub fn result_page_up(&mut self) {
        self.result_scroll = self.result_scroll.saturating_sub(10);
    }

    /// Page down na resposta final
    pub fn result_page_down(&mut self) {
        self.result_scroll += 10;
    }

    /// Seleciona item anterior no histórico visual
    pub fn history_select_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        match self.history_selected {
            Some(idx) if idx > 0 => {
                self.history_selected = Some(idx - 1);
            }
            None => {
                self.history_selected = Some(self.history.len().saturating_sub(1));
            }
            _ => {}
        }
    }

    /// Seleciona próximo item no histórico visual
    pub fn history_select_down(&mut self) {
        if self.history.is_empty() {
            return;
        }
        match self.history_selected {
            Some(idx) if idx < self.history.len() - 1 => {
                self.history_selected = Some(idx + 1);
            }
            None => {
                self.history_selected = Some(0);
            }
            _ => {}
        }
    }

    /// Usa o item selecionado do histórico
    pub fn use_selected_history(&mut self) {
        if let Some(idx) = self.history_selected {
            if let Some(question) = self.history.get(idx).cloned() {
                self.input_text = question;
                self.cursor_pos = self.input_text.chars().count();
                self.history_selected = None;
            }
        }
    }

    /// Limpa seleção do histórico
    pub fn clear_history_selection(&mut self) {
        self.history_selected = None;
    }

    // ─────────────────────────────────────────────────────────────────
    // Input handling
    // ─────────────────────────────────────────────────────────────────

    /// Retorna número de caracteres (não bytes)
    fn char_count(&self) -> usize {
        self.input_text.chars().count()
    }

    /// Insere caractere no input (suporta UTF-8)
    pub fn input_char(&mut self, c: char) {
        let mut chars: Vec<char> = self.input_text.chars().collect();
        let pos = self.cursor_pos.min(chars.len());
        chars.insert(pos, c);
        self.input_text = chars.into_iter().collect();
        self.cursor_pos += 1;
        self.history_index = None;
    }

    /// Remove caractere antes do cursor (backspace, suporta UTF-8)
    pub fn input_backspace(&mut self) {
        if self.cursor_pos > 0 {
            let mut chars: Vec<char> = self.input_text.chars().collect();
            let pos = (self.cursor_pos - 1).min(chars.len().saturating_sub(1));
            if pos < chars.len() {
                chars.remove(pos);
                self.input_text = chars.into_iter().collect();
                self.cursor_pos -= 1;
            }
        }
    }

    /// Remove caractere no cursor (delete, suporta UTF-8)
    pub fn input_delete(&mut self) {
        let char_count = self.char_count();
        if self.cursor_pos < char_count {
            let mut chars: Vec<char> = self.input_text.chars().collect();
            chars.remove(self.cursor_pos);
            self.input_text = chars.into_iter().collect();
        }
    }

    /// Move cursor para esquerda
    pub fn cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    /// Move cursor para direita
    pub fn cursor_right(&mut self) {
        let char_count = self.char_count();
        if self.cursor_pos < char_count {
            self.cursor_pos += 1;
        }
    }

    /// Move cursor para início
    pub fn cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move cursor para fim
    pub fn cursor_end(&mut self) {
        self.cursor_pos = self.char_count();
    }

    /// Alterna o foco do input durante a pesquisa
    pub fn toggle_input_focus(&mut self) {
        self.input_focused = !self.input_focused;
    }

    /// Foca o campo de input
    pub fn focus_input(&mut self) {
        self.input_focused = true;
    }

    /// Desfoca o campo de input
    pub fn unfocus_input(&mut self) {
        self.input_focused = false;
    }

    /// Enfileira uma mensagem do usuário para enviar ao agente
    pub fn queue_user_message(&mut self, message: String) {
        if !message.trim().is_empty() {
            self.user_message_queue.push_back(message.clone());
            self.pending_user_messages = self.user_message_queue.len();
            self.logs.push_back(LogEntry::new(
                LogLevel::Info,
                format!("📤 Mensagem enfileirada: {:.40}...", message),
            ));
        }
    }

    /// Retira a próxima mensagem da fila
    pub fn dequeue_user_message(&mut self) -> Option<String> {
        let msg = self.user_message_queue.pop_front();
        self.pending_user_messages = self.user_message_queue.len();
        msg
    }

    /// Verifica se há mensagens pendentes
    pub fn has_pending_messages(&self) -> bool {
        !self.user_message_queue.is_empty()
    }

    /// Navega para trás no histórico
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let new_index = match self.history_index {
            None => self.history.len() - 1,
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
        };
        self.history_index = Some(new_index);
        self.input_text = self.history[new_index].clone();
        self.cursor_pos = self.char_count();
    }

    /// Navega para frente no histórico
    pub fn history_down(&mut self) {
        if let Some(i) = self.history_index {
            if i + 1 < self.history.len() {
                self.history_index = Some(i + 1);
                self.input_text = self.history[i + 1].clone();
            } else {
                self.history_index = None;
                self.input_text.clear();
            }
            self.cursor_pos = self.char_count();
        }
    }

    /// Limpa o input
    pub fn clear_input(&mut self) {
        self.input_text.clear();
        self.cursor_pos = 0;
        self.history_index = None;
    }

    /// Reseta para nova pesquisa
    pub fn reset(&mut self) {
        // Gerar novo UUID para próxima sessão
        self.session_id = Uuid::new_v4().to_string();
        self.started_at = chrono::Local::now().to_rfc3339();
        self.screen = AppScreen::Input;
        self.active_tab = ActiveTab::Search;
        self.previous_screen = None;
        self.question.clear();
        self.current_step = 0;
        self.current_action = "Aguardando...".into();
        self.current_think.clear();
        self.logs.clear();
        self.url_count = 0;
        self.visited_count = 0;
        self.visited_urls.clear();
        self.tokens_used = 0;
        self.answer = None;
        self.references.clear();
        self.is_complete = false;
        self.error = None;
        self.start_time = None;
        self.final_elapsed_secs = None;
        self.total_time_ms = 0;
        self.search_time_ms = 0;
        self.read_time_ms = 0;
        self.llm_time_ms = 0;
        self.log_scroll = 0;
        self.result_scroll = 0;
        self.history_selected = None;
        self.personas.clear();
        self.active_batches.clear();
        self.completed_batches.clear();
        self.all_tasks.clear();
        self.completed_steps.clear();
        self.agent_analyzer = AgentAnalyzerState::default();
        self.clipboard_message = None;
    }

    // ─────────────────────────────────────────────────────────────────
    // Persistência de sessões
    // ─────────────────────────────────────────────────────────────────

    /// Retorna o diretório de sessões (no projeto)
    fn sessions_dir() -> PathBuf {
        // Usar CARGO_MANIFEST_DIR em tempo de compilação ou diretório atual
        let base = option_env!("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        base.join("sessions")
    }

    /// Retorna o diretório de logs (no projeto)
    fn logs_dir() -> PathBuf {
        let base = option_env!("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        base.join("logs")
    }

    /// Converte o estado atual para ResearchSession
    pub fn to_session(&self) -> ResearchSession {
        ResearchSession {
            id: self.session_id.clone(),
            started_at: self.started_at.clone(),
            finished_at: Some(chrono::Local::now().to_rfc3339()),
            question: self.question.clone(),
            answer: self.answer.clone(),
            references: self.references.clone(),
            visited_urls: self.visited_urls.clone(),
            logs: self.logs.iter().cloned().collect(),
            personas: self.personas.clone(),
            timing: SessionTiming {
                total_ms: self.total_time_ms,
                search_ms: self.search_time_ms,
                read_ms: self.read_time_ms,
                llm_ms: self.llm_time_ms,
            },
            stats: SessionStats {
                steps: self.current_step,
                urls_found: self.url_count,
                urls_visited: self.visited_count,
                tokens_used: self.tokens_used,
            },
            success: self.error.is_none() && self.answer.is_some(),
            error: self.error.clone(),
            parallel_batches: self.completed_batches.clone(),
            all_tasks: self.all_tasks.clone(),
            completed_steps: self.completed_steps.clone(),
            sandbox_executions: self.sandbox.executions.clone(),
        }
    }

    /// Salva a sessão atual em arquivo JSON e logs em TXT
    pub fn save_session(&self) {
        let session = self.to_session();
        let sessions_dir = Self::sessions_dir();
        let logs_dir = Self::logs_dir();

        // Criar diretórios se não existirem
        if let Err(e) = std::fs::create_dir_all(&sessions_dir) {
            log::warn!("Falha ao criar diretório de sessões: {}", e);
            return;
        }
        if let Err(e) = std::fs::create_dir_all(&logs_dir) {
            log::warn!("Falha ao criar diretório de logs: {}", e);
        }

        // Nome base: timestamp_uuid
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let base_name = format!("{}_{}", timestamp, &self.session_id[..8]);

        // Salvar JSON
        let json_path = sessions_dir.join(format!("{}.json", base_name));
        match serde_json::to_string_pretty(&session) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&json_path, &json) {
                    log::warn!("Falha ao salvar sessão JSON: {}", e);
                } else {
                    log::info!("💾 Sessão JSON: {}", json_path.display());
                }
            }
            Err(e) => {
                log::warn!("Falha ao serializar sessão: {}", e);
            }
        }

        // Salvar logs em TXT
        let logs_path = logs_dir.join(format!("{}.txt", base_name));
        let logs_content = self.format_logs_for_txt();
        if let Err(e) = std::fs::write(&logs_path, &logs_content) {
            log::warn!("Falha ao salvar logs TXT: {}", e);
        } else {
            log::info!("📄 Logs TXT: {}", logs_path.display());
        }
    }

    /// Formata logs para arquivo TXT
    fn format_logs_for_txt(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str("═══════════════════════════════════════════════════════════════════\n");
        output.push_str(&format!(" DEEP RESEARCH - Session {}\n", &self.session_id[..8]));
        output.push_str("═══════════════════════════════════════════════════════════════════\n\n");

        output.push_str(&format!("📅 Início: {}\n", self.started_at));
        output.push_str(&format!("❓ Pergunta: {}\n", self.question));
        output.push_str(&format!("📊 Steps: {} | URLs: {} | Tokens: {}\n",
            self.current_step, self.visited_count, self.tokens_used));
        output.push_str(&format!("⏱️  Tempo: {:.1}s total | {:.1}s busca | {:.1}s leitura | {:.1}s LLM\n\n",
            self.total_time_ms as f64 / 1000.0,
            self.search_time_ms as f64 / 1000.0,
            self.read_time_ms as f64 / 1000.0,
            self.llm_time_ms as f64 / 1000.0));

        // Logs
        output.push_str("───────────────────────────────────────────────────────────────────\n");
        output.push_str(" LOGS\n");
        output.push_str("───────────────────────────────────────────────────────────────────\n\n");

        for entry in &self.logs {
            let level_str = match entry.level {
                LogLevel::Info => "INFO",
                LogLevel::Success => "OK  ",
                LogLevel::Warning => "WARN",
                LogLevel::Error => "ERR ",
                LogLevel::Debug => "DBG ",
            };
            output.push_str(&format!("[{}] {} {}\n", entry.timestamp, level_str, entry.message));
        }

        // URLs visitadas
        if !self.visited_urls.is_empty() {
            output.push_str("\n───────────────────────────────────────────────────────────────────\n");
            output.push_str(" URLs VISITADAS\n");
            output.push_str("───────────────────────────────────────────────────────────────────\n\n");
            for (i, url) in self.visited_urls.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, url));
            }
        }

        // Referências
        if !self.references.is_empty() {
            output.push_str("\n───────────────────────────────────────────────────────────────────\n");
            output.push_str(" REFERÊNCIAS\n");
            output.push_str("───────────────────────────────────────────────────────────────────\n\n");
            for (i, reference) in self.references.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, reference));
            }
        }

        // Resposta
        if let Some(answer) = &self.answer {
            output.push_str("\n═══════════════════════════════════════════════════════════════════\n");
            output.push_str(" RESPOSTA FINAL\n");
            output.push_str("═══════════════════════════════════════════════════════════════════\n\n");
            output.push_str(answer);
            output.push_str("\n");
        }

        // Steps Completados
        if !self.completed_steps.is_empty() {
            output.push_str("\n───────────────────────────────────────────────────────────────────\n");
            output.push_str(" STEPS EXECUTADOS\n");
            output.push_str("───────────────────────────────────────────────────────────────────\n\n");
            for step in &self.completed_steps {
                let status = if step.success { "✅" } else { "❌" };
                output.push_str(&format!("{} Step #{} [{}] - {}\n",
                    status, step.step_num, step.completed_at, step.action));
                if !step.think.is_empty() {
                    let think_short = if step.think.len() > 100 {
                        format!("{}...", &step.think[..100])
                    } else {
                        step.think.clone()
                    };
                    output.push_str(&format!("   └─ Raciocínio: {}\n", think_short));
                }
            }
        }

        // Personas
        if !self.personas.is_empty() {
            output.push_str("\n───────────────────────────────────────────────────────────────────\n");
            output.push_str(" PERSONAS UTILIZADAS\n");
            output.push_str("───────────────────────────────────────────────────────────────────\n\n");
            for (name, stats) in &self.personas {
                output.push_str(&format!("• {} - Buscas: {} | Leituras: {} | Tokens: {}\n",
                    name, stats.searches, stats.reads, stats.tokens));
            }
        }

        // Batches Paralelos
        if !self.completed_batches.is_empty() {
            output.push_str("\n───────────────────────────────────────────────────────────────────\n");
            output.push_str(" EXECUÇÕES PARALELAS\n");
            output.push_str("───────────────────────────────────────────────────────────────────\n\n");
            for batch in &self.completed_batches {
                output.push_str(&format!("📦 Batch {} [{}]\n", &batch.id[..8], batch.batch_type));
                output.push_str(&format!("   Tempo total: {}ms | Tarefas: {} | ✅{} ❌{}\n",
                    batch.total_elapsed_ms, batch.tasks.len(), batch.completed, batch.failed));
                for task in &batch.tasks {
                    let status_str = match &task.status {
                        TaskStatus::Pending => "⏳",
                        TaskStatus::Running => "🔄",
                        TaskStatus::Completed => "✅",
                        TaskStatus::Failed(_) => "❌",
                    };
                    output.push_str(&format!("   {} {} | {}ms | {}\n",
                        status_str, task.task_type, task.elapsed_ms, task.description));
                    if !task.data_info.is_empty() {
                        output.push_str(&format!("      └─ Dados: {}\n", task.data_info));
                    }
                }
                output.push_str("\n");
            }
        }

        // Todas as Tarefas (detalhado)
        if !self.all_tasks.is_empty() {
            output.push_str("\n───────────────────────────────────────────────────────────────────\n");
            output.push_str(" TODAS AS TAREFAS (DETALHADO)\n");
            output.push_str("───────────────────────────────────────────────────────────────────\n\n");
            for task in &self.all_tasks {
                let status_str = match &task.status {
                    TaskStatus::Pending => "PEND",
                    TaskStatus::Running => "RUN ",
                    TaskStatus::Completed => "OK  ",
                    TaskStatus::Failed(e) => &format!("FAIL: {}", e),
                };
                output.push_str(&format!("[{}] {} | {}ms | Batch: {} | Thread: {}\n",
                    status_str,
                    task.task_type,
                    task.elapsed_ms,
                    &task.batch_id[..8],
                    task.thread_id.as_deref().unwrap_or("N/A")
                ));
                output.push_str(&format!("    URL: {}\n", task.description));
                if !task.data_info.is_empty() {
                    output.push_str(&format!("    Dados: {}\n", task.data_info));
                }
            }
        }

        // Execuções de Sandbox
        if !self.sandbox.executions.is_empty() {
            output.push_str("\n───────────────────────────────────────────────────────────────────\n");
            output.push_str(" EXECUÇÕES DE CÓDIGO (SANDBOX)\n");
            output.push_str("───────────────────────────────────────────────────────────────────\n\n");
            for (i, exec) in self.sandbox.executions.iter().enumerate() {
                let status = if exec.success { "✅" } else { "❌" };
                let lang_emoji = if exec.language == "Python" { "🐍" } else { "📜" };
                output.push_str(&format!("{} {} Execução #{} [{}] - {}\n",
                    status, lang_emoji, i + 1, exec.completed_at, exec.language));
                output.push_str(&format!("   Problema: {}\n",
                    if exec.problem.len() > 80 { format!("{}...", &exec.problem[..77]) } else { exec.problem.clone() }));
                output.push_str(&format!("   Tentativas: {} | Tempo: {}ms\n",
                    exec.attempts, exec.execution_time_ms));
                if let Some(out) = &exec.output {
                    let out_preview = if out.len() > 100 { format!("{}...", &out[..97]) } else { out.clone() };
                    output.push_str(&format!("   Output: {}\n", out_preview));
                }
                if let Some(err) = &exec.error {
                    output.push_str(&format!("   Erro: {}\n", err));
                }
                if !exec.code_preview.is_empty() {
                    output.push_str("   Código:\n");
                    for line in exec.code_preview.lines().take(5) {
                        output.push_str(&format!("      {}\n", line));
                    }
                    if exec.code_preview.lines().count() > 5 {
                        output.push_str("      ...\n");
                    }
                }
                output.push_str("\n");
            }
        }

        output.push_str("\n═══════════════════════════════════════════════════════════════════\n");
        output.push_str(&format!(" FIM - Session {}\n", self.session_id));
        output.push_str("═══════════════════════════════════════════════════════════════════\n");

        output
    }

    /// Carrega sessões anteriores do diretório
    pub fn load_sessions(&mut self) {
        let dir = Self::sessions_dir();
        if !dir.exists() {
            return;
        }

        let mut sessions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(session) = serde_json::from_str::<ResearchSession>(&content) {
                            sessions.push(session);
                        }
                    }
                }
            }
        }

        // Ordenar por data (mais recente primeiro)
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        // Manter apenas as últimas 50 sessões
        sessions.truncate(50);

        self.saved_sessions = sessions;

        // Popular histórico com perguntas das sessões
        for session in &self.saved_sessions {
            if !self.history.contains(&session.question) {
                self.history.push(session.question.clone());
            }
        }
    }

    /// Retorna o caminho do arquivo JSON da sessão atual
    pub fn current_session_path(&self) -> Option<PathBuf> {
        let dir = Self::sessions_dir();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.to_string_lossy().contains(&self.session_id[..8]) {
                    return Some(path);
                }
            }
        }
        None
    }
}
