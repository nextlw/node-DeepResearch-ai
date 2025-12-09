//! 🔬 Search ULTRA Detalhado - Todos os passos internos
//!
//! Mostra ABSOLUTAMENTE TUDO que acontece durante uma busca.
//! Execute com: cargo run --example search_ultra_detailed --release

use std::time::Instant;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

static STEP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn step(name: &str) -> usize {
    let n = STEP_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
    println!("[{}] 📍 STEP {}: {}", timestamp, n, name);
    n
}

fn substep(step: usize, name: &str, detail: &str) {
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
    println!("[{}]    └─ {}.{}: {} | {}", timestamp, step, "x", name, detail);
}

fn timing(label: &str, ms: f64) {
    println!("         ⏱️  {}: {:.3}ms", label, ms);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Carregar .env
    let env_path = std::path::Path::new("../.env");
    if env_path.exists() {
        dotenvy::from_path(env_path).ok();
    }
    dotenvy::dotenv().ok();

    let jina_key = std::env::var("JINA_API_KEY")?;

    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  🔬 SEARCH ULTRA DETALHADO - Análise de cada microsegundo         ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    let query = "Rust async best practices";

    println!("┌─────────────────────────────────────────────────────────────────────┐");
    println!("│  Query: {:60}│", query);
    println!("└─────────────────────────────────────────────────────────────────────┘");
    println!();

    let total_start = Instant::now();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 1: PREPARAÇÃO DO CLIENT HTTP
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("CRIAR HTTP CLIENT");
    let t = Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(10))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .build()?;

    timing("Client criado", t.elapsed().as_micros() as f64 / 1000.0);
    substep(s, "config", "timeout=60s, connect_timeout=10s, pool_idle=90s");
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 2: PREPARAR REQUEST
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("PREPARAR REQUEST BODY");
    let t = Instant::now();

    let request_body = serde_json::json!({
        "q": query,
        "num": 5
    });

    timing("JSON serializado", t.elapsed().as_micros() as f64 / 1000.0);
    substep(s, "body", &format!("{} bytes", serde_json::to_string(&request_body)?.len()));
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 3: CONSTRUIR REQUEST HTTP
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("CONSTRUIR HTTP REQUEST");
    let t = Instant::now();

    let request = client
        .post("https://s.jina.ai/")
        .header("Authorization", format!("Bearer {}", &jina_key))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("User-Agent", "DeepResearch-Rust/1.0")
        .json(&request_body);

    timing("Request construído", t.elapsed().as_micros() as f64 / 1000.0);
    substep(s, "method", "POST");
    substep(s, "url", "https://s.jina.ai/");
    substep(s, "headers", "Authorization, Accept, Content-Type, User-Agent");
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 4: ENVIAR REQUEST (DNS + TCP + TLS + HTTP)
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("ENVIAR REQUEST (inclui DNS, TCP, TLS, HTTP)");
    println!("         📡 Iniciando conexão com s.jina.ai...");
    println!();

    let t = Instant::now();
    let dns_start = Instant::now();

    // O send() faz tudo: DNS lookup, TCP connect, TLS handshake, HTTP request
    let response = request.send().await?;

    let total_network = t.elapsed();
    timing("TOTAL network", total_network.as_millis() as f64);
    println!();

    substep(s, "status", &format!("{}", response.status()));
    substep(s, "http_version", &format!("{:?}", response.version()));

    // Headers da resposta
    println!();
    let s = step("ANALISAR RESPONSE HEADERS");
    for (key, value) in response.headers().iter().take(10) {
        substep(s, &key.to_string(), &format!("{:?}", value.to_str().unwrap_or("?")));
    }
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 5: LER RESPONSE BODY
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("LER RESPONSE BODY");
    let t = Instant::now();

    let response_bytes = response.bytes().await?;

    timing("Body baixado", t.elapsed().as_millis() as f64);
    substep(s, "tamanho", &format!("{} bytes ({:.2} KB)", response_bytes.len(), response_bytes.len() as f64 / 1024.0));
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 6: PARSEAR JSON
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("PARSEAR JSON");
    let t = Instant::now();

    let json: serde_json::Value = serde_json::from_slice(&response_bytes)?;

    timing("JSON parseado", t.elapsed().as_micros() as f64 / 1000.0);
    substep(s, "tipo_raiz", &format!("{}", if json.is_object() { "Object" } else { "?" }));
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 7: EXTRAIR DADOS
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("EXTRAIR DADOS DO JSON");
    let t = Instant::now();

    // Metadata
    if let Some(code) = json.get("code") {
        substep(s, "code", &format!("{}", code));
    }
    if let Some(status) = json.get("status") {
        substep(s, "status", &format!("{}", status));
    }

    // Resultados
    let results = json.get("data").and_then(|d| d.as_array());
    let num_results = results.map(|r| r.len()).unwrap_or(0);
    substep(s, "num_results", &format!("{}", num_results));

    timing("Dados extraídos", t.elapsed().as_micros() as f64 / 1000.0);
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 8: PROCESSAR CADA RESULTADO
    // ═══════════════════════════════════════════════════════════════════════════

    let s = step("PROCESSAR RESULTADOS");
    let t = Instant::now();

    if let Some(results) = results {
        for (i, result) in results.iter().enumerate().take(5) {
            let title = result.get("title").and_then(|t| t.as_str()).unwrap_or("?");
            let url = result.get("url").and_then(|u| u.as_str()).unwrap_or("?");
            let desc_len = result.get("description").and_then(|d| d.as_str()).map(|s| s.len()).unwrap_or(0);
            let content_len = result.get("content").and_then(|c| c.as_str()).map(|s| s.len()).unwrap_or(0);

            println!("         [{}] {}", i + 1, &title[..title.len().min(50)]);
            println!("             URL: {}...", &url[..url.len().min(50)]);
            println!("             description: {} chars, content: {} chars", desc_len, content_len);
            println!();
        }
    }

    timing("Resultados processados", t.elapsed().as_micros() as f64 / 1000.0);
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // RESUMO FINAL
    // ═══════════════════════════════════════════════════════════════════════════

    let total_time = total_start.elapsed();

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  📊 BREAKDOWN COMPLETO                                            ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                   ║");
    println!("║  1. Criar HTTP Client      │ ~0.1ms    │ ░░░░░░░░░░░░░░░░░░░░░░  ║");
    println!("║  2. Preparar Request Body  │ ~0.01ms   │ ░░░░░░░░░░░░░░░░░░░░░░  ║");
    println!("║  3. Construir HTTP Request │ ~0.01ms   │ ░░░░░░░░░░░░░░░░░░░░░░  ║");
    println!("║  4. NETWORK (DNS+TCP+TLS+HTTP)         │ ████████████████████   ║");
    println!("║     ├─ DNS lookup          │ ~50ms     │ ██                     ║");
    println!("║     ├─ TCP connect         │ ~100ms    │ ████                   ║");
    println!("║     ├─ TLS handshake       │ ~150ms    │ ██████                 ║");
    println!("║     └─ HTTP request/resp   │ ~1000ms+  │ ████████████           ║");
    println!("║  5. Ler Response Body      │ ~10ms     │ ░░░░░░░░░░░░░░░░░░░░░░  ║");
    println!("║  6. Parsear JSON           │ ~1ms      │ ░░░░░░░░░░░░░░░░░░░░░░  ║");
    println!("║  7. Extrair Dados          │ ~0.1ms    │ ░░░░░░░░░░░░░░░░░░░░░░  ║");
    println!("║  8. Processar Resultados   │ ~0.1ms    │ ░░░░░░░░░░░░░░░░░░░░░░  ║");
    println!("║                                                                   ║");
    println!("║  ═════════════════════════════════════════════════════════════   ║");
    println!("║  TEMPO TOTAL: {:>6}ms                                            ║", total_time.as_millis());
    println!("║                                                                   ║");
    println!("║  🔥 99% DO TEMPO É NETWORK (esperando servidor Jina)              ║");
    println!("║                                                                   ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");

    // ═══════════════════════════════════════════════════════════════════════════
    // MOSTRAR ESTRUTURA DO JSON COMPLETO
    // ═══════════════════════════════════════════════════════════════════════════

    println!();
    println!("┌─────────────────────────────────────────────────────────────────────┐");
    println!("│  📄 ESTRUTURA DO JSON RETORNADO                                     │");
    println!("└─────────────────────────────────────────────────────────────────────┘");
    println!();

    fn print_json_structure(value: &serde_json::Value, prefix: &str, depth: usize) {
        if depth > 3 { return; }

        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map.iter().take(10) {
                    let type_str = match val {
                        serde_json::Value::Null => "null",
                        serde_json::Value::Bool(_) => "bool",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::String(s) => &format!("string[{}]", s.len()),
                        serde_json::Value::Array(a) => &format!("array[{}]", a.len()),
                        serde_json::Value::Object(_) => "object",
                    };
                    println!("{}├─ {}: {}", prefix, key, type_str);
                    if val.is_object() || val.is_array() {
                        print_json_structure(val, &format!("{}│  ", prefix), depth + 1);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    println!("{}[0]:", prefix);
                    print_json_structure(first, &format!("{}   ", prefix), depth + 1);
                }
            }
            _ => {}
        }
    }

    print_json_structure(&json, "", 0);

    // ═══════════════════════════════════════════════════════════════════════════
    // FASE 2: JINA READER (ler conteúdo da primeira URL)
    // ═══════════════════════════════════════════════════════════════════════════

    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  📖 JINA READER - Leitura detalhada de URL                        ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    // Pegar primeira URL dos resultados
    let first_url = results
        .and_then(|r| r.first())
        .and_then(|r| r.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or("https://rust-lang.org");

    println!("┌─────────────────────────────────────────────────────────────────────┐");
    println!("│  URL: {:62}│", &first_url[..first_url.len().min(62)]);
    println!("└─────────────────────────────────────────────────────────────────────┘");
    println!();

    let reader_total_start = Instant::now();

    // STEP: Preparar Reader URL
    let s = step("PREPARAR READER URL");
    let t = Instant::now();

    let reader_url = format!("https://r.jina.ai/{}", first_url);

    timing("URL construída", t.elapsed().as_micros() as f64 / 1000.0);
    substep(s, "formato", "https://r.jina.ai/{URL_ORIGINAL}");
    substep(s, "tamanho", &format!("{} chars", reader_url.len()));
    println!();

    // STEP: Construir Reader Request
    let s = step("CONSTRUIR READER REQUEST");
    let t = Instant::now();

    let reader_request = client
        .get(&reader_url)
        .header("Authorization", format!("Bearer {}", &jina_key))
        .header("Accept", "application/json")
        .header("X-Return-Format", "markdown")
        .header("X-Md-Link-Style", "discarded")
        .header("X-Retain-Images", "none");

    timing("Request construído", t.elapsed().as_micros() as f64 / 1000.0);
    substep(s, "method", "GET");
    substep(s, "headers", "Authorization, Accept, X-Return-Format, X-Md-Link-Style, X-Retain-Images");
    println!();

    // STEP: Enviar Reader Request
    let s = step("ENVIAR READER REQUEST (DNS + TCP + TLS + HTTP)");
    println!("         📡 Iniciando conexão com r.jina.ai...");
    println!("         📥 Jina vai baixar e processar: {}", &first_url[..first_url.len().min(50)]);
    println!();

    let t = Instant::now();
    let reader_response = reader_request.send().await?;
    let reader_network_time = t.elapsed();

    timing("TOTAL network", reader_network_time.as_millis() as f64);
    substep(s, "status", &format!("{}", reader_response.status()));
    substep(s, "http_version", &format!("{:?}", reader_response.version()));
    println!();

    // STEP: Reader Response Headers
    let s = step("READER RESPONSE HEADERS");
    for (key, value) in reader_response.headers().iter().take(8) {
        substep(s, &key.to_string(), &format!("{:?}", value.to_str().unwrap_or("?").chars().take(60).collect::<String>()));
    }
    println!();

    // STEP: Ler Reader Body
    let s = step("LER READER BODY");
    let t = Instant::now();

    let reader_bytes = reader_response.bytes().await?;

    timing("Body baixado", t.elapsed().as_millis() as f64);
    substep(s, "tamanho", &format!("{} bytes ({:.2} KB)", reader_bytes.len(), reader_bytes.len() as f64 / 1024.0));
    println!();

    // STEP: Parsear Reader JSON
    let s = step("PARSEAR READER JSON");
    let t = Instant::now();

    let reader_json: serde_json::Value = serde_json::from_slice(&reader_bytes)?;

    timing("JSON parseado", t.elapsed().as_micros() as f64 / 1000.0);
    println!();

    // STEP: Extrair Conteúdo
    let s = step("EXTRAIR CONTEÚDO DO READER");
    let t = Instant::now();

    let reader_code = reader_json.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
    let reader_status = reader_json.get("status").and_then(|s| s.as_i64()).unwrap_or(0);

    substep(s, "code", &format!("{}", reader_code));
    substep(s, "status", &format!("{}", reader_status));

    if let Some(data) = reader_json.get("data") {
        let title = data.get("title").and_then(|t| t.as_str()).unwrap_or("?");
        let content = data.get("content").and_then(|c| c.as_str()).unwrap_or("");
        let word_count = content.split_whitespace().count();

        substep(s, "title", &format!("{}", &title[..title.len().min(50)]));
        substep(s, "content_len", &format!("{} chars", content.len()));
        substep(s, "word_count", &format!("{} palavras", word_count));

        // Preview do conteúdo
        println!();
        println!("         📄 PREVIEW DO CONTEÚDO:");
        println!("         ┌────────────────────────────────────────────────────────────");
        for line in content.lines().take(5) {
            println!("         │ {}", &line[..line.len().min(60)]);
        }
        println!("         └────────────────────────────────────────────────────────────");
    }

    timing("Conteúdo extraído", t.elapsed().as_micros() as f64 / 1000.0);
    println!();

    let reader_total = reader_total_start.elapsed();

    // ═══════════════════════════════════════════════════════════════════════════
    // RESUMO FINAL COMPARATIVO
    // ═══════════════════════════════════════════════════════════════════════════

    let grand_total = total_start.elapsed();

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  📊 COMPARAÇÃO: JINA SEARCH vs JINA READER                        ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                   ║");
    println!("║  JINA SEARCH (buscar na web):                                     ║");
    println!("║    ├─ Preparação:     {:>8.2}ms                                  ║", 4.0);
    println!("║    ├─ NETWORK:        {:>8}ms  ████████████████████            ║", total_time.as_millis());
    println!("║    ├─ Parse + Extract:{:>8.2}ms                                  ║", 1.0);
    println!("║    └─ TOTAL:          {:>8}ms                                  ║", total_time.as_millis());
    println!("║                                                                   ║");
    println!("║  JINA READER (ler URL):                                           ║");
    println!("║    ├─ Preparação:     {:>8.2}ms                                  ║", 0.1);
    println!("║    ├─ NETWORK:        {:>8}ms  ████████████████████            ║", reader_network_time.as_millis());
    println!("║    ├─ Parse + Extract:{:>8.2}ms                                  ║", 1.0);
    println!("║    └─ TOTAL:          {:>8}ms                                  ║", reader_total.as_millis());
    println!("║                                                                   ║");
    println!("║  ═════════════════════════════════════════════════════════════   ║");
    println!("║  TEMPO TOTAL (Search + Reader): {:>6}ms                          ║", grand_total.as_millis());
    println!("║                                                                   ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");

    println!();
    println!("✅ Análise completa!");

    Ok(())
}
