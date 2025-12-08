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

/// Estado da tela
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppScreen {
    /// Tela de input da pergunta
    Input,
    /// Tela de pesquisa em andamento
    Research,
    /// Tela de resultado
    Result,
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
}

/// Estado da aplicação
pub struct App {
    /// UUID único da sessão atual
    pub session_id: String,
    /// Timestamp de início (ISO 8601)
    pub started_at: String,
    /// Tela atual
    pub screen: AppScreen,
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
    /// Mensagem temporária do clipboard (feedback ao usuário)
    pub clipboard_message: Option<String>,
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
            clipboard_message: None,
        };
        // Carregar sessões anteriores
        app.load_sessions();
        app
    }

    /// Cria app com pergunta pré-definida
    pub fn with_question(question: String) -> Self {
        let mut app = Self::new();
        app.session_id = Uuid::new_v4().to_string();
        app.started_at = chrono::Local::now().to_rfc3339();
        app.question = question;
        app.screen = AppScreen::Research;
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
