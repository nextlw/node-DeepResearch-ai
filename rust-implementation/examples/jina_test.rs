//! Teste simples de Jina API
//!
//! Execute com: cargo run --example jina_test --release

use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Carregar .env da raiz
    let env_path = std::path::Path::new("../.env");
    if env_path.exists() {
        dotenvy::from_path(env_path).ok();
    }
    dotenvy::dotenv().ok();

    let jina_key = std::env::var("JINA_API_KEY")
        .expect("JINA_API_KEY não encontrada! Configure no .env");

    println!("\n🧪 Teste Jina API - DeepResearch AI\n");
    println!("{}", "=".repeat(60));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // URLs para testar
    let urls = vec![
        "https://www.rust-lang.org/",
        "https://docs.rs/",
    ];

    println!("\n📖 Testando Jina Reader API...\n");

    for url in &urls {
        println!("🔗 URL: {}", url);

        let start = Instant::now();

        let response = client
            .get(format!("https://r.jina.ai/{}", url))
            .header("Authorization", format!("Bearer {}", jina_key))
            .header("Accept", "application/json")
            .header("X-Return-Format", "markdown")
            .send()
            .await?;

        let elapsed = start.elapsed();
        let status = response.status();

        if status.is_success() {
            let text = response.text().await?;
            let word_count = text.split_whitespace().count();

            println!("   ✅ Status: {}", status);
            println!("   ⏱️  Tempo: {:.2}ms", elapsed.as_millis());
            println!("   📝 Palavras: {}", word_count);
            println!("   📄 Preview: {}...", &text.chars().take(100).collect::<String>());
        } else {
            let error = response.text().await?;
            println!("   ❌ Erro: {} - {}", status, error);
        }

        println!();
    }

    // Testar Jina Search
    println!("🔍 Testando Jina Search API...\n");

    let query = "Rust programming language best practices 2024";
    println!("Query: {}\n", query);

    let start = Instant::now();

    let response = client
        .get(format!("https://s.jina.ai/{}", urlencoding::encode(query)))
        .header("Authorization", format!("Bearer {}", jina_key))
        .header("Accept", "application/json")
        .send()
        .await?;

    let elapsed = start.elapsed();
    let status = response.status();

    if status.is_success() {
        let text = response.text().await?;
        println!("   ✅ Status: {}", status);
        println!("   ⏱️  Tempo: {:.2}ms", elapsed.as_millis());
        println!("   📄 Resposta: {}...", &text.chars().take(500).collect::<String>());
    } else {
        let error = response.text().await?;
        println!("   ❌ Erro: {} - {}", status, error);
    }

    println!("\n{}", "=".repeat(60));
    println!("✅ Teste concluído!");

    Ok(())
}
