//! Runner que conecta o agente com a TUI

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use tokio::process::Command as TokioCommand;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::app::{App, AppEvent, LogEntry};
use super::ui;

/// Copia texto para o clipboard do sistema
#[cfg(feature = "clipboard")]
fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().map_err(|e| format!("Falha ao acessar clipboard: {}", e))?;
    clipboard.set_text(text).map_err(|e| format!("Falha ao copiar: {}", e))?;
    Ok(())
}

#[cfg(not(feature = "clipboard"))]
fn copy_to_clipboard(_text: &str) -> Result<(), String> {
    Err("Clipboard não disponível (compile com --features clipboard)".to_string())
}

/// Executa a TUI com um receptor de eventos
pub fn run_tui(question: String, event_rx: Receiver<AppEvent>, event_tx: Sender<AppEvent>) -> io::Result<App> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Criar app
    let mut app = App::with_question(question);

    // Loop principal
    let result = run_app(&mut terminal, &mut app, event_rx, event_tx);

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
    event_tx: Sender<AppEvent>,
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

                        // Tela de input requerido pelo agente
                        AppScreen::InputRequired { .. } => match key.code {
                            KeyCode::Enter => {
                                // Enviar resposta
                                if !app.input_text.is_empty() {
                                    // Extrair question_id se disponível
                                    let question_id = if let AppScreen::InputRequired { question_id, .. } = &app.screen {
                                        Some(question_id.clone())
                                    } else {
                                        None
                                    };

                                    let response = app.input_text.clone();
                                    app.handle_event(AppEvent::UserResponse {
                                        question_id,
                                        response,
                                    });
                                    app.input_text.clear();
                                    app.cursor_pos = 0;
                                }
                            }
                            KeyCode::Char(c) => {
                                app.input_char(c);
                            }
                            KeyCode::Backspace => {
                                app.input_backspace();
                            }
                            KeyCode::Left => {
                                app.cursor_left();
                            }
                            KeyCode::Right => {
                                app.cursor_right();
                            }
                            KeyCode::Home => {
                                app.cursor_home();
                            }
                            KeyCode::End => {
                                app.cursor_end();
                            }
                            KeyCode::Esc => {
                                // Cancelar - voltar para pesquisa
                                app.screen = AppScreen::Research;
                            }
                            _ => {}
                        },

                        // Tela de configurações
                        AppScreen::Config => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                                return Ok(());
                            }
                            KeyCode::Backspace | KeyCode::Tab => {
                                // Voltar para pesquisa
                                app.go_to_tab(super::app::ActiveTab::Search);
                            }
                            KeyCode::Char('1') => {
                                app.go_to_tab(super::app::ActiveTab::Search);
                            }
                            KeyCode::Char('2') => {
                                // Já está em Config
                            }
                            KeyCode::Char('3') => {
                                app.go_to_tab(super::app::ActiveTab::Benchmarks);
                            }
                            _ => {}
                        },
                        // Tela de benchmarks
                        AppScreen::Benchmarks => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                                return Ok(());
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.benchmarks.select_prev();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.benchmarks.select_next();
                            }
                            KeyCode::Enter => {
                                // Executar benchmark selecionado
                                if let Some(bench) = app.benchmarks.get_selected() {
                                    if app.benchmarks.running.is_none() {
                                        // Spawn task assíncrona para executar benchmark
                                        let bench_file = bench.bench_file.clone();
                                        let bench_name = bench.name.clone();
                                        let tx = event_tx.clone();

                                        // Enviar evento de início
                                        let _ = tx.send(AppEvent::BenchmarkStarted {
                                            bench_file: bench_file.clone(),
                                            bench_name: bench_name.clone(),
                                        });

                                        tokio::spawn(async move {
                                            execute_benchmark(bench_file, bench_name, tx).await;
                                        });
                                    }
                                }
                            }
                            KeyCode::PageUp => {
                                for _ in 0..5 {
                                    app.benchmarks.scroll_up();
                                }
                            }
                            KeyCode::PageDown => {
                                for _ in 0..5 {
                                    app.benchmarks.scroll_down();
                                }
                            }
                            KeyCode::Backspace | KeyCode::Tab => {
                                // Voltar para pesquisa
                                app.go_to_tab(super::app::ActiveTab::Search);
                            }
                            KeyCode::Char('1') => {
                                app.go_to_tab(super::app::ActiveTab::Search);
                            }
                            KeyCode::Char('2') => {
                                app.go_to_tab(super::app::ActiveTab::Config);
                            }
                            KeyCode::Char('3') => {
                                // Já está em Benchmarks
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

/// Executa um benchmark em background e envia eventos para a TUI
pub async fn execute_benchmark(bench_file: String, bench_name: String, tx: Sender<AppEvent>) {
    let start_time = Instant::now();

    // Encontrar diretório do projeto (benchmarks estão em rust-implementation/benches/)
    // O comando cargo bench precisa ser executado a partir do diretório rust-implementation
    let current_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."));

    // Detectar o diretório correto onde está o Cargo.toml e a pasta benches
    // Estratégia: procurar por Cargo.toml e benches/ no diretório atual ou em rust-implementation/
    let bench_dir = if current_dir.join("Cargo.toml").exists() && current_dir.join("benches").exists() {
        // Já estamos no diretório rust-implementation
        current_dir.clone()
    } else if current_dir.join("rust-implementation").join("Cargo.toml").exists()
        && current_dir.join("rust-implementation").join("benches").exists() {
        // Estamos no diretório raiz, ir para rust-implementation
        current_dir.join("rust-implementation")
    } else {
        // Fallback: tentar diretório atual mesmo
        current_dir.clone()
    };

    // Verificar se o diretório existe
    if !bench_dir.exists() {
        let _ = tx.send(AppEvent::BenchmarkLog {
            message: format!("❌ Diretório não encontrado: {}", bench_dir.display()),
            level: super::app::LogLevel::Error,
        });
        let _ = tx.send(AppEvent::BenchmarkComplete {
            bench_file: bench_file.clone(),
            bench_name: bench_name.clone(),
            success: false,
            output: String::new(),
            error: Some(format!("Diretório não encontrado: {}", bench_dir.display())),
            duration_secs: start_time.elapsed().as_secs_f64(),
        });
        return;
    }

    // Verificar se o arquivo de benchmark existe
    let bench_path = bench_dir.join("benches").join(format!("{}.rs", bench_file));
    if !bench_path.exists() {
        let _ = tx.send(AppEvent::BenchmarkLog {
            message: format!("❌ Arquivo de benchmark não encontrado: {}", bench_path.display()),
            level: super::app::LogLevel::Error,
        });
        let _ = tx.send(AppEvent::BenchmarkComplete {
            bench_file: bench_file.clone(),
            bench_name: bench_name.clone(),
            success: false,
            output: String::new(),
            error: Some(format!("Arquivo não encontrado: {}", bench_path.display())),
            duration_secs: start_time.elapsed().as_secs_f64(),
        });
        return;
    }

    let _ = tx.send(AppEvent::BenchmarkLog {
        message: format!("📦 Executando: cargo bench --bench {} (em {})", bench_file, bench_dir.display()),
        level: super::app::LogLevel::Info,
    });

    // Executar cargo bench de forma assíncrona
    let mut cmd = TokioCommand::new("cargo");
    cmd.arg("bench")
        .arg("--bench")
        .arg(&bench_file)
        .current_dir(&bench_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    match cmd.spawn() {
        Ok(mut child) => {
            let _ = tx.send(AppEvent::BenchmarkLog {
                message: "⏳ Aguardando conclusão do benchmark...".to_string(),
                level: super::app::LogLevel::Info,
            });

            // Aguardar conclusão
            match child.wait().await {
                Ok(status) => {
                    let duration = start_time.elapsed();
                    let duration_secs = duration.as_secs_f64();

                    // Capturar output
                    let mut stdout = String::new();
                    let mut stderr = String::new();

                    if let Some(mut stdout_handle) = child.stdout.take() {
                        use tokio::io::AsyncReadExt;
                        let mut buf = Vec::new();
                        if stdout_handle.read_to_end(&mut buf).await.is_ok() {
                            stdout = String::from_utf8_lossy(&buf).to_string();
                        }
                    }

                    if let Some(mut stderr_handle) = child.stderr.take() {
                        use tokio::io::AsyncReadExt;
                        let mut buf = Vec::new();
                        if stderr_handle.read_to_end(&mut buf).await.is_ok() {
                            stderr = String::from_utf8_lossy(&buf).to_string();
                        }
                    }

                    let success = status.success();
                    let output = if !stdout.is_empty() {
                        stdout
                    } else {
                        stderr.clone()
                    };

                    // Enviar logs da saída
                    for line in output.lines().take(50) {
                        if !line.trim().is_empty() {
                            let level = if line.contains("error") || line.contains("Error") {
                                super::app::LogLevel::Error
                            } else if line.contains("warning") || line.contains("Warning") {
                                super::app::LogLevel::Warning
                            } else if line.contains("test result: ok") || line.contains("Benchmarking") {
                                super::app::LogLevel::Success
                            } else {
                                super::app::LogLevel::Info
                            };

                            let _ = tx.send(AppEvent::BenchmarkLog {
                                message: line.to_string(),
                                level,
                            });
                        }
                    }

                    // Enviar resultado final
                    let error = if !success && !stderr.is_empty() {
                        Some(stderr)
                    } else if !success {
                        Some("Benchmark falhou sem mensagem de erro".to_string())
                    } else {
                        None
                    };

                    let _ = tx.send(AppEvent::BenchmarkComplete {
                        bench_file: bench_file.clone(),
                        bench_name: bench_name.clone(),
                        success,
                        output,
                        error,
                        duration_secs,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::BenchmarkLog {
                        message: format!("❌ Erro ao aguardar processo: {}", e),
                        level: super::app::LogLevel::Error,
                    });
                    let _ = tx.send(AppEvent::BenchmarkComplete {
                        bench_file: bench_file.clone(),
                        bench_name: bench_name.clone(),
                        success: false,
                        output: String::new(),
                        error: Some(format!("Erro ao executar: {}", e)),
                        duration_secs: start_time.elapsed().as_secs_f64(),
                    });
                }
            }
        }
        Err(e) => {
            let _ = tx.send(AppEvent::BenchmarkLog {
                message: format!("❌ Erro ao iniciar processo: {}", e),
                level: super::app::LogLevel::Error,
            });
            let _ = tx.send(AppEvent::BenchmarkComplete {
                bench_file: bench_file.clone(),
                bench_name: bench_name.clone(),
                success: false,
                output: String::new(),
                error: Some(format!("Erro ao iniciar cargo bench: {}", e)),
                duration_secs: start_time.elapsed().as_secs_f64(),
            });
        }
    }
}
