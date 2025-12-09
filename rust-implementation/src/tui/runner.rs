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

/// Cria o schema de campos dinâmicos para um benchmark específico
fn create_benchmark_schema(bench_name: &str) -> Vec<super::app::BenchmarkDynamicField> {
    use super::app::BenchmarkDynamicField;

    match bench_name {
        "Personas" => vec![
            BenchmarkDynamicField::new("status", "Status", 0)
                .with_icon("🎯")
                .with_group("Geral"),
            BenchmarkDynamicField::new("compiling", "Compilação", 1)
                .with_icon("🔨")
                .with_group("Geral"),
            BenchmarkDynamicField::new("running", "Execução", 2)
                .with_icon("🔄")
                .with_group("Geral"),
            BenchmarkDynamicField::new("warmup", "Warm-up", 10)
                .with_icon("🔥")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("collecting", "Coleta", 11)
                .with_icon("📊")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("analyzing", "Análise", 12)
                .with_icon("🔬")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("time_mean", "Tempo Médio", 20)
                .with_icon("⏱️")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("time_std", "Desvio Padrão", 21)
                .with_icon("📐")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("throughput", "Throughput", 22)
                .with_icon("🚀")
                .with_group("Resultados"),
        ],
        "Search" => vec![
            BenchmarkDynamicField::new("status", "Status", 0)
                .with_icon("🎯")
                .with_group("Geral"),
            BenchmarkDynamicField::new("compiling", "Compilação", 1)
                .with_icon("🔨")
                .with_group("Geral"),
            BenchmarkDynamicField::new("running", "Execução", 2)
                .with_icon("🔄")
                .with_group("Geral"),
            BenchmarkDynamicField::new("warmup", "Warm-up", 10)
                .with_icon("🔥")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("collecting", "Coleta", 11)
                .with_icon("📊")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("time_mean", "Tempo Médio", 20)
                .with_icon("⏱️")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("cache_hits", "Cache Hits", 21)
                .with_icon("💾")
                .with_group("Resultados"),
        ],
        "Evaluation" => vec![
            BenchmarkDynamicField::new("status", "Status", 0)
                .with_icon("🎯")
                .with_group("Geral"),
            BenchmarkDynamicField::new("compiling", "Compilação", 1)
                .with_icon("🔨")
                .with_group("Geral"),
            BenchmarkDynamicField::new("warmup", "Warm-up", 10)
                .with_icon("🔥")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("time_mean", "Tempo Médio", 20)
                .with_icon("⏱️")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("accuracy", "Precisão", 21)
                .with_icon("🎯")
                .with_group("Resultados"),
        ],
        "Agent" => vec![
            BenchmarkDynamicField::new("status", "Status", 0)
                .with_icon("🎯")
                .with_group("Geral"),
            BenchmarkDynamicField::new("compiling", "Compilação", 1)
                .with_icon("🔨")
                .with_group("Geral"),
            BenchmarkDynamicField::new("warmup", "Warm-up", 10)
                .with_icon("🔥")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("time_mean", "Tempo Médio", 20)
                .with_icon("⏱️")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("memory", "Memória", 21)
                .with_icon("💾")
                .with_group("Resultados"),
        ],
        "E2E" => vec![
            BenchmarkDynamicField::new("status", "Status", 0)
                .with_icon("🎯")
                .with_group("Geral"),
            BenchmarkDynamicField::new("compiling", "Compilação", 1)
                .with_icon("🔨")
                .with_group("Geral"),
            BenchmarkDynamicField::new("setup", "Setup", 2)
                .with_icon("⚙️")
                .with_group("Geral"),
            BenchmarkDynamicField::new("warmup", "Warm-up", 10)
                .with_icon("🔥")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("time_total", "Tempo Total", 20)
                .with_icon("⏱️")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("steps", "Steps Executados", 21)
                .with_icon("📝")
                .with_group("Resultados"),
        ],
        "SIMD" => vec![
            BenchmarkDynamicField::new("status", "Status", 0)
                .with_icon("🎯")
                .with_group("Geral"),
            BenchmarkDynamicField::new("compiling", "Compilação", 1)
                .with_icon("🔨")
                .with_group("Geral"),
            BenchmarkDynamicField::new("warmup", "Warm-up", 10)
                .with_icon("🔥")
                .with_group("Benchmark"),
            BenchmarkDynamicField::new("time_simd", "Tempo SIMD", 20)
                .with_icon("⚡")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("time_scalar", "Tempo Scalar", 21)
                .with_icon("🔢")
                .with_group("Resultados"),
            BenchmarkDynamicField::new("speedup", "Speedup", 22)
                .with_icon("🚀")
                .with_group("Resultados"),
        ],
        _ => vec![
            BenchmarkDynamicField::new("status", "Status", 0)
                .with_icon("🎯")
                .with_group("Geral"),
            BenchmarkDynamicField::new("compiling", "Compilação", 1)
                .with_icon("🔨")
                .with_group("Geral"),
            BenchmarkDynamicField::new("result", "Resultado", 10)
                .with_icon("📊")
                .with_group("Resultados"),
        ],
    }
}

/// Analisa uma linha de log e extrai resultados dinâmicos
fn parse_benchmark_line(line: &str, bench_name: &str, tx: &Sender<AppEvent>) {
    use super::app::{AppEvent, FieldStatus};

    // Detectar compilação
    if line.contains("Compiling") {
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "compiling".to_string(),
            value: "Em andamento...".to_string(),
            status: FieldStatus::Running,
        });
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "status".to_string(),
            value: "Compilando".to_string(),
            status: FieldStatus::Running,
        });
    }

    // Detectar fim de compilação
    if line.contains("Finished") || line.contains("Compiled") {
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "compiling".to_string(),
            value: "Concluída ✓".to_string(),
            status: FieldStatus::Success,
        });
    }

    // Detectar início de execução
    if line.contains("Running") && line.contains("bench") {
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "running".to_string(),
            value: "Executando...".to_string(),
            status: FieldStatus::Running,
        });
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "status".to_string(),
            value: "Executando benchmark".to_string(),
            status: FieldStatus::Running,
        });
    }

    // Detectar warmup
    if line.contains("Warming up") || line.contains("warm") {
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "warmup".to_string(),
            value: "Aquecendo...".to_string(),
            status: FieldStatus::Running,
        });
    }

    // Detectar coleta de amostras
    if line.contains("Collecting") && line.contains("sample") {
        // Tentar extrair número de samples
        let samples = if let Some(num) = line.split_whitespace()
            .find(|s| s.parse::<u32>().is_ok())
        {
            format!("{} samples", num)
        } else {
            "Coletando...".to_string()
        };
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "collecting".to_string(),
            value: samples,
            status: FieldStatus::Running,
        });
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "warmup".to_string(),
            value: "Concluído ✓".to_string(),
            status: FieldStatus::Success,
        });
    }

    // Detectar análise
    if line.contains("Analyzing") {
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "analyzing".to_string(),
            value: "Analisando...".to_string(),
            status: FieldStatus::Running,
        });
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "collecting".to_string(),
            value: "Concluído ✓".to_string(),
            status: FieldStatus::Success,
        });
    }

    // Detectar resultados de tempo (padrão Criterion)
    // Exemplo: "time:   [1.2345 ms 1.2456 ms 1.2567 ms]"
    if line.contains("time:") && line.contains("[") {
        // Extrair valores entre colchetes
        if let Some(start) = line.find('[') {
            if let Some(end) = line.find(']') {
                let time_str = &line[start + 1..end];
                let parts: Vec<&str> = time_str.split_whitespace().collect();

                // Formato típico: "1.2345 ms 1.2456 ms 1.2567 ms" (low, mean, high)
                if parts.len() >= 4 {
                    let mean = format!("{} {}", parts[2], parts[3]); // meio
                    let _ = tx.send(AppEvent::BenchmarkUpdateField {
                        field_id: "time_mean".to_string(),
                        value: mean,
                        status: FieldStatus::Success,
                    });
                }
            }
        }
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "analyzing".to_string(),
            value: "Concluído ✓".to_string(),
            status: FieldStatus::Success,
        });
    }

    // Detectar throughput
    if line.contains("thrpt:") || line.contains("throughput") {
        if let Some(start) = line.find('[') {
            if let Some(end) = line.find(']') {
                let thrpt_str = &line[start + 1..end];
                let parts: Vec<&str> = thrpt_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    let thrpt = format!("{} {}", parts[0], parts[1]);
                    let _ = tx.send(AppEvent::BenchmarkUpdateField {
                        field_id: "throughput".to_string(),
                        value: thrpt,
                        status: FieldStatus::Success,
                    });
                }
            }
        }
    }

    // Detectar erro
    if line.contains("error") || line.contains("Error") || line.contains("FAILED") {
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "status".to_string(),
            value: "Erro!".to_string(),
            status: FieldStatus::Failed,
        });
    }

    // Detectar sucesso final
    if line.contains("test result: ok") || line.contains("benchmark complete") {
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "status".to_string(),
            value: "Sucesso!".to_string(),
            status: FieldStatus::Success,
        });
        let _ = tx.send(AppEvent::BenchmarkUpdateField {
            field_id: "running".to_string(),
            value: "Concluído ✓".to_string(),
            status: FieldStatus::Success,
        });
    }

    // Tratamentos específicos por tipo de benchmark
    match bench_name {
        "SIMD" => {
            // Detectar resultados específicos de SIMD
            if line.contains("simd") && line.contains("time:") {
                // Já tratado acima, mas podemos especializar
            }
            if line.contains("scalar") && line.contains("time:") {
                if let Some(start) = line.find('[') {
                    if let Some(end) = line.find(']') {
                        let time_str = &line[start + 1..end];
                        let parts: Vec<&str> = time_str.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let mean = format!("{} {}", parts[2], parts[3]);
                            let _ = tx.send(AppEvent::BenchmarkUpdateField {
                                field_id: "time_scalar".to_string(),
                                value: mean,
                                status: FieldStatus::Success,
                            });
                        }
                    }
                }
            }
            if line.to_lowercase().contains("speedup") {
                // Tentar extrair valor de speedup
                for word in line.split_whitespace() {
                    if word.contains("x") || word.parse::<f64>().is_ok() {
                        let _ = tx.send(AppEvent::BenchmarkUpdateField {
                            field_id: "speedup".to_string(),
                            value: word.to_string(),
                            status: FieldStatus::Success,
                        });
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

/// Executa um benchmark em background e envia eventos para a TUI (com logs em tempo real)
pub async fn execute_benchmark(bench_file: String, bench_name: String, tx: Sender<AppEvent>) {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let start_time = Instant::now();

    // Enviar schema de campos esperados para este benchmark
    let schema = create_benchmark_schema(&bench_name);
    let _ = tx.send(AppEvent::BenchmarkSetSchema {
        bench_name: bench_name.clone(),
        fields: schema,
    });

    // Inicializar status como pendente
    let _ = tx.send(AppEvent::BenchmarkUpdateField {
        field_id: "status".to_string(),
        value: "Iniciando...".to_string(),
        status: super::app::FieldStatus::Running,
    });

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

    // Executar cargo bench de forma assíncrona com --nocapture para saída em tempo real
    let mut cmd = TokioCommand::new("cargo");
    cmd.arg("bench")
        .arg("--bench")
        .arg(&bench_file)
        .arg("--")
        .arg("--nocapture")  // Força saída imediata
        .current_dir(&bench_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    match cmd.spawn() {
        Ok(mut child) => {
            let _ = tx.send(AppEvent::BenchmarkLog {
                message: "⏳ Executando benchmark (logs em tempo real)...".to_string(),
                level: super::app::LogLevel::Info,
            });

            // Criar readers para stdout e stderr
            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            // Clonar tx para as tasks de leitura
            let tx_stdout = tx.clone();
            let tx_stderr = tx.clone();

            // Clonar bench_name para as tasks
            let bench_name_stdout = bench_name.clone();
            let bench_name_stderr = bench_name.clone();

            // Coletar saída enquanto lê em tempo real
            let stdout_handle = tokio::spawn(async move {
                let mut lines = Vec::new();
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    if !line.trim().is_empty() {
                        let level = if line.contains("error") || line.contains("Error") {
                            super::app::LogLevel::Error
                        } else if line.contains("warning") || line.contains("Warning") {
                            super::app::LogLevel::Warning
                        } else if line.contains("test result: ok") || line.contains("Benchmarking") || line.contains("time:") {
                            super::app::LogLevel::Success
                        } else {
                            super::app::LogLevel::Info
                        };

                        let _ = tx_stdout.send(AppEvent::BenchmarkLog {
                            message: line.clone(),
                            level,
                        });

                        // Parsear linha para extrair resultados dinâmicos
                        parse_benchmark_line(&line, &bench_name_stdout, &tx_stdout);
                    }
                    lines.push(line);
                }
                lines
            });

            let stderr_handle = tokio::spawn(async move {
                let mut lines = Vec::new();
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    if !line.trim().is_empty() {
                        let level = if line.contains("error") || line.contains("Error") {
                            super::app::LogLevel::Error
                        } else if line.contains("warning") || line.contains("Warning") {
                            super::app::LogLevel::Warning
                        } else if line.contains("Compiling") || line.contains("Finished") {
                            super::app::LogLevel::Info
                        } else {
                            super::app::LogLevel::Debug
                        };

                        let _ = tx_stderr.send(AppEvent::BenchmarkLog {
                            message: line.clone(),
                            level,
                        });

                        // Parsear linha para extrair resultados dinâmicos
                        parse_benchmark_line(&line, &bench_name_stderr, &tx_stderr);
                    }
                    lines.push(line);
                }
                lines
            });

            // Aguardar processo terminar
            let status = child.wait().await;

            // Aguardar leitura completa
            let stdout_lines = stdout_handle.await.unwrap_or_default();
            let stderr_lines = stderr_handle.await.unwrap_or_default();

            let duration = start_time.elapsed();
            let duration_secs = duration.as_secs_f64();

            let stdout = stdout_lines.join("\n");
            let stderr = stderr_lines.join("\n");

            match status {
                Ok(s) => {
                    let success = s.success();
                    let output = if !stdout.is_empty() {
                        stdout
                    } else {
                        stderr.clone()
                    };

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
