//! Runner que conecta o agente com a TUI

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::app::{App, AppEvent, LogEntry};
use super::ui;

/// Executa a TUI com um receptor de eventos
pub fn run_tui(question: String, event_rx: Receiver<AppEvent>) -> io::Result<App> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Criar app
    let mut app = App::new(question);

    // Loop principal
    let result = run_app(&mut terminal, &mut app, event_rx);

    // Restaurar terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result?;
    Ok(app)
}

/// Loop principal da TUI
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    event_rx: Receiver<AppEvent>,
) -> io::Result<()> {
    loop {
        // Renderizar
        terminal.draw(|frame| ui::render(frame, app))?;

        // Processar eventos do agente (não bloqueante)
        while let Ok(event) = event_rx.try_recv() {
            app.handle_event(event);
        }

        // Processar input do usuário (com timeout)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                            return Ok(());
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.scroll_up();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.scroll_down();
                        }
                        KeyCode::Esc => {
                            if app.is_complete {
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Sair se completo e usuário pressionar qualquer tecla
        if app.is_complete && app.should_quit {
            return Ok(());
        }
    }
}

/// Cria um canal para enviar eventos para a TUI
pub fn create_event_channel() -> (Sender<AppEvent>, Receiver<AppEvent>) {
    mpsc::channel()
}

/// Wrapper para enviar logs formatados
pub struct TuiLogger {
    tx: Sender<AppEvent>,
}

impl TuiLogger {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        Self { tx }
    }

    pub fn info(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::info(msg)));
    }

    pub fn success(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::success(msg)));
    }

    pub fn warning(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::warning(msg)));
    }

    pub fn error(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::error(msg)));
    }

    pub fn set_step(&self, step: usize) {
        let _ = self.tx.send(AppEvent::SetStep(step));
    }

    pub fn set_action(&self, action: impl Into<String>) {
        let _ = self.tx.send(AppEvent::SetAction(action.into()));
    }

    pub fn set_think(&self, think: impl Into<String>) {
        let _ = self.tx.send(AppEvent::SetThink(think.into()));
    }

    pub fn set_urls(&self, total: usize, visited: usize) {
        let _ = self.tx.send(AppEvent::SetUrlCount(total));
        let _ = self.tx.send(AppEvent::SetVisitedCount(visited));
    }

    pub fn set_tokens(&self, tokens: u64) {
        let _ = self.tx.send(AppEvent::SetTokens(tokens));
    }

    pub fn complete(&self, answer: String, references: Vec<String>) {
        let _ = self.tx.send(AppEvent::SetAnswer(answer));
        let _ = self.tx.send(AppEvent::SetReferences(references));
        let _ = self.tx.send(AppEvent::Complete);
    }
}
