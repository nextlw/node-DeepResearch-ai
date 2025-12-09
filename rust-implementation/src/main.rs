// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEEP RESEARCH CLI
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// CLI para execução do agente de pesquisa profunda.
//
// Uso:
//   deep-research-cli "Qual é a população do Brasil?"
//   deep-research-cli --tui "pergunta"  (modo TUI interativo)
//   deep-research-cli --budget 500000 "pergunta complexa"
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use deep_research::config::{
    create_tokio_runtime, install_panic_hook, load_runtime_config, RuntimeConfig,
    load_llm_config, load_agent_config, LlmConfig, AgentConfig,
};
use deep_research::llm::OpenAiClient;
use deep_research::prelude::*;
use deep_research::reader_comparison::ReaderComparison;
use deep_research::search::JinaClient;
use deep_research::tui::create_event_channel;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

/// Configuração global do runtime (carregada uma vez, thread-safe)
static RUNTIME_CONFIG: OnceLock<RuntimeConfig> = OnceLock::new();
/// Configuração global do LLM
static LLM_CONFIG: OnceLock<LlmConfig> = OnceLock::new();
/// Configuração global do agente
static AGENT_CONFIG: OnceLock<AgentConfig> = OnceLock::new();

/// Obtém a configuração do runtime (thread-safe)
fn get_runtime_config() -> &'static RuntimeConfig {
    RUNTIME_CONFIG.get().expect("Runtime config not initialized")
}

/// Obtém a configuração do LLM (thread-safe)
fn get_llm_config() -> &'static LlmConfig {
    LLM_CONFIG.get().expect("LLM config not initialized")
}

/// Obtém a configuração do agente (thread-safe)
#[allow(dead_code)]
fn get_agent_config() -> &'static AgentConfig {
    AGENT_CONFIG.get().expect("Agent config not initialized")
}

/// Tenta carregar o arquivo .env de múltiplos locais possíveis
fn load_dotenv() {
    // Lista de possíveis locais para o .env
    let possible_paths = [
        // Diretório atual
        PathBuf::from(".env"),
        // Diretório pai (se executando de rust-implementation)
        PathBuf::from("../.env"),
        // Caminho absoluto em tempo de compilação (fallback)
        {
            let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.pop();
            p.push(".env");
            p
        },
    ];

    for path in &possible_paths {
        if path.exists() {
            match dotenvy::from_path(path) {
                Ok(_) => {
                    eprintln!(
                        "✓ Carregado .env de: {:?}",
                        path.canonicalize().unwrap_or(path.clone())
                    );
                    return;
                }
                Err(e) => {
                    eprintln!("⚠ Erro ao carregar {:?}: {}", path, e);
                }
            }
        }
    }

    // Última tentativa: dotenvy padrão
    if dotenvy::dotenv().is_ok() {
        eprintln!("✓ Carregado .env do diretório atual");
    } else {
        eprintln!("⚠ Nenhum arquivo .env encontrado. Certifique-se de que OPENAI_API_KEY e JINA_API_KEY estão definidas.");
    }
}

fn main() -> anyhow::Result<()> {
    // Carregar .env PRIMEIRO, antes de qualquer coisa
    load_dotenv();

    // Parse argumentos ANTES de inicializar logging
    let args: Vec<String> = std::env::args().collect();
    let is_tui_mode = args.len() >= 2 && args[1] == "--tui";

    // Inicializar logging apenas se NÃO for modo TUI
    // (TUI não funciona com env_logger pois corrompe a tela)
    if !is_tui_mode {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Carregar configuração do runtime a partir do .env
    let config = load_runtime_config();

    // Carregar configuração do LLM a partir do .env
    let llm_config = load_llm_config();

    // Carregar configuração do agente a partir do .env
    let agent_config = load_agent_config();

    // Log da configuração efetiva
    let effective_threads = config.effective_worker_threads();
    if !is_tui_mode {
        log::info!(
            "🚀 Runtime: {} threads (max: {}) | WebReader: {}",
            effective_threads,
            config.max_threads,
            config.webreader
        );
        log::info!(
            "🤖 LLM: {} | Model: {}",
            llm_config.provider,
            llm_config.model
        );
        log::info!(
            "🔢 Embeddings: {} | Model: {}",
            llm_config.embedding_provider,
            llm_config.active_embedding_model()
        );
        log::info!(
            "⚙️  Agent: min_steps={} | allow_direct={} | budget={}",
            agent_config.min_steps_before_answer,
            agent_config.allow_direct_answer,
            agent_config.default_token_budget
        );
    }

    // Armazenar configurações globalmente para acesso em outras funções (thread-safe)
    RUNTIME_CONFIG.set(config.clone()).expect("Runtime config already initialized");
    LLM_CONFIG.set(llm_config.clone()).expect("LLM config already initialized");
    AGENT_CONFIG.set(agent_config.clone()).expect("Agent config already initialized");

    // Instalar panic hook customizado (isolamento de threads)
    install_panic_hook();

    // Criar runtime Tokio com configuração customizada
    let runtime = create_tokio_runtime(&config)?;

    // Executar main async dentro do runtime customizado
    runtime.block_on(async_main(args, is_tui_mode))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MENU DE INICIALIZAÇÃO (LAUNCHER)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Opções do menu de launcher
#[derive(Clone, Copy, PartialEq)]
enum LauncherOption {
    TuiMode,
    DirectSearch,
    CompareReaders,
    CompareLive,
    Help,
    Quit,
}

impl LauncherOption {
    fn all() -> Vec<Self> {
        vec![
            Self::TuiMode,
            Self::DirectSearch,
            Self::CompareReaders,
            Self::CompareLive,
            Self::Help,
            Self::Quit,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::TuiMode => "🖥️  Interface TUI Interativa",
            Self::DirectSearch => "🔍 Pesquisa Direta (terminal)",
            Self::CompareReaders => "📊 Comparar Web Readers",
            Self::CompareLive => "⚡ Pesquisa com Comparação Live",
            Self::Help => "❓ Ajuda (mostrar comandos)",
            Self::Quit => "❌ Sair",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::TuiMode => "Abre a interface visual completa com histórico e follow-ups",
            Self::DirectSearch => "Executa pesquisa no terminal (modo clássico)",
            Self::CompareReaders => "Compara Jina Reader vs Rust+OpenAI em URLs específicas",
            Self::CompareLive => "Pesquisa com comparação de readers em tempo real",
            Self::Help => "Mostra todos os comandos disponíveis",
            Self::Quit => "Encerra o programa",
        }
    }
}

/// Estado do menu launcher
struct LauncherState {
    selected: usize,
    options: Vec<LauncherOption>,
    input_mode: bool,
    input_text: String,
    input_label: String,
}

impl LauncherState {
    fn new() -> Self {
        Self {
            selected: 0,
            options: LauncherOption::all(),
            input_mode: false,
            input_text: String::new(),
            input_label: String::new(),
        }
    }

    fn selected_option(&self) -> LauncherOption {
        self.options[self.selected]
    }

    fn next(&mut self) {
        self.selected = (self.selected + 1) % self.options.len();
    }

    fn prev(&mut self) {
        self.selected = if self.selected == 0 {
            self.options.len() - 1
        } else {
            self.selected - 1
        };
    }
}

/// Renderiza o menu launcher
fn render_launcher(frame: &mut ratatui::Frame, state: &LauncherState) {
    use ratatui::layout::{Constraint, Direction, Layout, Rect};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

    let area = frame.area();

    // Background
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(20, 20, 30))),
        area,
    );

    // Layout principal
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(32), // Header/Logo (ASCII art + texto)
            Constraint::Length(1),  // Espaço
            Constraint::Min(10),    // Menu
            Constraint::Length(2),  // Footer
        ])
        .split(area);

    // Header com logo ASCII art - Do arquivo config/ascii-art.txt
    let blue = Color::Rgb(100, 150, 255);
    let accent = Color::Rgb(255, 200, 100);

    let logo = vec![
        Line::from(vec![Span::styled("", Style::default())]),
        Line::from(vec![Span::styled(" .................................................................................................. ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![Span::styled(" .................................................................................................. ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![Span::styled(" .................................................................................................. ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![Span::styled(" .......-.......................................................................................... ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![Span::styled(" .....................................                  ........................................... ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![
            Span::styled(" ...................................     ", Style::default().fg(Color::DarkGray)),
            Span::styled("############", Style::default().fg(blue)),
            Span::styled(".     ........................................ ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" .................................    ", Style::default().fg(Color::DarkGray)),
            Span::styled("###            ####", Style::default().fg(blue)),
            Span::styled("   ....................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ...............................    ", Style::default().fg(Color::DarkGray)),
            Span::styled("##                  ###", Style::default().fg(blue)),
            Span::styled("     ................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ..............................   ", Style::default().fg(Color::DarkGray)),
            Span::styled("##       #########       ###", Style::default().fg(blue)),
            Span::styled("             ........................ ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ..............................  ", Style::default().fg(Color::DarkGray)),
            Span::styled("+#     ###        ###       .####+", Style::default().fg(accent)),
            Span::styled("           ..................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" .............................   ", Style::default().fg(Color::DarkGray)),
            Span::styled("#     ##    ..      ##+ ...       +#########", Style::default().fg(accent)),
            Span::styled(" ..................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ............-...............   ", Style::default().fg(Color::DarkGray)),
            Span::styled("##    ##         ######+ .        #####+", Style::default().fg(accent)),
            Span::styled("      ..................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" .......................-...   ", Style::default().fg(Color::DarkGray)),
            Span::styled("##  .  #   -######          #######", Style::default().fg(accent)),
            Span::styled("           ...................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ..........................   ", Style::default().fg(Color::DarkGray)),
            Span::styled("##  ..  ####          .######", Style::default().fg(accent)),
            Span::styled("           ............................. ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" .......................    ", Style::default().fg(Color::DarkGray)),
            Span::styled("-##  ...         ########", Style::default().fg(accent)),
            Span::styled("         ..................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" .....................    ", Style::default().fg(Color::DarkGray)),
            Span::styled("###         ########        ######", Style::default().fg(accent)),
            Span::styled(" ..................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ..................     ", Style::default().fg(Color::DarkGray)),
            Span::styled("##      ######. ##          ##     ##", Style::default().fg(accent)),
            Span::styled(" ..................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ................     ", Style::default().fg(Color::DarkGray)),
            Span::styled("##########  ##      ##########       #-", Style::default().fg(accent)),
            Span::styled(" ..................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ................. ", Style::default().fg(Color::DarkGray)),
            Span::styled("#####.          ###                   ##", Style::default().fg(accent)),
            Span::styled("   ..................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" .................          ......   ", Style::default().fg(Color::DarkGray)),
            Span::styled("###.             ###", Style::default().fg(accent)),
            Span::styled("    ...................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" .................    .............     ", Style::default().fg(Color::DarkGray)),
            Span::styled("#######+######", Style::default().fg(accent)),
            Span::styled("     ........................................ ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" ..........  .........................        ", Style::default().fg(Color::DarkGray)),
            Span::styled("++-", Style::default().fg(accent)),
            Span::styled("        .......................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![Span::styled(" ..........................................       ................................................. ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![Span::styled(" ............. .................................................................................... ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![Span::styled(" ......................................................................................--.......... ", Style::default().fg(Color::DarkGray))]),
        Line::from(vec![
            Span::styled(" ................................... -.......-", Style::default().fg(Color::DarkGray)),
            Span::styled("+", Style::default().fg(accent)),
            Span::styled(".................................................... ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![Span::styled("", Style::default())]),
        Line::from(vec![
            Span::styled("              ", Style::default()),
            Span::styled("  DEEP RESEARCH", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("              ", Style::default()),
            Span::styled("  Pesquisa Profunda", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("              ", Style::default()),
            Span::styled(format!("  v{}", deep_research::VERSION), Style::default().fg(Color::Yellow)),
        ]),
    ];

    let header = Paragraph::new(logo);
    frame.render_widget(header, chunks[0]);

    // Menu central
    let menu_area = centered_rect(60, 70, chunks[2]);

    if state.input_mode {
        // Modo de input
        let input_block = Block::default()
            .title(format!(" {} ", state.input_label))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let input = Paragraph::new(state.input_text.as_str())
            .style(Style::default().fg(Color::White))
            .block(input_block);

        frame.render_widget(input, menu_area);

        // Mostrar cursor
        frame.set_cursor_position((
            menu_area.x + state.input_text.len() as u16 + 1,
            menu_area.y + 1,
        ));
    } else {
        // Lista de opções
        let items: Vec<ListItem> = state
            .options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let style = if i == state.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if i == state.selected { "▶ " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(opt.label(), style),
                ]))
            })
            .collect();

        let menu = List::new(items)
            .block(
                Block::default()
                    .title(" Selecione uma opção ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(menu, menu_area);

        // Descrição da opção selecionada
        let desc_area = Rect {
            x: menu_area.x,
            y: menu_area.y + menu_area.height + 1,
            width: menu_area.width,
            height: 2,
        };

        let description = Paragraph::new(state.selected_option().description())
            .style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC));

        frame.render_widget(description, desc_area);
    }

    // Footer
    let footer_text = if state.input_mode {
        "Enter: Confirmar │ Esc: Cancelar"
    } else {
        "↑↓: Navegar │ Enter: Selecionar │ q: Sair"
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(footer, chunks[3]);
}

/// Helper para centralizar um retângulo
fn centered_rect(percent_x: u16, percent_y: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Resultado do menu launcher
enum LauncherResult {
    RunTui(String),
    RunDirect(String, Option<u64>),
    RunCompare(String),
    RunCompareLive(String),
    ShowHelp,
    Quit,
}

/// Executa o menu launcher
fn run_launcher_menu() -> std::io::Result<LauncherResult> {
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = LauncherState::new();
    let result;

    loop {
        terminal.draw(|f| render_launcher(f, &state))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if state.input_mode {
                match key.code {
                    KeyCode::Enter => {
                        let text = state.input_text.clone();
                        result = match state.selected_option() {
                            LauncherOption::TuiMode => LauncherResult::RunTui(text),
                            LauncherOption::DirectSearch => LauncherResult::RunDirect(text, None),
                            LauncherOption::CompareReaders => LauncherResult::RunCompare(text),
                            LauncherOption::CompareLive => LauncherResult::RunCompareLive(text),
                            _ => LauncherResult::Quit,
                        };
                        break;
                    }
                    KeyCode::Esc => {
                        state.input_mode = false;
                        state.input_text.clear();
                    }
                    KeyCode::Char(c) => {
                        state.input_text.push(c);
                    }
                    KeyCode::Backspace => {
                        state.input_text.pop();
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => state.prev(),
                    KeyCode::Down | KeyCode::Char('j') => state.next(),
                    KeyCode::Enter => {
                        match state.selected_option() {
                            LauncherOption::TuiMode => {
                                // TUI pode ser aberto direto ou com pergunta
                                state.input_mode = true;
                                state.input_label = "Pergunta (ou Enter vazio para interface)".to_string();
                            }
                            LauncherOption::DirectSearch => {
                                state.input_mode = true;
                                state.input_label = "Digite sua pergunta".to_string();
                            }
                            LauncherOption::CompareReaders => {
                                state.input_mode = true;
                                state.input_label = "URLs para comparar (separadas por vírgula)".to_string();
                            }
                            LauncherOption::CompareLive => {
                                state.input_mode = true;
                                state.input_label = "Digite sua pergunta".to_string();
                            }
                            LauncherOption::Help => {
                                result = LauncherResult::ShowHelp;
                                break;
                            }
                            LauncherOption::Quit => {
                                result = LauncherResult::Quit;
                                break;
                            }
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        result = LauncherResult::Quit;
                        break;
                    }
                    KeyCode::Char('1') => {
                        state.selected = 0;
                    }
                    KeyCode::Char('2') => {
                        state.selected = 1;
                    }
                    KeyCode::Char('3') => {
                        state.selected = 2;
                    }
                    KeyCode::Char('4') => {
                        state.selected = 3;
                    }
                    KeyCode::Char('5') => {
                        state.selected = 4;
                    }
                    KeyCode::Char('6') => {
                        state.selected = 5;
                    }
                    _ => {}
                }
            }
        }
    }

    // Restaurar terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(result)
}

/// Mostra help no terminal
fn show_help(program_name: &str) {
    println!("Deep Research CLI v{}", deep_research::VERSION);
    println!();
    println!("Uso: {} <pergunta>", program_name);
    println!();
    println!("Opções:");
    println!("  --tui [pergunta]      Modo TUI interativo (com campo de texto)");
    println!("  --budget <tokens>     Budget máximo de tokens (padrão: 1000000)");
    println!("  --compare <urls>      Comparar Jina Reader vs Rust+OpenAI (URLs separadas por vírgula)");
    println!("  --compare-live        Habilita comparação Jina vs Rust durante pesquisa");
    println!();
    println!("Exemplos:");
    println!("  {} \"Qual é a população do Brasil?\"", program_name);
    println!("  {} --tui                              # Abre interface para digitar", program_name);
    println!("  {} --tui \"Qual é a capital da França?\"", program_name);
    println!("  {} --compare \"https://example.com,https://rust-lang.org\"", program_name);
    println!("  {} --compare-live \"pergunta\"         # Pesquisa com comparação", program_name);
    println!();
    println!("Features de compilação:");
    println!("  cargo build --release                           # Produção (sem clipboard)");
    println!("  cargo build --release --features clipboard      # Com suporte a clipboard");
    println!("  cargo build --release --features postgres       # Com PostgreSQL");
    println!("  cargo build --release --features qdrant         # Com Qdrant");
}

async fn async_main(args: Vec<String>, is_tui_mode: bool) -> anyhow::Result<()> {

    // Se não tem argumentos, mostra o menu launcher
    if args.len() < 2 {
        match run_launcher_menu()? {
            LauncherResult::RunTui(question) => {
                return run_tui_mode(&question).await;
            }
            LauncherResult::RunDirect(question, budget) => {
                if question.is_empty() {
                    eprintln!("✗ Erro: Pergunta não pode ser vazia!");
        std::process::exit(1);
                }
                // Continua abaixo com a pergunta
                return run_direct_mode(&question, budget, false).await;
            }
            LauncherResult::RunCompare(urls) => {
                if urls.is_empty() {
                    eprintln!("✗ Erro: URLs não podem ser vazias!");
                    std::process::exit(1);
                }
                return run_comparison_mode(&urls).await;
            }
            LauncherResult::RunCompareLive(question) => {
                if question.is_empty() {
                    eprintln!("✗ Erro: Pergunta não pode ser vazia!");
                    std::process::exit(1);
                }
                return run_direct_mode(&question, None, true).await;
            }
            LauncherResult::ShowHelp => {
                show_help(&args[0]);
                return Ok(());
            }
            LauncherResult::Quit => {
                return Ok(());
            }
        }
    }

    // Modo TUI
    if is_tui_mode {
        // Se tem pergunta após --tui, usa ela; senão abre input interativo
        let question = if args.len() > 2 {
            args[2..].join(" ")
        } else {
            String::new()
        };
        return run_tui_mode(&question).await;
    }

    // Modo comparação standalone
    if args.len() >= 3 && args[1] == "--compare" {
        return run_comparison_mode(&args[2]).await;
    }

    // Verificar flag de comparação em tempo real
    let enable_compare_live = args.iter().any(|a| a == "--compare-live");

    // Parse budget e question (considerando --compare-live)
    let (budget, question) = {
        let filtered_args: Vec<&str> = args
            .iter()
            .skip(1)
            .filter(|a| *a != "--compare-live")
            .map(|s| s.as_str())
            .collect();

        if filtered_args.len() >= 3 && filtered_args[0] == "--budget" {
            let budget: u64 = filtered_args[1].parse().unwrap_or(1_000_000);
            let question = filtered_args[2..].join(" ");
        (Some(budget), question)
    } else {
            (None, filtered_args.join(" "))
        }
    };

    run_direct_mode(&question, budget, enable_compare_live).await
}

/// Executa o modo de pesquisa direta no terminal
async fn run_direct_mode(question: &str, budget: Option<u64>, enable_compare_live: bool) -> anyhow::Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" DEEP RESEARCH v{}", deep_research::VERSION);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Pergunta: {}", question);
    if let Some(b) = budget {
        println!("Budget: {} tokens", b);
    }
    if enable_compare_live {
        println!("🔬 Modo de comparação: Jina vs Rust local ATIVADO");
    }
    println!();

    // Criar clientes reais com API keys de variáveis de ambiente
    let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: OPENAI_API_KEY não encontrada!");
        eprintln!();
        eprintln!("Certifique-se de que:");
        eprintln!("  1. O arquivo .env existe no diretório raiz do projeto");
        eprintln!("  2. O arquivo contém: OPENAI_API_KEY=sua-chave-aqui");
        eprintln!();
        eprintln!("Ou defina a variável de ambiente diretamente:");
        eprintln!("  export OPENAI_API_KEY=sua-chave-aqui");
        std::process::exit(1);
    });

    let jina_key = std::env::var("JINA_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: JINA_API_KEY não encontrada!");
        eprintln!();
        eprintln!("Certifique-se de que:");
        eprintln!("  1. O arquivo .env existe no diretório raiz do projeto");
        eprintln!("  2. O arquivo contém: JINA_API_KEY=sua-chave-aqui");
        eprintln!();
        eprintln!("Ou defina a variável de ambiente diretamente:");
        eprintln!("  export JINA_API_KEY=sua-chave-aqui");
        std::process::exit(1);
    });

    // Criar cliente LLM com configuração do .env
    let llm_client: Arc<dyn deep_research::llm::LlmClient> =
        Arc::new(OpenAiClient::from_config(openai_key, get_llm_config()));

    // Usar preferência de WebReader da configuração global
    let webreader_pref = get_runtime_config().webreader;
    let search_client: Arc<dyn deep_research::search::SearchClient> =
        Arc::new(JinaClient::with_preference(jina_key, webreader_pref));

    // Criar e executar agente
    let agent = DeepResearchAgent::new(llm_client, search_client, budget)
        .with_comparative_read(enable_compare_live);

    println!("Iniciando pesquisa...");
    println!();

    let result = agent.run(question.to_string()).await;

    // Exibir resultado
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" RESULTADO");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if result.success {
        println!("✓ Pesquisa concluída com sucesso!");
        println!();

        if result.trivial {
            println!("[Pergunta trivial - resposta direta]");
            println!();
        }

        if let Some(answer) = &result.answer {
            println!("Resposta:");
            println!("{}", answer);
            println!();
        }

        if !result.references.is_empty() {
            println!("Referências:");
            for (i, reference) in result.references.iter().enumerate() {
                println!("  {}. {} - {}", i + 1, reference.title, reference.url);
            }
            println!();
        }
    } else {
        println!("✗ Pesquisa falhou");
        if let Some(error) = &result.error {
            println!("Erro: {}", error);
        }
        println!();
    }

    // Estatísticas
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" ESTATÍSTICAS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("⏱️  Tempo total: {:.2}s", result.total_time_ms as f64 / 1000.0);
    println!("    - Busca:   {}ms", result.search_time_ms);
    println!("    - Leitura: {}ms", result.read_time_ms);
    println!("    - LLM:     {}ms", result.llm_time_ms);
    println!();
    println!("🎫 Tokens utilizados:");
    println!("    - Prompt:     {}", result.token_usage.prompt_tokens);
    println!("    - Completion: {}", result.token_usage.completion_tokens);
    println!("    - Total:      {}", result.token_usage.total_tokens);
    println!();
    println!("🔗 URLs visitadas: {}", result.visited_urls.len());
    for url in &result.visited_urls {
        println!("    - {}", url);
    }
    println!();

    Ok(())
}

/// Executa o modo de comparação entre Jina Reader e Rust+OpenAI
async fn run_comparison_mode(urls_arg: &str) -> anyhow::Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" COMPARAÇÃO: JINA READER vs RUST + OPENAI GPT-4O-MINI");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: OPENAI_API_KEY não encontrada!");
        std::process::exit(1);
    });

    let jina_key = std::env::var("JINA_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: JINA_API_KEY não encontrada!");
        std::process::exit(1);
    });

    // Parse URLs
    let urls: Vec<&str> = urls_arg.split(',').map(|s| s.trim()).collect();
    println!("URLs para comparar: {:?}", urls);
    println!();

    let comparison = ReaderComparison::new(jina_key, openai_key);
    let results = comparison.compare_batch(&urls).await;

    // Exibir resultados detalhados
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" RESULTADOS DETALHADOS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    for result in &results {
        println!("URL: {}", result.url);
        println!(
            "  Vencedor: {} (diff: {}ms)",
            result.faster,
            result.time_diff_ms.abs()
        );

        if let Some(jina) = &result.jina {
            println!("  📘 Jina Reader:");
            println!("     - Tempo: {}ms", jina.time_ms);
            println!(
                "     - Título: {}",
                jina.title.chars().take(50).collect::<String>()
            );
            println!("     - Palavras: {}", jina.word_count);
            if let Some(err) = &jina.error {
                println!("     - Erro: {}", err);
            }
        }

        if let Some(openai) = &result.rust_openai {
            println!("  🤖 Rust + OpenAI:");
            println!("     - Tempo: {}ms", openai.time_ms);
            println!(
                "     - Título: {}",
                openai.title.chars().take(50).collect::<String>()
            );
            println!("     - Palavras: {}", openai.word_count);
            if let Some(err) = &openai.error {
                println!("     - Erro: {}", err);
            }
        }
        println!();
    }

    // Estatísticas finais
    let jina_wins = results.iter().filter(|r| r.faster == "jina").count();
    let openai_wins = results.iter().filter(|r| r.faster == "rust_openai").count();
    let jina_total_ms: u128 = results
        .iter()
        .filter_map(|r| r.jina.as_ref())
        .map(|j| j.time_ms)
        .sum();
    let openai_total_ms: u128 = results
        .iter()
        .filter_map(|r| r.rust_openai.as_ref())
        .map(|o| o.time_ms)
        .sum();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" RESUMO FINAL");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("📘 Jina Reader:");
    println!("   Vitórias: {}", jina_wins);
    println!("   Tempo total: {}ms", jina_total_ms);
    println!();
    println!("🤖 Rust + OpenAI gpt-4o-mini:");
    println!("   Vitórias: {}", openai_wins);
    println!("   Tempo total: {}ms", openai_total_ms);
    println!();

    if jina_total_ms < openai_total_ms {
        let speedup = (openai_total_ms as f64 / jina_total_ms as f64) * 100.0 - 100.0;
        println!("🏆 Jina Reader foi {:.1}% mais rápido no geral!", speedup);
    } else if openai_total_ms < jina_total_ms {
        let speedup = (jina_total_ms as f64 / openai_total_ms as f64) * 100.0 - 100.0;
        println!("🏆 Rust + OpenAI foi {:.1}% mais rápido no geral!", speedup);
    } else {
        println!("🏆 Empate!");
    }
    println!();

    Ok(())
}

/// Cria LoadedConfig a partir das configurações globais
fn create_loaded_config() -> deep_research::tui::LoadedConfig {
    use deep_research::tui::LoadedConfig;

    let runtime = get_runtime_config();
    let llm = get_llm_config();
    let agent = get_agent_config();

    LoadedConfig {
        // Runtime
        worker_threads: runtime.worker_threads
            .map(|w| format!("{}", w))
            .unwrap_or_else(|| format!("auto ({})", runtime.effective_worker_threads())),
        max_threads: runtime.max_threads,
        max_blocking_threads: runtime.max_blocking_threads,
        webreader: format!("{:?}", runtime.webreader),
        // LLM
        llm_provider: format!("{:?}", llm.provider),
        llm_model: llm.model.clone(),
        embedding_provider: format!("{:?}", llm.embedding_provider),
        embedding_model: llm.active_embedding_model().to_string().clone(),
        temperature: llm.default_temperature,
        api_base_url: llm.api_base_url.clone(),
        // Agent
        min_steps_before_answer: agent.min_steps_before_answer,
        allow_direct_answer: agent.allow_direct_answer,
        default_token_budget: agent.default_token_budget,
        max_urls_per_step: agent.max_urls_per_step,
        max_queries_per_step: agent.max_queries_per_step,
        max_consecutive_failures: agent.max_consecutive_failures,
        // API Keys (verificar presença)
        openai_key_present: std::env::var("OPENAI_API_KEY").is_ok(),
        jina_key_present: std::env::var("JINA_API_KEY").is_ok(),
    }
}

/// Executa o modo TUI interativo
async fn run_tui_mode(question: &str) -> anyhow::Result<()> {
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use deep_research::tui::{App, AppScreen, ActiveTab, execute_benchmark};
    use deep_research::agent::UserResponse;
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;
    use std::time::Duration;

    // Criar clientes
    let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: OPENAI_API_KEY não encontrada!");
        std::process::exit(1);
    });

    let jina_key = std::env::var("JINA_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: JINA_API_KEY não encontrada!");
        std::process::exit(1);
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Criar app - com ou sem pergunta inicial
    let mut app = if question.is_empty() {
        App::new()
    } else {
        App::with_question(question.to_string())
    };

    // Carregar configurações para exibição
    app.set_loaded_config(create_loaded_config());

    // Canal para eventos da TUI
    let (tx, rx) = create_event_channel();

    // Canal para enviar respostas do usuário de volta para o agente
    // Usamos Option para poder substituir quando uma nova pesquisa inicia
    let mut user_response_tx: Option<tokio::sync::mpsc::Sender<UserResponse>> = None;

    // Handle de tarefa do agente (opcional)
    let mut agent_task: Option<tokio::task::JoinHandle<_>> = None;

    // Se já tem pergunta, iniciar pesquisa
    if !question.is_empty() {
        let (new_tx, new_rx) = tokio::sync::mpsc::channel::<UserResponse>(16);
        user_response_tx = Some(new_tx);
        agent_task = Some(spawn_research_task(
            question.to_string(),
            openai_key.clone(),
            jina_key.clone(),
            tx.clone(),
            new_rx,
        ));
    }

    // Loop principal da TUI
    loop {
        // Atualizar métricas do sistema
        update_system_metrics(&mut app);

        // Renderizar
        terminal.draw(|frame| deep_research::tui::ui::render(frame, &app))?;

        // Processar eventos do agente (não bloqueante)
        while let Ok(event) = rx.try_recv() {
            app.handle_event(event);
        }

        // Processar input do usuário (com timeout curto)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.screen {
                        AppScreen::Input => {
                            match key.code {
                                KeyCode::Enter => {
                                    if !app.input_text.is_empty() {
                                        let q = app.input_text.clone();
                                        app.start_research();

                                        // Criar novo canal para resposta do usuário para esta pesquisa
                                        let (new_tx, new_rx) = tokio::sync::mpsc::channel::<UserResponse>(16);
                                        user_response_tx = Some(new_tx);

                                        agent_task = Some(spawn_research_task(
                                            q,
                                            openai_key.clone(),
                                            jina_key.clone(),
                                            tx.clone(),
                                            new_rx,
                                        ));
                                    }
                                }
                                KeyCode::Esc => {
                                    app.should_quit = true;
                                    break;
                                }
                                // Navegação por tabs (antes do Char genérico)
                                KeyCode::Tab => app.next_tab(),
                                KeyCode::Char('1') => app.go_to_tab(ActiveTab::Search),
                                KeyCode::Char('2') => app.go_to_tab(ActiveTab::Config),
                                KeyCode::Char('3') => app.go_to_tab(ActiveTab::Benchmarks),
                                // Caso genérico para caracteres (input de texto)
                                KeyCode::Char(c) => app.input_char(c),
                                KeyCode::Backspace => app.input_backspace(),
                                KeyCode::Delete => app.input_delete(),
                                KeyCode::Left => app.cursor_left(),
                                KeyCode::Right => app.cursor_right(),
                                KeyCode::Home => app.cursor_home(),
                                KeyCode::End => app.cursor_end(),
                                KeyCode::Up => app.history_up(),
                                KeyCode::Down => app.history_down(),
                                _ => {}
                            }
                        }
                        AppScreen::Research => {
                            // Se o input está focado, processar digitação
                            if app.input_focused {
                                match key.code {
                                    KeyCode::Tab | KeyCode::Esc => {
                                        // Desfocar input
                                        app.unfocus_input();
                                    }
                                    KeyCode::Enter => {
                                        // Enviar mensagem para a fila do agente
                                        if !app.input_text.is_empty() {
                                            let message = app.input_text.clone();
                                            app.queue_user_message(message.clone());

                                            // Enviar via canal para o agente (async, não bloqueia)
                                            if let Some(ref tx) = user_response_tx {
                                                let user_resp = UserResponse::spontaneous(message);
                                                let _ = tx.try_send(user_resp);
                                            }

                                            app.input_text.clear();
                                            app.cursor_pos = 0;
                                        }
                                    }
                                    KeyCode::Char(c) => app.input_char(c),
                                    KeyCode::Backspace => app.input_backspace(),
                                    KeyCode::Delete => app.input_delete(),
                                    KeyCode::Left => app.cursor_left(),
                                    KeyCode::Right => app.cursor_right(),
                                    KeyCode::Home => app.cursor_home(),
                                    KeyCode::End => app.cursor_end(),
                                    _ => {}
                                }
                            } else {
                                // Input não focado - navegação normal
                                match key.code {
                                    KeyCode::Char('q') => {
                                        app.should_quit = true;
                                        break;
                                    }
                                    KeyCode::Esc => {
                                        // Esc sem input focado = sair
                                        app.should_quit = true;
                                        break;
                                    }
                                    KeyCode::Tab => {
                                        // Focar no input
                                        app.focus_input();
                                    }
                                    // Navegação por tabs (números)
                                    KeyCode::Char('1') => app.go_to_tab(ActiveTab::Search),
                                    KeyCode::Char('2') => app.go_to_tab(ActiveTab::Config),
                                    // Alternar entre Research e Result quando concluído
                                    KeyCode::Char('r') if app.is_complete => {
                                        app.toggle_result_research();
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
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
                                    _ => {}
                                }
                            }
                        }
                        AppScreen::Result => {
                            // Se o input está focado, processar digitação
                            if app.input_focused {
                                match key.code {
                                    KeyCode::Tab | KeyCode::Esc => {
                                        // Desfocar input
                                        app.unfocus_input();
                                    }
                                    KeyCode::Enter => {
                                        // Iniciar nova pesquisa com o texto digitado
                                        if !app.input_text.is_empty() {
                                            let q = app.input_text.clone();
                                            app.reset();
                                            app.input_text = q.clone();
                                            app.start_research();

                                            // Criar novo canal para resposta do usuário
                                            let (new_tx, new_rx) = tokio::sync::mpsc::channel::<UserResponse>(16);
                                            user_response_tx = Some(new_tx);

                                            agent_task = Some(spawn_research_task(
                                                q,
                                                openai_key.clone(),
                                                jina_key.clone(),
                                                tx.clone(),
                                                new_rx,
                                            ));
                                        }
                                    }
                                    KeyCode::Char(c) => app.input_char(c),
                                    KeyCode::Backspace => app.input_backspace(),
                                    KeyCode::Delete => app.input_delete(),
                                    KeyCode::Left => app.cursor_left(),
                                    KeyCode::Right => app.cursor_right(),
                                    KeyCode::Home => app.cursor_home(),
                                    KeyCode::End => app.cursor_end(),
                                    _ => {}
                                }
                            } else {
                                // Input não focado - navegação normal
                                match key.code {
                                    KeyCode::Char('q') => {
                                        app.should_quit = true;
                                        break;
                                    }
                                    KeyCode::Esc => {
                                        app.should_quit = true;
                                        break;
                                    }
                                    KeyCode::Tab => {
                                        // Focar no input para follow-up
                                        app.focus_input();
                                    }
                                    // Navegação por tabs
                                    KeyCode::Char('1') => app.go_to_tab(ActiveTab::Search),
                                    KeyCode::Char('2') => app.go_to_tab(ActiveTab::Config),
                                    KeyCode::Char('3') => app.go_to_tab(ActiveTab::Benchmarks),
                                    // Alternar para ver logs da pesquisa
                                    KeyCode::Char('r') => app.toggle_result_research(),
                                    // Copiar resposta
                                    KeyCode::Char('c') => {
                                            #[cfg(feature = "clipboard")]
                                            {
                                            if let Some(answer) = &app.answer {
                                                // Tentar copiar para clipboard
                                                if let Ok(mut ctx) = arboard::Clipboard::new() {
                                                    if ctx.set_text(answer.clone()).is_ok() {
                                                        app.clipboard_message = Some("✅ Copiado!".to_string());
                                                    }
                                                    }
                                                }
                                            }
                                            #[cfg(not(feature = "clipboard"))]
                                            {
                                            if app.answer.is_some() {
                                                app.clipboard_message = Some("📋 Clipboard não disponível".to_string());
                                            }
                                        }
                                    }
                                    // Scroll na resposta
                                    KeyCode::Up | KeyCode::Char('k') => app.result_scroll_up(),
                                    KeyCode::Down | KeyCode::Char('j') => app.result_scroll_down(),
                                    KeyCode::PageUp => app.result_page_up(),
                                    KeyCode::PageDown => app.result_page_down(),
                                    KeyCode::Home => app.result_scroll = 0,
                                    KeyCode::End => app.result_scroll = usize::MAX,
                                    _ => {}
                                }
                            }
                        }
                        // Tela de configurações
                        AppScreen::Config => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.should_quit = true;
                                    break;
                                }
                                // Navegação por tabs
                                KeyCode::Tab => app.next_tab(),
                                KeyCode::Char('1') => app.go_to_tab(ActiveTab::Search),
                                KeyCode::Char('2') => app.go_to_tab(ActiveTab::Config),
                                KeyCode::Char('3') => app.go_to_tab(ActiveTab::Benchmarks),
                                // Backspace volta para tela anterior
                                KeyCode::Backspace => app.go_to_tab(ActiveTab::Search),
                                _ => {}
                            }
                        }
                        // Tela de benchmarks
                        AppScreen::Benchmarks => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.should_quit = true;
                                    break;
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
                                            let tx_clone = tx.clone();

                                            // Enviar evento de início
                                            let _ = tx.send(deep_research::tui::AppEvent::BenchmarkStarted {
                                                bench_file: bench_file.clone(),
                                                bench_name: bench_name.clone(),
                                            });

                                            tokio::spawn(async move {
                                                execute_benchmark(bench_file, bench_name, tx_clone).await;
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
                                // Navegação por tabs
                                KeyCode::Tab => app.next_tab(),
                                KeyCode::Char('1') => app.go_to_tab(ActiveTab::Search),
                                KeyCode::Char('2') => app.go_to_tab(ActiveTab::Config),
                                KeyCode::Char('3') => {
                                    // Já está em Benchmarks
                                }
                                KeyCode::Backspace => {
                                    // Voltar para pesquisa
                                    app.go_to_tab(ActiveTab::Search);
                                }
                                _ => {}
                            }
                        }
                        // Tela de input requerido pelo agente
                        AppScreen::InputRequired { ref question_id, .. } => {
                            match key.code {
                                KeyCode::Enter => {
                                    // Enviar resposta do usuário
                                    if !app.input_text.is_empty() {
                                        let response_text = app.input_text.clone();
                                        let qid = question_id.clone();

                                        // Enviar para a TUI (atualiza UI)
                                        app.handle_event(deep_research::tui::AppEvent::UserResponse {
                                            question_id: Some(qid.clone()),
                                            response: response_text.clone(),
                                        });

                                        // Enviar para o agente via canal
                                        if let Some(ref tx) = user_response_tx {
                                            let user_resp = UserResponse::to_question(qid, response_text);
                                            let _ = tx.try_send(user_resp);
                                        }

                                        app.input_text.clear();
                                        app.cursor_pos = 0;
                                    }
                                }
                                KeyCode::Char(c) => app.input_char(c),
                                KeyCode::Backspace => app.input_backspace(),
                                KeyCode::Delete => app.input_delete(),
                                KeyCode::Left => app.cursor_left(),
                                KeyCode::Right => app.cursor_right(),
                                KeyCode::Home => app.cursor_home(),
                                KeyCode::End => app.cursor_end(),
                                KeyCode::Esc => {
                                    // Cancelar - voltar para pesquisa sem resposta
                                    // Enviar resposta vazia para desbloquear o agente
                                    if let Some(ref tx) = user_response_tx {
                                        let qid = question_id.clone();
                                        let user_resp = UserResponse::to_question(qid, "[cancelado]");
                                        let _ = tx.try_send(user_resp);
                                    }
                                    app.screen = AppScreen::Research;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restaurar terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Aguardar agente terminar se houver
    if let Some(task) = agent_task {
        if let Ok(result) = task.await {
            // Mostrar resultado no terminal após sair da TUI
            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!(" RESULTADO");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!();

            if result.success {
                if let Some(answer) = &result.answer {
                    println!("✅ {}", answer);
                    println!();
                }
                if !result.references.is_empty() {
                    println!("📚 Referências:");
                    for (i, r) in result.references.iter().take(5).enumerate() {
                        println!("   {}. {} - {}", i + 1, r.title, r.url);
                    }
                }
            } else {
                println!("❌ Erro: {}", result.error.unwrap_or_default());
            }

            println!();
            println!(
                "⏱️  {:.2}s │ 🎫 {} tokens │ 🔗 {} URLs",
                result.total_time_ms as f64 / 1000.0,
                result.token_usage.total_tokens,
                result.visited_urls.len()
            );
            println!();
        }
    }

    Ok(())
}

/// Spawna tarefa de pesquisa
fn spawn_research_task(
    question: String,
    openai_key: String,
    jina_key: String,
    tx: std::sync::mpsc::Sender<deep_research::tui::AppEvent>,
    mut user_response_rx: tokio::sync::mpsc::Receiver<deep_research::agent::UserResponse>,
) -> tokio::task::JoinHandle<deep_research::agent::ResearchResult> {
    use deep_research::agent::AgentProgress;
    use deep_research::tui::{AppEvent, LogEntry, LogLevel};

    // Obter preferência de WebReader da configuração global
    let webreader_pref = get_runtime_config().webreader;

    // Clonar configuração do LLM para mover para a task
    let llm_config = get_llm_config().clone();

    tokio::spawn(async move {
        // Criar cliente LLM com configuração do .env
        let llm_client: Arc<dyn deep_research::llm::LlmClient> =
            Arc::new(OpenAiClient::from_config(openai_key, &llm_config));
        let search_client: Arc<dyn deep_research::search::SearchClient> =
            Arc::new(JinaClient::with_preference(jina_key, webreader_pref));

        // Criar callback para enviar eventos em tempo real para a TUI
        let tx_clone = tx.clone();
        let progress_callback = Arc::new(move |event: AgentProgress| {
            use deep_research::tui::PersonaStats;

            let app_event = match event {
                AgentProgress::Info(msg) => AppEvent::Log(LogEntry::new(LogLevel::Info, msg)),
                AgentProgress::Success(msg) => AppEvent::Log(LogEntry::new(LogLevel::Success, msg)),
                AgentProgress::Warning(msg) => AppEvent::Log(LogEntry::new(LogLevel::Warning, msg)),
                AgentProgress::Error(msg) => AppEvent::Log(LogEntry::new(LogLevel::Error, msg)),
                AgentProgress::Step(step) => AppEvent::SetStep(step),
                AgentProgress::Action(action) => AppEvent::SetAction(action),
                AgentProgress::Think(think) => AppEvent::SetThink(think),
                AgentProgress::Urls(total, visited) => {
                    let _ = tx_clone.send(AppEvent::SetUrlCount(total));
                    AppEvent::SetVisitedCount(visited)
                }
                AgentProgress::Tokens(tokens) => AppEvent::SetTokens(tokens),
                AgentProgress::Persona { name, searches, reads, answers, tokens, is_active } => {
                    AppEvent::UpdatePersona(PersonaStats {
                        name,
                        searches,
                        reads,
                        answers,
                        tokens,
                        is_active,
                    })
                }
                AgentProgress::VisitedUrl(url) => AppEvent::AddVisitedUrl(url),
                AgentProgress::BatchStart { batch_id, batch_type, task_count } => {
                    AppEvent::StartBatch { batch_id, batch_type, task_count }
                }
                AgentProgress::TaskUpdate {
                    task_id, batch_id, task_type, description,
                    data_info, status, elapsed_ms, thread_id,
                    progress, read_method, bytes_processed, bytes_total
                } => {
                    use deep_research::tui::{ParallelTask, TaskStatus, ReadMethod};
                    let task_status = if status == "pending" {
                        TaskStatus::Pending
                    } else if status == "running" {
                        TaskStatus::Running
                    } else if status == "completed" {
                        TaskStatus::Completed
                    } else if status.starts_with("failed:") {
                        TaskStatus::Failed(status.replace("failed:", ""))
                    } else {
                        TaskStatus::Running
                    };
                    let method = match read_method.as_str() {
                        "jina" => ReadMethod::Jina,
                        "rust_local" => ReadMethod::RustLocal,
                        "file" => ReadMethod::FileRead,
                        _ => ReadMethod::Unknown,
                    };
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    AppEvent::UpdateTask(ParallelTask {
                        id: task_id,
                        batch_id,
                        task_type,
                        description,
                        data_info,
                        status: task_status,
                        started_at: now,
                        elapsed_ms,
                        thread_id,
                        progress,
                        read_method: method,
                        bytes_processed,
                        bytes_total,
                    })
                }
                AgentProgress::BatchEnd { batch_id, total_ms, success_count, fail_count } => {
                    AppEvent::EndBatch { batch_id, total_ms, success_count, fail_count }
                }
                // Novos eventos de personas e deduplicação
                AgentProgress::PersonaQuery { persona, original, expanded, weight } => {
                    AppEvent::Log(LogEntry::new(
                        LogLevel::Info,
                        format!("🎭 [{}] {:.30}... → {:.50}... (w:{:.1})",
                            persona, original, expanded, weight)
                    ))
                }
                AgentProgress::Dedup { original_count, unique_count, removed_count, threshold } => {
                    if removed_count > 0 {
                        AppEvent::Log(LogEntry::new(
                            LogLevel::Warning,
                            format!("🔄 Dedup: {} → {} queries ({} duplicadas, thresh:{:.2})",
                                original_count, unique_count, removed_count, threshold)
                        ))
                    } else {
                        AppEvent::Log(LogEntry::new(
                            LogLevel::Info,
                            format!("🔄 Dedup: {} queries únicas (0 duplicadas)",
                                unique_count)
                        ))
                    }
                }
                // Eventos de validação fast-fail
                AgentProgress::ValidationStart { eval_types } => {
                    AppEvent::Log(LogEntry::new(
                        LogLevel::Info,
                        format!("🔍 Validação Fast-Fail iniciada: [{}]",
                            eval_types.join(" → "))
                    ))
                }
                AgentProgress::ValidationStep { eval_type, passed, confidence, reasoning, duration_ms } => {
                    let icon = if passed { "✅" } else { "❌" };
                    let level = if passed { LogLevel::Success } else { LogLevel::Warning };
                    AppEvent::Log(LogEntry::new(
                        level,
                        format!("{} {}: {:.0}% conf | {}ms | {:.40}...",
                            icon, eval_type, confidence * 100.0, duration_ms, reasoning)
                    ))
                }
                AgentProgress::ValidationEnd { overall_passed, failed_at, total_evals, passed_evals } => {
                    let (icon, msg, level) = if overall_passed {
                        ("✅", format!("Validação APROVADA: {}/{} etapas", passed_evals, total_evals), LogLevel::Success)
                    } else {
                        let fail_point = failed_at.unwrap_or("?".into());
                        ("❌", format!("Validação REPROVADA em {}: {}/{} etapas", fail_point, passed_evals, total_evals), LogLevel::Error)
                    };
                    AppEvent::Log(LogEntry::new(level, format!("{} {}", icon, msg)))
                }
                // Eventos do AgentAnalyzer (análise de erros em background)
                AgentProgress::AgentAnalysisStarted { failures_count, diary_entries } => {
                    // Enviar evento específico para o AgentAnalyzer
                    let _ = tx_clone.send(AppEvent::AgentAnalyzerStarted {
                        failures_count,
                        diary_entries,
                    });
                    // Também enviar log geral
                    AppEvent::Log(LogEntry::new(
                        LogLevel::Info,
                        format!("🔬 AgentAnalyzer: Analisando {} falhas ({} entradas)...",
                            failures_count, diary_entries)
                    ))
                }
                AgentProgress::AgentAnalysisCompleted { recap, blame, improvement, duration_ms } => {
                    // Enviar evento específico para o AgentAnalyzer
                    let _ = tx_clone.send(AppEvent::AgentAnalyzerCompleted {
                        recap: recap.clone(),
                        blame: blame.clone(),
                        improvement: improvement.clone(),
                        duration_ms,
                    });
                    // Também enviar log geral resumido
                    AppEvent::Log(LogEntry::new(
                        LogLevel::Success,
                        format!("🔬 AgentAnalyzer concluído ({}ms) - Melhoria aplicada ao prompt",
                            duration_ms)
                    ))
                }
                // Eventos de interação com usuário
                AgentProgress::AgentQuestion { question_id, question_type, question, options, is_blocking } => {
                    // Enviar evento para a TUI mostrar a pergunta
                    let _ = tx_clone.send(AppEvent::AgentQuestion {
                        question_id: question_id.clone(),
                        question_type: question_type.clone(),
                        question: question.clone(),
                        options: options.clone(),
                        is_blocking,
                    });
                    let icon = if is_blocking { "⏸️" } else { "💬" };
                    AppEvent::Log(LogEntry::new(
                        LogLevel::Info,
                        format!("{} [{}] Pergunta ao usuário: {:.50}...",
                            icon, question_type, question)
                    ))
                }
                AgentProgress::UserResponseReceived { question_id, response, was_spontaneous } => {
                    let icon = if was_spontaneous { "💬" } else { "✅" };
                    let msg = if let Some(qid) = question_id {
                        format!("{} Resposta recebida ({}): {:.50}...", icon, qid, response)
                    } else {
                        format!("{} Mensagem do usuário: {:.50}...", icon, response)
                    };
                    AppEvent::Log(LogEntry::new(LogLevel::Success, msg))
                }
                AgentProgress::ResumedAfterInput { question_id } => {
                    AppEvent::Log(LogEntry::new(
                        LogLevel::Info,
                        format!("▶️ Retomando execução após resposta ({})", question_id)
                    ))
                }
                // Eventos de Sandbox (execução de código)
                AgentProgress::SandboxStart { problem, max_attempts, timeout_ms, language } => {
                    let _ = tx_clone.send(AppEvent::SandboxStart {
                        problem: problem.clone(),
                        max_attempts,
                        timeout_ms,
                        language: language.clone(),
                    });
                    let lang_emoji = if language == "Python" { "🐍" } else { "📜" };
                    AppEvent::Log(LogEntry::new(
                        LogLevel::Info,
                        format!("{} Sandbox {} iniciado: {} (max {} tentativas, {}ms timeout)",
                            lang_emoji, language,
                            if problem.len() > 50 { format!("{}...", &problem[..50]) } else { problem },
                            max_attempts, timeout_ms)
                    ))
                }
                AgentProgress::SandboxAttempt { attempt, max_attempts, code_preview, status, error } => {
                    let _ = tx_clone.send(AppEvent::SandboxAttempt {
                        attempt,
                        max_attempts,
                        code_preview: code_preview.clone(),
                        status: status.clone(),
                        error: error.clone(),
                    });
                    let msg = if let Some(e) = error {
                        format!("🔄 Sandbox tentativa {}/{}: {} - {}", attempt, max_attempts, status, e)
                    } else {
                        format!("🔄 Sandbox tentativa {}/{}: {}", attempt, max_attempts, status)
                    };
                    AppEvent::Log(LogEntry::new(LogLevel::Info, msg))
                }
                AgentProgress::SandboxComplete { success, output, error, attempts, execution_time_ms, code_preview, language } => {
                    let _ = tx_clone.send(AppEvent::SandboxComplete {
                        success,
                        output: output.clone(),
                        error: error.clone(),
                        attempts,
                        execution_time_ms,
                        code_preview: code_preview.clone(),
                        language: language.clone(),
                    });
                    let lang_emoji = if language == "Python" { "🐍" } else { "📜" };
                    if success {
                        let out_preview = output.as_ref()
                            .map(|o| if o.len() > 80 { format!("{}...", &o[..80]) } else { o.clone() })
                            .unwrap_or_default();
                        AppEvent::Log(LogEntry::new(
                            LogLevel::Success,
                            format!("{} {} concluído: {} tentativas, {}ms → {}",
                                lang_emoji, language, attempts, execution_time_ms, out_preview)
                        ))
                    } else {
                        let err = error.unwrap_or_else(|| "Unknown error".into());
                        AppEvent::Log(LogEntry::new(
                            LogLevel::Error,
                            format!("{} {} falhou após {} tentativas, {}ms: {}",
                                lang_emoji, language, attempts, execution_time_ms, err)
                        ))
                    }
                }
            };
            let _ = tx_clone.send(app_event);
        });

        // Criar agente com callback de progresso e canais de interação
        let (agent, response_tx, _question_rx) = DeepResearchAgent::new(llm_client, search_client, None)
            .with_progress_callback(progress_callback)
            .with_interaction_channels(16);

        // Spawn task para receber respostas do usuário da TUI e enviar para o agente
        let tx_for_bridge = tx.clone();
        tokio::spawn(async move {
            while let Some(user_resp) = user_response_rx.recv().await {
                // Enviar resposta para o InteractionHub do agente
                if response_tx.send(user_resp.clone()).await.is_err() {
                    // Canal fechado, agente terminou
                    break;
                }
                // Log de debug
                let _ = tx_for_bridge.send(AppEvent::Log(LogEntry::new(
                    LogLevel::Info,
                    format!("📨 Resposta enviada ao agente: {:.30}...", user_resp.content)
                )));
            }
        });

        let result = agent.run(question).await;

        // Enviar estatísticas finais detalhadas
        let _ = tx.send(AppEvent::Log(LogEntry::new(
            LogLevel::Info,
            format!(
                "📊 Estatísticas: {} steps | {} URLs visitadas | {} tokens",
                result.visited_urls.len(),
                result.visited_urls.len(),
                result.token_usage.total_tokens
            ),
        )));
        let _ = tx.send(AppEvent::Log(LogEntry::new(
            LogLevel::Info,
            format!(
                "⏱️ Tempo: {:.1}s total | {:.1}s busca | {:.1}s leitura | {:.1}s LLM",
                result.total_time_ms as f64 / 1000.0,
                result.search_time_ms as f64 / 1000.0,
                result.read_time_ms as f64 / 1000.0,
                result.llm_time_ms as f64 / 1000.0
            ),
        )));
        let _ = tx.send(AppEvent::Log(LogEntry::new(
            LogLevel::Info,
            format!(
                "🎟️ Tokens: {} prompt + {} completion = {} total",
                result.token_usage.prompt_tokens,
                result.token_usage.completion_tokens,
                result.token_usage.total_tokens
            ),
        )));

        let _ = tx.send(AppEvent::SetVisitedCount(result.visited_urls.len()));
        let _ = tx.send(AppEvent::SetTokens(result.token_usage.total_tokens));

        // Enviar tempos detalhados
        let _ = tx.send(AppEvent::SetTimes {
            total_ms: result.total_time_ms,
            search_ms: result.search_time_ms,
            read_ms: result.read_time_ms,
            llm_ms: result.llm_time_ms,
        });

        // Enviar resultado
        if result.success {
            if let Some(ref answer) = result.answer {
                let _ = tx.send(AppEvent::Log(LogEntry::new(
                    LogLevel::Success,
                    format!("✅ Resposta gerada ({} chars, {} referências)", answer.len(), result.references.len()),
                )));
                let refs: Vec<String> = result
                    .references
                    .iter()
                    .map(|r| {
                        // Incluir score de relevância se disponível (referência semântica)
                        if let Some(score) = r.relevance_score {
                            format!("[{:.0}%] {} - {}", score * 100.0, r.title, r.url)
                        } else {
                            format!("{} - {}", r.title, r.url)
                        }
                    })
                    .collect();
                let _ = tx.send(AppEvent::SetAnswer(answer.clone()));
                let _ = tx.send(AppEvent::SetReferences(refs));
            }
            let _ = tx.send(AppEvent::Complete);
        } else {
            let _ = tx.send(AppEvent::Error(
                result.error.clone().unwrap_or_else(|| "Erro desconhecido".into()),
            ));
        }

        result
    })
}

/// Conta threads reais do processo atual
fn count_process_threads() -> usize {
    #[cfg(target_os = "macos")]
    {
        // No macOS, usar mach API ou contar via sysctl
        use std::process::Command;
        Command::new("ps")
            .args(["-M", "-p", &std::process::id().to_string()])
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Cada linha após o header é uma thread
                Some(stdout.lines().count().saturating_sub(1).max(1))
            })
            .unwrap_or(1)
    }
    #[cfg(target_os = "linux")]
    {
        // No Linux, contar diretórios em /proc/self/task
        std::fs::read_dir("/proc/self/task")
            .map(|entries| entries.count())
            .unwrap_or(1)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    }
}

/// Obtém uso de memória do processo atual em MB
fn get_process_memory_mb() -> f64 {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        // Usar ps para obter RSS (Resident Set Size)
        Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.trim().parse::<f64>().ok()
            })
            .map(|kb| kb / 1024.0) // Converter KB para MB
            .unwrap_or(0.0)
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/statm")
            .ok()
            .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
            .map(|pages| (pages * 4096) as f64 / 1024.0 / 1024.0)
            .unwrap_or(0.0)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        0.0
    }
}

/// Atualiza métricas do sistema
fn update_system_metrics(app: &mut deep_research::tui::App) {
    use deep_research::tui::SystemMetrics;

    // Contar threads reais do processo
    let threads = count_process_threads();

    // Memória real do processo
    let memory_mb = get_process_memory_mb();

    // Contar tarefas ativas nos batches como indicador de carga
    let active_tasks: usize = app.active_batches.values()
        .flat_map(|batch| batch.tasks.iter())
        .filter(|t| matches!(t.status, deep_research::tui::TaskStatus::Running))
        .count();

    app.metrics = SystemMetrics {
        threads: threads + active_tasks, // Threads do processo + tarefas tokio ativas
        memory_mb,
        cpu_percent: 0.0,
    };
}
