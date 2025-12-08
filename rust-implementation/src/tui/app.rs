//! Estado da aplicação TUI

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Nível de severidade do log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Default)]
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
    pub is_active: bool,
}

/// Métricas do sistema
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// Threads ativas
    pub threads: usize,
    /// Uso de memória em MB
    pub memory_mb: f64,
    /// CPU %
    pub cpu_percent: f32,
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
    /// Pesquisa concluída
    Complete,
    /// Erro fatal
    Error(String),
}

/// Estado da aplicação
pub struct App {
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
    pub start_time: Option<Instant>,
    /// Tempo final (congelado quando completa)
    pub final_elapsed_secs: Option<f64>,
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
    /// Índice no histórico
    pub history_index: Option<usize>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Cria nova instância da aplicação
    pub fn new() -> Self {
        Self {
            screen: AppScreen::Input,
            input_text: String::new(),
            cursor_pos: 0,
            question: String::new(),
            current_step: 0,
            current_action: "Aguardando...".into(),
            current_think: String::new(),
            logs: VecDeque::with_capacity(100),
            url_count: 0,
            visited_count: 0,
            tokens_used: 0,
            answer: None,
            references: Vec::new(),
            is_complete: false,
            error: None,
            start_time: None,
            final_elapsed_secs: None,
            log_scroll: 0,
            should_quit: false,
            metrics: SystemMetrics::default(),
            personas: HashMap::new(),
            history: Vec::new(),
            history_index: None,
        }
    }

    /// Cria app com pergunta pré-definida
    pub fn with_question(question: String) -> Self {
        let mut app = Self::new();
        app.question = question;
        app.screen = AppScreen::Research;
        app.start_time = Some(Instant::now());
        app
    }

    /// Inicia a pesquisa com o texto atual
    pub fn start_research(&mut self) {
        if !self.input_text.is_empty() {
            self.question = self.input_text.clone();
            self.history.push(self.input_text.clone());
            self.input_text.clear();
            self.cursor_pos = 0;
            self.screen = AppScreen::Research;
            self.start_time = Some(Instant::now());
            self.logs.push_back(LogEntry::info("Pesquisa iniciada..."));
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
            AppEvent::Complete => {
                self.is_complete = true;
                self.screen = AppScreen::Result;
                // Congelar o tempo final
                self.final_elapsed_secs = self.start_time.map(|t| t.elapsed().as_secs_f64());
            }
            AppEvent::Error(msg) => {
                self.error = Some(msg.clone());
                self.logs.push_back(LogEntry::error(msg));
                // Congelar o tempo em caso de erro também
                self.final_elapsed_secs = self.start_time.map(|t| t.elapsed().as_secs_f64());
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
        self.screen = AppScreen::Input;
        self.question.clear();
        self.current_step = 0;
        self.current_action = "Aguardando...".into();
        self.current_think.clear();
        self.logs.clear();
        self.url_count = 0;
        self.visited_count = 0;
        self.tokens_used = 0;
        self.answer = None;
        self.references.clear();
        self.is_complete = false;
        self.error = None;
        self.start_time = None;
        self.final_elapsed_secs = None;
        self.log_scroll = 0;
        self.personas.clear();
    }
}
