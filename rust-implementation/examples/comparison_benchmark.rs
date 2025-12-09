//! 🏎️ Benchmark Comparativo - Rust vs TypeScript
//!
//! Execute com: cargo run --example comparison_benchmark --release

use std::time::{Duration, Instant};
use deep_research::performance::simd::cosine_similarity;

/// Resultado de um benchmark
#[derive(Debug)]
struct BenchResult {
    name: String,
    avg_us: f64,
    min_us: f64,
    max_us: f64,
}

/// Executa benchmark e retorna estatísticas
fn benchmark<F, T>(name: &str, mut f: F, iterations: usize) -> BenchResult
where
    F: FnMut() -> T,
{
    let mut times = Vec::with_capacity(iterations);
    
    // Warmup
    for _ in 0..100 {
        std::hint::black_box(f());
    }
    
    // Medição
    for _ in 0..iterations {
        let start = Instant::now();
        std::hint::black_box(f());
        let elapsed = start.elapsed();
        times.push(elapsed.as_nanos() as f64 / 1000.0); // ns -> us
    }
    
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    
    BenchResult {
        name: name.to_string(),
        avg_us: (avg * 100.0).round() / 100.0,
        min_us: (times[0] * 100.0).round() / 100.0,
        max_us: (times[times.len() - 1] * 100.0).round() / 100.0,
    }
}

/// Gera vetor aleatório
fn random_vector(size: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    size.hash(&mut hasher);
    let seed = hasher.finish();
    
    (0..size)
        .map(|i| {
            let x = (seed.wrapping_add(i as u64) % 1000) as f32 / 1000.0;
            x
        })
        .collect()
}

/// Simula expansão de query por persona
fn expand_query(query: &str, persona: &str) -> String {
    let suffix = match persona {
        "Skeptic" => "problems issues failures real experiences",
        "Detail" => "specifications technical details comparison",
        "Historical" => "history evolution changes over time",
        "Comparative" => "vs alternatives comparison pros cons",
        "Temporal" => "2024",
        "Globalizer" => "worldwide international global",
        "Reality" => "wrong myth debunked evidence against",
        _ => "",
    };
    format!("{} {}", query, suffix)
}

fn main() {
    println!("\n🏎️  Rust Benchmark - DeepResearch AI\n");
    println!("{}", "=".repeat(60));
    
    let mut results = Vec::new();
    
    // 1. Cosine Similarity - vetores pequenos (8 dims)
    let vec_a8 = random_vector(8);
    let vec_b8 = random_vector(8);
    results.push(benchmark("cosine_8dim", || {
        cosine_similarity(&vec_a8, &vec_b8)
    }, 10000));
    
    // 2. Cosine Similarity - vetores médios (768 dims - embedding)
    let vec_a768 = random_vector(768);
    let vec_b768 = random_vector(768);
    results.push(benchmark("cosine_768dim", || {
        cosine_similarity(&vec_a768, &vec_b768)
    }, 1000));
    
    // 3. Cosine Similarity - vetores grandes (1536 dims - OpenAI)
    let vec_a1536 = random_vector(1536);
    let vec_b1536 = random_vector(1536);
    results.push(benchmark("cosine_1536dim", || {
        cosine_similarity(&vec_a1536, &vec_b1536)
    }, 1000));
    
    // 4. Batch cosine - 100 comparações
    let embeddings100: Vec<Vec<f32>> = (0..100).map(|_| random_vector(768)).collect();
    let query_emb = random_vector(768);
    results.push(benchmark("batch_100_cosine", || {
        embeddings100.iter().map(|emb| cosine_similarity(&query_emb, emb)).collect::<Vec<_>>()
    }, 100));
    
    // 5. Batch cosine - 1000 comparações (com Rayon)
    let embeddings1000: Vec<Vec<f32>> = (0..1000).map(|_| random_vector(768)).collect();
    results.push(benchmark("batch_1000_cosine", || {
        use rayon::prelude::*;
        embeddings1000.par_iter().map(|emb| cosine_similarity(&query_emb, emb)).collect::<Vec<_>>()
    }, 10));
    
    // 6. Query expansion - 7 personas
    let query = "What are the best practices for Rust programming?";
    let personas = ["Skeptic", "Detail", "Historical", "Comparative", "Temporal", "Globalizer", "Reality"];
    results.push(benchmark("expand_7_personas", || {
        personas.iter().map(|p| expand_query(query, p)).collect::<Vec<_>>()
    }, 1000));
    
    // 7. String operations - simula processamento
    let long_text = "Lorem ipsum ".repeat(1000);
    results.push(benchmark("string_split_1000", || {
        long_text.split(' ').collect::<Vec<_>>()
    }, 1000));
    
    // 8. Array operations - map/filter/reduce
    let numbers: Vec<i32> = (0..10000).collect();
    results.push(benchmark("array_ops_10k", || {
        numbers.iter()
            .map(|n| n * 2)
            .filter(|n| n % 3 == 0)
            .sum::<i32>()
    }, 100));
    
    // Output resultados
    println!("\n📊 Resultados Rust:\n");
    for r in &results {
        println!("{:20} avg: {:>10.2} µs  min: {:>10.2} µs  max: {:>10.2} µs", 
                 r.name, r.avg_us, r.min_us, r.max_us);
    }
    
    // Output JSON
    println!("\n📦 JSON:");
    println!("{{");
    println!("  \"language\": \"Rust\",");
    println!("  \"results\": [");
    for (i, r) in results.iter().enumerate() {
        let comma = if i < results.len() - 1 { "," } else { "" };
        println!("    {{ \"name\": \"{}\", \"avg_us\": {:.2}, \"min_us\": {:.2}, \"max_us\": {:.2} }}{}",
                 r.name, r.avg_us, r.min_us, r.max_us, comma);
    }
    println!("  ]");
    println!("}}");
}

