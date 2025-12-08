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

use deep_research::llm::OpenAiClient;
use deep_research::prelude::*;
use deep_research::reader_comparison::ReaderComparison;
use deep_research::search::JinaClient;
use deep_research::tui::{create_event_channel, run_tui, AppEvent, TuiLogger};
use std::path::PathBuf;
use std::sync::Arc;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Carregar .env PRIMEIRO, antes de qualquer coisa
    load_dotenv();

    // Inicializar logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Parse argumentos
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Deep Research CLI v{}", deep_research::VERSION);
        eprintln!();
        eprintln!("Uso: {} <pergunta>", args[0]);
        eprintln!();
        eprintln!("Opções:");
        eprintln!("  --tui              Modo TUI interativo (interface rica)");
        eprintln!("  --budget <tokens>  Budget máximo de tokens (padrão: 1000000)");
        eprintln!(
            "  --compare <urls>   Comparar Jina Reader vs Rust+OpenAI (URLs separadas por vírgula)"
        );
        eprintln!();
        eprintln!("Exemplos:");
        eprintln!("  {} \"Qual é a população do Brasil em 2024?\"", args[0]);
        eprintln!("  {} --tui \"Qual é a capital da França?\"", args[0]);
        eprintln!(
            "  {} --compare \"https://example.com,https://rust-lang.org\"",
            args[0]
        );
        std::process::exit(1);
    }

    // Modo TUI
    if args.len() >= 3 && args[1] == "--tui" {
        return run_tui_mode(&args[2..].join(" ")).await;
    }

    // Modo comparação
    if args.len() >= 3 && args[1] == "--compare" {
        return run_comparison_mode(&args[2]).await;
    }

    // Parse budget se fornecido
    let (budget, question) = if args.len() >= 4 && args[1] == "--budget" {
        let budget: u64 = args[2].parse().unwrap_or(1_000_000);
        let question = args[3..].join(" ");
        (Some(budget), question)
    } else {
        (None, args[1..].join(" "))
    };

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" DEEP RESEARCH v{}", deep_research::VERSION);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Pergunta: {}", question);
    if let Some(b) = budget {
        println!("Budget: {} tokens", b);
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
    let search_client: Arc<dyn deep_research::search::SearchClient> =
        Arc::new(JinaClient::new(jina_key));

    // Criar e executar agente
    let agent = DeepResearchAgent::new(llm_client, search_client, budget);

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
    // Criar clientes
    let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: OPENAI_API_KEY não encontrada!");
        std::process::exit(1);
    });

    let jina_key = std::env::var("JINA_API_KEY").unwrap_or_else(|_| {
        eprintln!("✗ Erro: JINA_API_KEY não encontrada!");
        std::process::exit(1);
    });

    let llm_client: Arc<dyn deep_research::llm::LlmClient> =
        Arc::new(OpenAiClient::new(openai_key));
    let search_client: Arc<dyn deep_research::search::SearchClient> =
        Arc::new(JinaClient::new(jina_key));

    // Criar canal de eventos para TUI
    let (tx, rx) = create_event_channel();
    let logger = TuiLogger::new(tx.clone());

    // Executar agente em thread separada
    let question_clone = question.to_string();
    let agent_handle = tokio::spawn(async move {
        // Criar agente
        let agent = DeepResearchAgent::new(llm_client, search_client, None);

        logger.info("Iniciando pesquisa...");
        logger.set_action("Inicializando");

        // Executar pesquisa
        let result = agent.run(question_clone).await;

        // Enviar resultado para TUI
        if result.success {
            if let Some(ref answer) = result.answer {
                let refs: Vec<String> = result.references
                    .iter()
                    .map(|r| format!("{} - {}", r.title, r.url))
                    .collect();
                logger.complete(answer.clone(), refs);
            }
        } else {
            let _ = tx.send(AppEvent::Error(
                result.error.clone().unwrap_or_else(|| "Erro desconhecido".into())
            ));
        }

        result
    });

    // Executar TUI (bloqueia até terminar)
    let app = run_tui(question.to_string(), rx)?;

    // Aguardar agente terminar
    let result = agent_handle.await?;

    // Se o usuário saiu antes, mostrar resultado no terminal
    if app.should_quit && !app.is_complete {
        println!("\n⚠️  TUI encerrada pelo usuário");
        if let Some(answer) = result.answer {
            println!("\nResposta parcial: {}", answer);
        }
    }

    Ok(())
}
