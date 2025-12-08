//! Estado da aplicação TUI

use std::collections::VecDeque;
use std::time::Instant;

/// Nível de severidade do log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
    Debug,
}

impl LogLevel {
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
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let now = chrono::Local::now();
        Self {
            timestamp: now.format("%H:%M:%S").to_string(),
            level,
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Success, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }
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
    /// Pesquisa concluída
    Complete,
    /// Erro fatal
    Error(String),
}

/// Estado da aplicação
pub struct App {
    /// Pergunta sendo pesquisada
    pub question: String,
    /// Step atual
    pub current_step: usize,
    /// Ação atual sendo executada
    pub current_action: String,
    /// Raciocínio atual do agente
    pub current_think: String,
    /// Logs da sessão (últimos 100)
    pub logs: VecDeque<LogEntry>,
    /// URLs encontradas
    pub url_count: usize,
    /// URLs visitadas
    pub visited_count: usize,
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
    pub start_time: Instant,
    /// Scroll position dos logs
    pub log_scroll: usize,
    /// Se deve sair
    pub should_quit: bool,
}

impl App {
    /// Cria nova instância da aplicação
    pub fn new(question: String) -> Self {
        Self {
            question,
            current_step: 0,
            current_action: "Iniciando...".into(),
            current_think: String::new(),
            logs: VecDeque::with_capacity(100),
            url_count: 0,
            visited_count: 0,
            tokens_used: 0,
            answer: None,
            references: Vec::new(),
            is_complete: false,
            error: None,
            start_time: Instant::now(),
            log_scroll: 0,
            should_quit: false,
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
            AppEvent::Complete => {
                self.is_complete = true;
            }
            AppEvent::Error(msg) => {
                self.error = Some(msg.clone());
                self.logs.push_back(LogEntry::error(msg));
            }
        }
    }

    /// Tempo decorrido em segundos
    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
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
}
