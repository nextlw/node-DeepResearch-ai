//! 🔥 Comparação: JINA READER vs RUST LOCAL + OpenAI
//!
//! Compara as duas implementações de leitura de URLs:
//! 1. Jina Reader API - API externa faz download + extração
//! 2. Rust Local + OpenAI - Rust baixa HTML, OpenAI extrai conteúdo
//!
//! Execute com: cargo run --example jina_vs_rust_comparison --release

use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Carregar .env
    let env_path = std::path::Path::new("../.env");
    if env_path.exists() {
        dotenvy::from_path(env_path).ok();
    }
    dotenvy::dotenv().ok();

    let jina_key = std::env::var("JINA_API_KEY")?;
    let openai_key = std::env::var("OPENAI_API_KEY")?;

    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  🔥 COMPARAÇÃO: JINA READER vs RUST LOCAL + OpenAI                ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // URL para testar
    let test_url = "https://www.rust-lang.org/";

    println!("┌─────────────────────────────────────────────────────────────────────┐");
    println!("│  URL de teste: {:54}│", test_url);
    println!("└─────────────────────────────────────────────────────────────────────┘");
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // MÉTODO 1: JINA READER API
    // ═══════════════════════════════════════════════════════════════════════════

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  📖 MÉTODO 1: JINA READER API                                     ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║  Fluxo: Seu código → Jina API → (Jina baixa + extrai) → Resposta  ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    let jina_start = Instant::now();

    // Step 1: Enviar request
    println!("[JINA] 📍 Step 1: Enviando request para r.jina.ai...");
    let jina_req_start = Instant::now();

    let jina_response = client
        .get(format!("https://r.jina.ai/{}", test_url))
        .header("Authorization", format!("Bearer {}", jina_key))
        .header("Accept", "application/json")
        .header("X-Return-Format", "markdown")
        .send()
        .await?;

    let jina_network_time = jina_req_start.elapsed();
    println!("[JINA]        ⏱️  Network: {}ms", jina_network_time.as_millis());
    println!("[JINA]        Status: {}", jina_response.status());
    println!();

    // Step 2: Ler body
    println!("[JINA] 📍 Step 2: Lendo response body...");
    let jina_body_start = Instant::now();
    let jina_bytes = jina_response.bytes().await?;
    let jina_body_time = jina_body_start.elapsed();
    println!("[JINA]        ⏱️  Download: {}ms", jina_body_time.as_millis());
    println!("[JINA]        Tamanho: {} bytes ({:.2} KB)", jina_bytes.len(), jina_bytes.len() as f64 / 1024.0);
    println!();

    // Step 3: Parsear JSON
    println!("[JINA] 📍 Step 3: Parseando JSON...");
    let jina_parse_start = Instant::now();
    let jina_json: serde_json::Value = serde_json::from_slice(&jina_bytes)?;
    let jina_parse_time = jina_parse_start.elapsed();
    println!("[JINA]        ⏱️  Parse: {:.2}ms", jina_parse_time.as_micros() as f64 / 1000.0);
    println!();

    // Extrair conteúdo
    let jina_content = jina_json
        .get("data")
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");
    let jina_title = jina_json
        .get("data")
        .and_then(|d| d.get("title"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let jina_word_count = jina_content.split_whitespace().count();

    let jina_total = jina_start.elapsed();

    println!("[JINA] ✅ RESULTADO:");
    println!("[JINA]    Título: {}", &jina_title[..jina_title.len().min(50)]);
    println!("[JINA]    Conteúdo: {} chars, {} palavras", jina_content.len(), jina_word_count);
    println!("[JINA]    ⏱️  TEMPO TOTAL: {}ms", jina_total.as_millis());
    println!();

    // Mostrar preview do conteúdo Jina
    println!("[JINA] 📄 PREVIEW DO CONTEÚDO EXTRAÍDO:");
    println!("┌────────────────────────────────────────────────────────────────────────");
    for line in jina_content.lines().take(15) {
        if !line.trim().is_empty() {
            println!("│ {}", &line[..line.len().min(70)]);
        }
    }
    println!("└────────────────────────────────────────────────────────────────────────");
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // MÉTODO 2: RUST LOCAL + OpenAI
    // ═══════════════════════════════════════════════════════════════════════════

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  🦀 MÉTODO 2: RUST LOCAL + OpenAI                                 ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║  Fluxo: Rust baixa HTML → OpenAI extrai conteúdo → Resposta       ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    let rust_start = Instant::now();

    // Step 1: Baixar HTML diretamente
    println!("[RUST] 📍 Step 1: Baixando HTML diretamente...");
    let rust_download_start = Instant::now();

    let html_response = client
        .get(test_url)
        .header("User-Agent", "Mozilla/5.0 (compatible; DeepResearch/1.0)")
        .send()
        .await?;

    let rust_download_time = rust_download_start.elapsed();
    println!("[RUST]        ⏱️  Network: {}ms", rust_download_time.as_millis());
    println!("[RUST]        Status: {}", html_response.status());

    let html_bytes = html_response.bytes().await?;
    println!("[RUST]        Tamanho HTML: {} bytes ({:.2} KB)", html_bytes.len(), html_bytes.len() as f64 / 1024.0);
    println!();

    // Step 2: Extrair texto do HTML (sem OpenAI, usando readability simples)
    println!("[RUST] 📍 Step 2: Extraindo texto do HTML (local)...");
    let rust_extract_start = Instant::now();

    let html_str = String::from_utf8_lossy(&html_bytes);

    // Extração simples: remover tags HTML
    let text_content = extract_text_from_html(&html_str);
    let rust_word_count = text_content.split_whitespace().count();

    let rust_extract_time = rust_extract_start.elapsed();
    println!("[RUST]        ⏱️  Extração local: {:.2}ms", rust_extract_time.as_micros() as f64 / 1000.0);
    println!("[RUST]        Texto extraído: {} chars, {} palavras", text_content.len(), rust_word_count);
    println!();

    // Step 3: (Opcional) Enviar para OpenAI para sumarizar
    println!("[RUST] 📍 Step 3: Enviando para OpenAI gpt-4o-mini...");
    let openai_start = Instant::now();

    let openai_response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {
                    "role": "system",
                    "content": "Extract the main title and a brief summary (max 100 words) from this HTML content. Return as JSON: {\"title\": \"...\", \"summary\": \"...\"}"
                },
                {
                    "role": "user",
                    "content": &text_content[..text_content.len().min(4000)]
                }
            ],
            "max_tokens": 200,
            "temperature": 0
        }))
        .send()
        .await?;

    let openai_network_time = openai_start.elapsed();
    println!("[RUST]        ⏱️  OpenAI Network: {}ms", openai_network_time.as_millis());
    println!("[RUST]        Status: {}", openai_response.status());

    let openai_body = openai_response.text().await?;
    let openai_json: serde_json::Value = serde_json::from_str(&openai_body)?;

    let ai_content = openai_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("{}");

    println!("[RUST]        OpenAI resposta: {} chars", ai_content.len());
    println!();

    let rust_total = rust_start.elapsed();

    println!("[RUST] ✅ RESULTADO:");
    println!("[RUST]    Conteúdo local: {} chars, {} palavras", text_content.len(), rust_word_count);
    println!("[RUST]    OpenAI resposta: {}", &ai_content[..ai_content.len().min(100)]);
    println!("[RUST]    ⏱️  TEMPO TOTAL: {}ms", rust_total.as_millis());
    println!();

    // Mostrar preview do conteúdo Rust Local
    println!("[RUST] 📄 PREVIEW DO CONTEÚDO EXTRAÍDO (Local):");
    println!("┌────────────────────────────────────────────────────────────────────────");
    for line in text_content.lines().take(15) {
        if !line.trim().is_empty() {
            println!("│ {}", &line[..line.len().min(70)]);
        }
    }
    println!("└────────────────────────────────────────────────────────────────────────");
    println!();

    // Mostrar resposta completa do OpenAI
    println!("[RUST] 🤖 RESPOSTA OPENAI (Sumarização):");
    println!("┌────────────────────────────────────────────────────────────────────────");
    for line in ai_content.lines() {
        println!("│ {}", &line[..line.len().min(70)]);
    }
    println!("└────────────────────────────────────────────────────────────────────────");
    println!();

    // ═══════════════════════════════════════════════════════════════════════════
    // COMPARAÇÃO FINAL
    // ═══════════════════════════════════════════════════════════════════════════

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  📊 COMPARAÇÃO FINAL                                              ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                   ║");
    println!("║  JINA READER:                                                     ║");
    println!("║    ├─ Network (Jina):    {:>6}ms                                 ║", jina_network_time.as_millis());
    println!("║    ├─ Download body:     {:>6}ms                                 ║", jina_body_time.as_millis());
    println!("║    ├─ Parse JSON:        {:>6.2}ms                                 ║", jina_parse_time.as_micros() as f64 / 1000.0);
    println!("║    ├─ Conteúdo:          {:>6} palavras                          ║", jina_word_count);
    println!("║    └─ TOTAL:             {:>6}ms                                 ║", jina_total.as_millis());
    println!("║                                                                   ║");
    println!("║  RUST LOCAL + OpenAI:                                             ║");
    println!("║    ├─ Download HTML:     {:>6}ms                                 ║", rust_download_time.as_millis());
    println!("║    ├─ Extração local:    {:>6.2}ms                                 ║", rust_extract_time.as_micros() as f64 / 1000.0);
    println!("║    ├─ OpenAI call:       {:>6}ms                                 ║", openai_network_time.as_millis());
    println!("║    ├─ Conteúdo:          {:>6} palavras                          ║", rust_word_count);
    println!("║    └─ TOTAL:             {:>6}ms                                 ║", rust_total.as_millis());
    println!("║                                                                   ║");
    println!("║  ═════════════════════════════════════════════════════════════   ║");

    let diff = jina_total.as_millis() as i64 - rust_total.as_millis() as i64;
    let winner = if diff > 0 { "RUST LOCAL" } else { "JINA" };
    let faster_by = diff.abs();

    println!("║  🏆 VENCEDOR: {} ({}ms mais rápido)                    ║", winner, faster_by);
    println!("║                                                                   ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");

    // Breakdown visual
    println!();
    println!("📈 TIMELINE:");
    println!();

    let max_time = jina_total.as_millis().max(rust_total.as_millis()) as f64;
    let jina_bar = (jina_total.as_millis() as f64 / max_time * 40.0) as usize;
    let rust_bar = (rust_total.as_millis() as f64 / max_time * 40.0) as usize;

    println!("JINA:  {} {}ms", "█".repeat(jina_bar), jina_total.as_millis());
    println!("RUST:  {} {}ms", "█".repeat(rust_bar), rust_total.as_millis());

    println!();
    println!("✅ Comparação completa!");

    Ok(())
}

/// Extrai texto de HTML removendo tags (implementação simples)
fn extract_text_from_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let html_lower = html.to_lowercase();
    let mut i = 0;
    let chars: Vec<char> = html.chars().collect();

    while i < chars.len() {
        let c = chars[i];

        // Detectar início de script ou style
        if i + 7 < chars.len() {
            let slice: String = chars[i..i+7].iter().collect();
            if slice.to_lowercase() == "<script" {
                in_script = true;
            }
            if slice.to_lowercase() == "<style>" || (i + 6 < chars.len() && chars[i..i+6].iter().collect::<String>().to_lowercase() == "<style") {
                in_style = true;
            }
        }

        // Detectar fim de script ou style
        if i + 9 < chars.len() {
            let slice: String = chars[i..i+9].iter().collect();
            if slice.to_lowercase() == "</script>" {
                in_script = false;
                i += 9;
                continue;
            }
        }
        if i + 8 < chars.len() {
            let slice: String = chars[i..i+8].iter().collect();
            if slice.to_lowercase() == "</style>" {
                in_style = false;
                i += 8;
                continue;
            }
        }

        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
            result.push(' ');
        } else if !in_tag && !in_script && !in_style {
            result.push(c);
        }

        i += 1;
    }

    // Limpar espaços extras
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
