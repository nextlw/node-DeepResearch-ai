//! Runner que conecta o agente com a TUI

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::app::{App, AppEvent, LogEntry};
use super::ui;

/// Copia texto para o clipboard do sistema
fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().map_err(|e| format!("Falha ao acessar clipboard: {}", e))?;
    clipboard.set_text(text).map_err(|e| format!("Falha ao copiar: {}", e))?;
    Ok(())
}

/// Executa a TUI com um receptor de eventos
pub fn run_tui(question: String, event_rx: Receiver<AppEvent>) -> io::Result<App> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Criar app
    let mut app = App::with_question(question);

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
    use super::app::AppScreen;

    loop {
        // Renderizar
        terminal.draw(|frame| ui::render(frame, app))?;

        // Processar eventos do agente (não bloqueante)
        while let Ok(event) = event_rx.try_recv() {
            app.handle_event(event);
        }

        // Processar input do usuário (com timeout)
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                // Eventos de teclado
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match app.screen {
                        // Tela de pesquisa - scroll nos logs
                        AppScreen::Research => match key.code {
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
                            KeyCode::PageUp => {
                                for _ in 0..5 {
                                    app.scroll_up();
                                }
                            }
                            KeyCode::PageDown => {
                                for _ in 0..5 {
                                    app.scroll_down();
                                }
                            }
                            KeyCode::Esc => {
                                if app.is_complete {
                                    return Ok(());
                                }
                            }
                            _ => {}
                        },

                        // Tela de resultado - scroll na resposta + copiar
                        AppScreen::Result => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                                return Ok(());
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.result_scroll_up();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.result_scroll_down();
                            }
                            KeyCode::PageUp => {
                                app.result_page_up();
                            }
                            KeyCode::PageDown => {
                                app.result_page_down();
                            }
                            KeyCode::Home => {
                                app.result_scroll = 0;
                            }
                            KeyCode::End => {
                                app.result_scroll = usize::MAX; // será limitado pelo render
                            }
                            KeyCode::Char('c') => {
                                // Copiar resposta para clipboard
                                if let Some(answer) = &app.answer {
                                    if copy_to_clipboard(answer).is_ok() {
                                        app.clipboard_message = Some("✓ Copiado!".to_string());
                                    } else {
                                        app.clipboard_message = Some("✗ Erro ao copiar".to_string());
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // Nova pesquisa
                                app.reset();
                            }
                            _ => {}
                        },

                        // Tela de input - já tratada no main.rs
                        AppScreen::Input => match key.code {
                            KeyCode::Char('q') if app.input_text.is_empty() => {
                                app.should_quit = true;
                                return Ok(());
                            }
                            KeyCode::Esc => {
                                if app.history_selected.is_some() {
                                    app.clear_history_selection();
                                } else {
                                    app.should_quit = true;
                                    return Ok(());
                                }
                            }
                            _ => {}
                        },
                    }
                }

                // Eventos de mouse - scroll
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            match app.screen {
                                AppScreen::Research => app.scroll_up(),
                                AppScreen::Result => app.result_scroll_up(),
                                _ => {}
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            match app.screen {
                                AppScreen::Research => app.scroll_down(),
                                AppScreen::Result => app.result_scroll_down(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }

                _ => {}
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

/// Wrapper para enviar logs e eventos formatados para a TUI.
///
/// Encapsula o canal de comunicação com a interface gráfica,
/// fornecendo métodos convenientes para diferentes tipos de log.
pub struct TuiLogger {
    tx: Sender<AppEvent>,
}

impl TuiLogger {
    /// Cria um novo logger para a TUI.
    ///
    /// # Argumentos
    ///
    /// * `tx` - Canal de envio para eventos da aplicação
    pub fn new(tx: Sender<AppEvent>) -> Self {
        Self { tx }
    }

    /// Envia uma mensagem informativa (azul).
    pub fn info(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::info(msg)));
    }

    /// Envia uma mensagem de sucesso (verde).
    pub fn success(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::success(msg)));
    }

    /// Envia uma mensagem de aviso (amarelo).
    pub fn warning(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::warning(msg)));
    }

    /// Envia uma mensagem de erro (vermelho).
    pub fn error(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::error(msg)));
    }

    /// Define o passo atual da pesquisa.
    pub fn set_step(&self, step: usize) {
        let _ = self.tx.send(AppEvent::SetStep(step));
    }

    /// Define a ação atual sendo executada.
    pub fn set_action(&self, action: impl Into<String>) {
        let _ = self.tx.send(AppEvent::SetAction(action.into()));
    }

    /// Define o pensamento/raciocínio atual do agente.
    pub fn set_think(&self, think: impl Into<String>) {
        let _ = self.tx.send(AppEvent::SetThink(think.into()));
    }

    /// Atualiza contadores de URLs (total e visitadas).
    pub fn set_urls(&self, total: usize, visited: usize) {
        let _ = self.tx.send(AppEvent::SetUrlCount(total));
        let _ = self.tx.send(AppEvent::SetVisitedCount(visited));
    }

    /// Atualiza o contador de tokens consumidos.
    pub fn set_tokens(&self, tokens: u64) {
        let _ = self.tx.send(AppEvent::SetTokens(tokens));
    }

    /// Marca a pesquisa como completa com resposta e referências.
    pub fn complete(&self, answer: String, references: Vec<String>) {
        let _ = self.tx.send(AppEvent::SetAnswer(answer));
        let _ = self.tx.send(AppEvent::SetReferences(references));
        let _ = self.tx.send(AppEvent::Complete);
    }
}
