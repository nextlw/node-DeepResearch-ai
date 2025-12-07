// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEEP RESEARCH CLI
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// CLI para execução do agente de pesquisa profunda.
//
// Uso:
//   deep-research-cli "Qual é a população do Brasil?"
//   deep-research-cli --budget 500000 "pergunta complexa"
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::sync::Arc;
use deep_research::prelude::*;
use deep_research::llm::MockLlmClient;
use deep_research::search::MockSearchClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Inicializar logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    // Parse argumentos
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Deep Research CLI v{}", deep_research::VERSION);
        eprintln!();
        eprintln!("Uso: {} <pergunta>", args[0]);
        eprintln!();
        eprintln!("Opções:");
        eprintln!("  --budget <tokens>  Budget máximo de tokens (padrão: 1000000)");
        eprintln!();
        eprintln!("Exemplo:");
        eprintln!("  {} \"Qual é a população do Brasil em 2024?\"", args[0]);
        std::process::exit(1);
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

    // Criar clientes (usar mocks por enquanto)
    // TODO: Implementar clientes reais com API keys
    let llm_client: Arc<dyn deep_research::llm::LlmClient> =
        Arc::new(MockLlmClient::new());
    let search_client: Arc<dyn deep_research::search::SearchClient> =
        Arc::new(MockSearchClient::new());

    // Criar e executar agente
    let agent = DeepResearchAgent::new(
        llm_client,
        search_client,
        budget,
    );

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
    println!("Tokens utilizados:");
    println!("  Prompt:     {}", result.token_usage.prompt_tokens);
    println!("  Completion: {}", result.token_usage.completion_tokens);
    println!("  Total:      {}", result.token_usage.total_tokens);
    println!();
    println!("URLs visitadas: {}", result.visited_urls.len());
    for url in &result.visited_urls {
        println!("  - {}", url);
    }
    println!();

    Ok(())
}
