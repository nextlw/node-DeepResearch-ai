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

/// Obtém a configuração do runtime (thread-safe)
fn get_runtime_config() -> &'static RuntimeConfig {
    RUNTIME_CONFIG.get().expect("Runtime config not initialized")
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

    // Log da configuração efetiva
    let effective_threads = config.effective_worker_threads();
    if !is_tui_mode {
        log::info!(
            "🚀 Runtime: {} threads (max: {}) | WebReader: {}",
            effective_threads,
            config.max_threads,
            config.webreader
        );
    }

    // Armazenar configuração globalmente para acesso em outras funções (thread-safe)
    RUNTIME_CONFIG.set(config.clone()).expect("Runtime config already initialized");

    // Instalar panic hook customizado (isolamento de threads)
    install_panic_hook();

    // Criar runtime Tokio com configuração customizada
    let runtime = create_tokio_runtime(&config)?;

    // Executar main async dentro do runtime customizado
    runtime.block_on(async_main(args, is_tui_mode))
}

async fn async_main(args: Vec<String>, is_tui_mode: bool) -> anyhow::Result<()> {

    if args.len() < 2 {
        eprintln!("Deep Research CLI v{}", deep_research::VERSION);
        eprintln!();
        eprintln!("Uso: {} <pergunta>", args[0]);
        eprintln!();
        eprintln!("Opções:");
        eprintln!("  --tui [pergunta]      Modo TUI interativo (com campo de texto)");
        eprintln!("  --budget <tokens>     Budget máximo de tokens (padrão: 1000000)");
        eprintln!(
            "  --compare <urls>      Comparar Jina Reader vs Rust+OpenAI (URLs separadas por vírgula)"
        );
        eprintln!("  --compare-live        Habilita comparação Jina vs Rust durante pesquisa");
        eprintln!();
        eprintln!("Exemplos:");
        eprintln!("  {} \"Qual é a população do Brasil?\"", args[0]);
        eprintln!("  {} --tui                              # Abre interface para digitar", args[0]);
        eprintln!("  {} --tui \"Qual é a capital da França?\"", args[0]);
        eprintln!(
            "  {} --compare \"https://example.com,https://rust-lang.org\"",
            args[0]
        );
        eprintln!(
            "  {} --compare-live \"pergunta\"             # Pesquisa com comparação Jina/Rust",
            args[0]
        );
        std::process::exit(1);
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

    let llm_client: Arc<dyn deep_research::llm::LlmClient> =
        Arc::new(OpenAiClient::new(openai_key));

    // Usar preferência de WebReader da configuração global
    let webreader_pref = get_runtime_config().webreader;
    let search_client: Arc<dyn deep_research::search::SearchClient> =
        Arc::new(JinaClient::with_preference(jina_key, webreader_pref));

    // Criar e executar agente
    let agent = DeepResearchAgent::new(llm_client, search_client, budget)
        .with_comparative_read(enable_compare_live);

    println!("Iniciando pesquisa...");
    println!();

    let result = agent.run(question).await;

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

/// Executa o modo TUI interativo
async fn run_tui_mode(question: &str) -> anyhow::Result<()> {
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use deep_research::tui::{App, AppScreen};
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

    // Canal para eventos
    let (tx, rx) = create_event_channel();

    // Handle de tarefa do agente (opcional)
    let mut agent_task: Option<tokio::task::JoinHandle<_>> = None;

    // Se já tem pergunta, iniciar pesquisa
    if !question.is_empty() {
        agent_task = Some(spawn_research_task(
            question.to_string(),
            openai_key.clone(),
            jina_key.clone(),
            tx.clone(),
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
                                        agent_task = Some(spawn_research_task(
                                            q,
                                            openai_key.clone(),
                                            jina_key.clone(),
                                            tx.clone(),
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
                                KeyCode::Up => app.history_up(),
                                KeyCode::Down => app.history_down(),
                                KeyCode::Esc => {
                                    app.should_quit = true;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        AppScreen::Research => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.should_quit = true;
                                    break;
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
                        AppScreen::Result => {
                            match key.code {
                                KeyCode::Enter => {
                                    app.reset();
                                }
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.should_quit = true;
                                    break;
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
) -> tokio::task::JoinHandle<deep_research::agent::ResearchResult> {
    use deep_research::agent::AgentProgress;
    use deep_research::tui::{AppEvent, LogEntry, LogLevel};

    // Obter preferência de WebReader da configuração global
    let webreader_pref = get_runtime_config().webreader;

    tokio::spawn(async move {
        let llm_client: Arc<dyn deep_research::llm::LlmClient> =
            Arc::new(OpenAiClient::new(openai_key));
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
            };
            let _ = tx_clone.send(app_event);
        });

        // Criar agente com callback de progresso
        let agent = DeepResearchAgent::new(llm_client, search_client, None)
            .with_progress_callback(progress_callback);

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
                    .map(|r| format!("{} - {}", r.title, r.url))
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
