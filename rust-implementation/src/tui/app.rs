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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    /// Tela de input da pergunta
    Input,
    /// Tela de pesquisa em andamento
    Research,
    /// Tela de resultado
    Result,
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
    }

    // ─────────────────────────────────────────────────────────────────
    // Persistência de sessões
    // ─────────────────────────────────────────────────────────────────

    /// Retorna o diretório de sessões
    fn sessions_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".deep-research").join("sessions")
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
        }
    }

    /// Salva a sessão atual em arquivo JSON
    pub fn save_session(&self) {
        let session = self.to_session();
        let dir = Self::sessions_dir();

        // Criar diretório se não existir
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!("Falha ao criar diretório de sessões: {}", e);
            return;
        }

        // Nome do arquivo: timestamp_uuid.json
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.json", timestamp, &self.session_id[..8]);
        let filepath = dir.join(&filename);

        // Serializar e salvar
        match serde_json::to_string_pretty(&session) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&filepath, json) {
                    log::warn!("Falha ao salvar sessão: {}", e);
                } else {
                    log::info!("💾 Sessão salva: {}", filepath.display());
                }
            }
            Err(e) => {
                log::warn!("Falha ao serializar sessão: {}", e);
            }
        }
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
