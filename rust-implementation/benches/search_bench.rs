//! Benchmarks do cliente de busca.
//!
//! Testa performance de:
//! - Parsing de resultados de busca
//! - Cálculo de boosts (hostname, path)
//! - Reranking de URLs
//! - Extração de hostname
//!
//! Executar: `cargo bench --bench search_bench`

use criterion::{
    black_box, criterion_group, criterion_main,
    BenchmarkId, Criterion, Throughput,
};
use deep_research::search::{
    SearchResult, UrlContent,
    extract_hostname, hostname_boost, path_boost,
};
use deep_research::types::BoostedSearchSnippet;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPERS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn create_test_snippets(count: usize) -> Vec<BoostedSearchSnippet> {
    (0..count)
        .map(|i| BoostedSearchSnippet {
            url: format!("https://example{}.com/page/{}", i % 10, i),
            title: format!("Result {} - Technical Documentation", i),
            description: format!("This is the description for result {}. It contains relevant information about the topic.", i),
            weight: 1.0,
            freq_boost: 1.0 + (i as f32 * 0.01),
            hostname_boost: if i % 3 == 0 { 1.5 } else { 1.0 },
            path_boost: if i % 5 == 0 { 1.3 } else { 1.0 },
            jina_rerank_boost: 1.0,
            final_score: 0.0,
            score: 0.0,
            merged: String::new(),
        })
        .collect()
}

fn create_test_urls() -> Vec<String> {
    vec![
        "https://en.wikipedia.org/wiki/Rust_(programming_language)".to_string(),
        "https://doc.rust-lang.org/book/".to_string(),
        "https://github.com/rust-lang/rust".to_string(),
        "https://stackoverflow.com/questions/tagged/rust".to_string(),
        "https://docs.rs/tokio/latest/tokio/".to_string(),
        "https://arxiv.org/abs/2301.00001".to_string(),
        "https://random-blog.com/rust-tutorial".to_string(),
        "https://medium.com/@user/rust-article".to_string(),
        "https://dev.to/rust-tips".to_string(),
        "https://news.ycombinator.com/item?id=12345".to_string(),
    ]
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Extração de Hostname
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_extract_hostname(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_hostname");

    let urls = create_test_urls();

    group.bench_function("single_url", |bencher| {
        bencher.iter(|| {
            black_box(extract_hostname("https://en.wikipedia.org/wiki/Rust"))
        })
    });

    group.bench_function("batch_10_urls", |bencher| {
        bencher.iter(|| {
            let results: Vec<_> = urls.iter()
                .map(|url| extract_hostname(url))
                .collect();
            black_box(results)
        })
    });

    // URLs problemáticas
    let edge_cases = vec![
        "https://subdomain.example.com/path",
        "http://localhost:8080/api",
        "https://192.168.1.1/admin",
        "invalid-url",
        "",
    ];

    group.bench_function("edge_cases", |bencher| {
        bencher.iter(|| {
            let results: Vec<_> = edge_cases.iter()
                .map(|url| extract_hostname(url))
                .collect();
            black_box(results)
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Cálculo de Boosts
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_hostname_boost(c: &mut Criterion) {
    let mut group = c.benchmark_group("hostname_boost");

    let hostnames = vec![
        "en.wikipedia.org",
        "arxiv.org",
        "github.com",
        "stackoverflow.com",
        "docs.rs",
        "rust-lang.org",
        "random-blog.com",
        "example.com",
    ];

    group.bench_function("trusted_hostname", |bencher| {
        bencher.iter(|| {
            black_box(hostname_boost("en.wikipedia.org"))
        })
    });

    group.bench_function("untrusted_hostname", |bencher| {
        bencher.iter(|| {
            black_box(hostname_boost("random-site.com"))
        })
    });

    group.bench_function("batch_hostnames", |bencher| {
        bencher.iter(|| {
            let results: Vec<_> = hostnames.iter()
                .map(|h| hostname_boost(h))
                .collect();
            black_box(results)
        })
    });

    group.finish();
}

fn bench_path_boost(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_boost");

    let urls = vec![
        "https://example.com/docs/api/v1",
        "https://example.com/documentation/guide",
        "https://example.com/tutorial/basics",
        "https://example.com/guide/getting-started",
        "https://example.com/blog/article",
        "https://example.com/news/update",
        "https://example.com/about",
        "https://example.com/products",
    ];

    group.bench_function("docs_path", |bencher| {
        bencher.iter(|| {
            black_box(path_boost("https://example.com/docs/api"))
        })
    });

    group.bench_function("regular_path", |bencher| {
        bencher.iter(|| {
            black_box(path_boost("https://example.com/about"))
        })
    });

    group.bench_function("batch_paths", |bencher| {
        bencher.iter(|| {
            let results: Vec<_> = urls.iter()
                .map(|url| path_boost(url))
                .collect();
            black_box(results)
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Criação de Resultados
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_search_result(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_result");

    for count in [10, 25, 50, 100].iter() {
        let snippets = create_test_snippets(*count);
        let snippet_texts: Vec<String> = snippets.iter()
            .map(|s| s.description.clone())
            .collect();

        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(
            BenchmarkId::new("create", count),
            count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(SearchResult {
                        urls: snippets.clone(),
                        snippets: snippet_texts.clone(),
                        total_results: *count as u64 * 10,
                    })
                })
            },
        );
    }

    group.finish();
}

fn bench_url_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_content");

    // Diferentes tamanhos de conteúdo
    let content_sizes = [100, 500, 1000, 5000, 10000];

    for &size in &content_sizes {
        let text: String = (0..size)
            .map(|i| format!("Word{} ", i))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("create", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(UrlContent {
                        title: "Test Page Title".to_string(),
                        text: text.clone(),
                        url: "https://example.com/page".to_string(),
                        word_count: size,
                    })
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Cálculo de Score Final
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_score_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("score_calculation");

    for count in [10, 50, 100, 200].iter() {
        let base_snippets = create_test_snippets(*count);

        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(
            BenchmarkId::new("calculate_final_scores", count),
            &base_snippets,
            |bencher, snippets| {
                bencher.iter(|| {
                    let mut s = snippets.clone();
                    for snippet in s.iter_mut() {
                        snippet.final_score = snippet.weight
                            * snippet.freq_boost
                            * snippet.hostname_boost
                            * snippet.path_boost
                            * snippet.jina_rerank_boost;
                        snippet.score = snippet.final_score;
                    }
                    black_box(s)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sort_by_score", count),
            &base_snippets,
            |bencher, snippets| {
                bencher.iter(|| {
                    let mut sorted = snippets.clone();
                    // Calcular scores
                    for snippet in sorted.iter_mut() {
                        snippet.final_score = snippet.weight
                            * snippet.freq_boost
                            * snippet.hostname_boost
                            * snippet.path_boost;
                    }
                    sorted.sort_by(|a, b| {
                        b.final_score.partial_cmp(&a.final_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    black_box(sorted)
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Combinação de Boosts (Pipeline Completo)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_boost_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("boost_pipeline");
    group.sample_size(50);

    let urls = vec![
        "https://en.wikipedia.org/wiki/Rust",
        "https://doc.rust-lang.org/book/",
        "https://github.com/rust-lang/rust",
        "https://stackoverflow.com/questions/tagged/rust",
        "https://random-blog.com/rust-tutorial",
        "https://medium.com/@user/rust-article",
    ];

    group.bench_function("full_boost_pipeline", |bencher| {
        bencher.iter(|| {
            let results: Vec<(Option<String>, f32, f32)> = urls.iter()
                .map(|url| {
                    let hostname = extract_hostname(url);
                    let h_boost = hostname.as_ref()
                        .map(|h| hostname_boost(h))
                        .unwrap_or(1.0);
                    let p_boost = path_boost(url);
                    (hostname, h_boost, p_boost)
                })
                .collect();
            black_box(results)
        })
    });

    group.bench_function("calculate_combined_boost", |bencher| {
        bencher.iter(|| {
            let combined: Vec<f32> = urls.iter()
                .map(|url| {
                    let hostname = extract_hostname(url);
                    let h_boost = hostname.as_ref()
                        .map(|h| hostname_boost(h))
                        .unwrap_or(1.0);
                    let p_boost = path_boost(url);
                    h_boost * p_boost
                })
                .collect();
            black_box(combined)
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Filtragem de Resultados
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_result_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_filtering");

    for count in [50, 100, 200].iter() {
        let mut snippets = create_test_snippets(*count);

        // Atribuir scores variados
        for (i, snippet) in snippets.iter_mut().enumerate() {
            snippet.final_score = (i as f32 / *count as f32) * 2.0;
        }

        group.throughput(Throughput::Elements(*count as u64));

        // Filtrar por threshold de score
        group.bench_with_input(
            BenchmarkId::new("filter_by_score", count),
            count,
            |bencher, _| {
                bencher.iter(|| {
                    let filtered: Vec<_> = snippets.iter()
                        .filter(|s| s.final_score >= 0.5)
                        .collect();
                    black_box(filtered)
                })
            },
        );

        // Top N resultados
        group.bench_with_input(
            BenchmarkId::new("top_10", count),
            count,
            |bencher, _| {
                bencher.iter(|| {
                    let mut sorted = snippets.clone();
                    sorted.sort_by(|a, b| {
                        b.final_score.partial_cmp(&a.final_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let top: Vec<_> = sorted.into_iter().take(10).collect();
                    black_box(top)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_extract_hostname,
    bench_hostname_boost,
    bench_path_boost,
    bench_search_result,
    bench_url_content,
    bench_score_calculation,
    bench_boost_pipeline,
    bench_result_filtering,
);

criterion_main!(benches);
