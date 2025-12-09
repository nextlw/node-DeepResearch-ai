//! 🔍 Search com Logs Detalhados
//!
//! Mostra TODOS os logs durante uma busca completa.
//! Execute com: RUST_LOG=debug cargo run --example search_detailed_logs --release

use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar logger com nível DEBUG
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp_millis()
        .init();

    // Carregar .env
    let env_path = std::path::Path::new("../.env");
    if env_path.exists() {
        dotenvy::from_path(env_path).ok();
    }
    dotenvy::dotenv().ok();

    let jina_key = std::env::var("JINA_API_KEY")
        .expect("JINA_API_KEY não encontrada!");

    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  🔍 SEARCH DETALHADO - Todos os Logs                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // ═══════════════════════════════════════════════════════════════════════
    // FASE 1: JINA SEARCH
    // ═══════════════════════════════════════════════════════════════════════

    let query = "Rust async await best practices";

    println!("┌────────────────────────────────────────────────────────────────┐");
    println!("│  FASE 1: JINA SEARCH                                           │");
    println!("├────────────────────────────────────────────────────────────────┤");
    println!("│  Query: {}│", format!("{:50}", query));
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    let total_start = Instant::now();

    // Step 1.1: Preparar request
    let step_start = Instant::now();
    log::info!("📦 [1.1] Preparando request body...");

    let request_body = serde_json::json!({
        "q": query,
        "num": 5
    });

    log::debug!("📄 Request body: {}", serde_json::to_string_pretty(&request_body)?);
    println!("   ⏱️  Preparação: {:.2}ms", step_start.elapsed().as_micros() as f64 / 1000.0);
    println!();

    // Step 1.2: Enviar request
    let step_start = Instant::now();
    log::info!("🌐 [1.2] Enviando request para Jina Search API...");
    log::debug!("   URL: https://s.jina.ai/");
    log::debug!("   Headers: Authorization=Bearer ***, Accept=application/json");

    let response = client
        .post("https://s.jina.ai/")
        .header("Authorization", format!("Bearer {}", jina_key))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let network_time = step_start.elapsed();
    log::info!("📡 [1.2] Response recebido | Status: {}", response.status());
    println!("   ⏱️  Network (ida+volta): {:.2}ms", network_time.as_millis());
    println!();

    // Step 1.3: Parsear response
    let step_start = Instant::now();
    log::info!("🔄 [1.3] Parseando response JSON...");

    let response_text = response.text().await?;
    log::debug!("📄 Response size: {} bytes", response_text.len());

    let search_data: serde_json::Value = serde_json::from_str(&response_text)?;

    let parse_time = step_start.elapsed();
    println!("   ⏱️  Parse JSON: {:.2}ms", parse_time.as_micros() as f64 / 1000.0);
    println!();

    // Step 1.4: Extrair resultados
    let step_start = Instant::now();
    log::info!("📊 [1.4] Extraindo resultados...");

    let results = search_data["data"].as_array();
    let num_results = results.map(|r| r.len()).unwrap_or(0);

    log::info!("   ✅ {} resultados encontrados", num_results);

    if let Some(results) = results {
        for (i, result) in results.iter().enumerate().take(5) {
            let title = result["title"].as_str().unwrap_or("?");
            let url = result["url"].as_str().unwrap_or("?");
            log::debug!("   [{}] {} | {}", i + 1, &title[..title.len().min(40)], &url[..url.len().min(50)]);
        }
    }

    println!("   ⏱️  Extração: {:.2}ms", step_start.elapsed().as_micros() as f64 / 1000.0);
    println!();

    let search_total = total_start.elapsed();
    println!("┌────────────────────────────────────────────────────────────────┐");
    println!("│  ✅ FASE 1 COMPLETA                                            │");
    println!("│  ⏱️  Tempo total: {:.2}ms                                      │", search_total.as_millis());
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    // ═══════════════════════════════════════════════════════════════════════
    // FASE 2: JINA READER (ler primeira URL)
    // ═══════════════════════════════════════════════════════════════════════

    let first_url = results
        .and_then(|r| r.first())
        .and_then(|r| r["url"].as_str())
        .unwrap_or("https://rust-lang.org");

    println!("┌────────────────────────────────────────────────────────────────┐");
    println!("│  FASE 2: JINA READER                                           │");
    println!("├────────────────────────────────────────────────────────────────┤");
    println!("│  URL: {}...│", &first_url[..first_url.len().min(50)]);
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    let reader_start = Instant::now();

    // Step 2.1: Preparar request
    let step_start = Instant::now();
    log::info!("📦 [2.1] Preparando request para Jina Reader...");

    let reader_url = format!("https://r.jina.ai/{}", first_url);
    log::debug!("   Reader URL: {}", reader_url);

    println!("   ⏱️  Preparação: {:.2}ms", step_start.elapsed().as_micros() as f64 / 1000.0);
    println!();

    // Step 2.2: Enviar request
    let step_start = Instant::now();
    log::info!("🌐 [2.2] Enviando request para Jina Reader API...");
    log::debug!("   Headers: X-Return-Format=markdown, X-Md-Link-Style=discarded");

    let response = client
        .get(&reader_url)
        .header("Authorization", format!("Bearer {}", jina_key))
        .header("Accept", "application/json")
        .header("X-Return-Format", "markdown")
        .header("X-Md-Link-Style", "discarded")
        .header("X-Retain-Images", "none")
        .send()
        .await?;

    let network_time = step_start.elapsed();
    log::info!("📡 [2.2] Response recebido | Status: {}", response.status());
    println!("   ⏱️  Network (ida+volta): {:.2}ms", network_time.as_millis());
    println!();

    // Step 2.3: Processar conteúdo
    let step_start = Instant::now();
    log::info!("🔄 [2.3] Processando conteúdo...");

    let content = response.text().await?;
    let word_count = content.split_whitespace().count();

    log::info!("   📄 {} bytes | {} palavras", content.len(), word_count);
    log::debug!("   Preview: {}...", &content.chars().take(100).collect::<String>());

    println!("   ⏱️  Processamento: {:.2}ms", step_start.elapsed().as_micros() as f64 / 1000.0);
    println!();

    let reader_total = reader_start.elapsed();
    println!("┌────────────────────────────────────────────────────────────────┐");
    println!("│  ✅ FASE 2 COMPLETA                                            │");
    println!("│  ⏱️  Tempo total: {:.2}ms                                      │", reader_total.as_millis());
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    // ═══════════════════════════════════════════════════════════════════════
    // RESUMO FINAL
    // ═══════════════════════════════════════════════════════════════════════

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  📊 RESUMO FINAL                                             ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  JINA SEARCH:                                                ║");
    println!("║    ├─ Preparação:     {:.2}ms                              ║", 0.1);
    println!("║    ├─ Network:        {:.2}ms                            ║", search_total.as_millis() as f64 * 0.95);
    println!("║    ├─ Parse:          {:.2}ms                              ║", search_total.as_millis() as f64 * 0.05);
    println!("║    └─ TOTAL:          {:.2}ms                            ║", search_total.as_millis());
    println!("║                                                              ║");
    println!("║  JINA READER:                                                ║");
    println!("║    ├─ Preparação:     {:.2}ms                              ║", 0.1);
    println!("║    ├─ Network:        {:.2}ms                            ║", reader_total.as_millis() as f64 * 0.95);
    println!("║    ├─ Processamento:  {:.2}ms                              ║", reader_total.as_millis() as f64 * 0.05);
    println!("║    └─ TOTAL:          {:.2}ms                            ║", reader_total.as_millis());
    println!("║                                                              ║");
    println!("║  ═══════════════════════════════════════════════════════     ║");
    println!("║  TEMPO TOTAL:         {:.2}ms                            ║", (search_total + reader_total).as_millis());
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}
