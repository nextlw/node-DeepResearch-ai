//! Benchmarks do Sistema de Personas Cognitivas.
//!
//! Testa performance de:
//! - Criação do orquestrador
//! - Expansão de queries (paralelo vs sequencial)
//! - Diferentes tipos de perguntas
//! - Variações de tópicos e idiomas
//!
//! Executar: `cargo bench --bench personas_bench`

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use deep_research::personas::{PersonaOrchestrator, QueryContext};
use deep_research::types::{Language, TopicCategory};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPERS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn create_test_context(question: &str) -> QueryContext {
    QueryContext {
        original_query: question.to_string(),
        user_intent: format!("Find information about: {}", question),
        soundbites: vec![],
        current_date: Utc::now().date_naive(),
        detected_language: Language::English,
        detected_topic: TopicCategory::Technology,
    }
}

fn create_context_with_soundbites(question: &str, soundbites: Vec<String>) -> QueryContext {
    QueryContext {
        original_query: question.to_string(),
        user_intent: format!("Find information about: {}", question),
        soundbites,
        current_date: Utc::now().date_naive(),
        detected_language: Language::English,
        detected_topic: TopicCategory::Technology,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Criação do Orquestrador
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_orchestrator_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("orchestrator_creation");

    group.bench_function("new_default", |bencher| {
        bencher.iter(|| black_box(PersonaOrchestrator::new()))
    });

    group.bench_function("technical", |bencher| {
        bencher.iter(|| black_box(PersonaOrchestrator::technical()))
    });

    group.bench_function("investigative", |bencher| {
        bencher.iter(|| black_box(PersonaOrchestrator::investigative()))
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Expansão de Queries
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_query_expansion(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_expansion");

    let orchestrator = PersonaOrchestrator::new();

    // Diferentes tipos de perguntas
    let questions = [
        ("short", "What is Rust?"),
        ("medium", "How does Rust handle memory safety without garbage collection?"),
        ("technical", "What are the performance implications of using async/await in Rust?"),
        ("long", "Explain the complete lifecycle of a Rust program from compilation through LLVM to native code, including all optimization passes and how the borrow checker integrates with this process."),
    ];

    for (name, question) in questions.iter() {
        let context = create_test_context(question);

        group.bench_with_input(
            BenchmarkId::new("parallel", name),
            &context,
            |bencher, ctx| {
                bencher.iter(|| {
                    black_box(orchestrator.expand_query_parallel(ctx.original_query.as_str(), ctx))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sequential", name),
            &context,
            |bencher, ctx| {
                bencher.iter(|| {
                    black_box(
                        orchestrator.expand_query_sequential(ctx.original_query.as_str(), ctx),
                    )
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Paralelismo
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_parallelism(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallelism");
    group.sample_size(50);

    let orchestrator = PersonaOrchestrator::new();
    let context = create_test_context(
        "What are the best practices for building high-performance web APIs in Rust?",
    );

    // Múltiplas expansões em sequência (batch)
    for batch_size in [1, 5, 10, 20].iter() {
        let queries: Vec<String> = (0..*batch_size)
            .map(|i| format!("Query number {} about Rust programming", i))
            .collect();

        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("batch_expand", batch_size),
            &queries,
            |bencher, qs| bencher.iter(|| black_box(orchestrator.expand_batch(qs, &context))),
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Variações de Tópico
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_topic_variations(c: &mut Criterion) {
    let mut group = c.benchmark_group("topic_variations");

    let orchestrator = PersonaOrchestrator::new();
    let base_question = "What are the latest developments?";

    let topics = [
        ("technology", TopicCategory::Technology),
        ("science", TopicCategory::Science),
        ("finance", TopicCategory::Finance),
        ("health", TopicCategory::Health),
        ("general", TopicCategory::General),
    ];

    for (name, topic) in topics.iter() {
        let context = QueryContext {
            original_query: base_question.to_string(),
            user_intent: format!("Find {} developments", name),
            soundbites: vec![],
            current_date: Utc::now().date_naive(),
            detected_language: Language::English,
            detected_topic: topic.clone(),
        };

        group.bench_with_input(BenchmarkId::new("topic", name), &context, |bencher, ctx| {
            bencher.iter(|| {
                black_box(orchestrator.expand_query_parallel(ctx.original_query.as_str(), ctx))
            })
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Variações de Idioma
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_language_variations(c: &mut Criterion) {
    let mut group = c.benchmark_group("language_variations");

    let orchestrator = PersonaOrchestrator::new();

    let languages = [
        ("english", Language::English, "What is machine learning?"),
        (
            "portuguese",
            Language::Portuguese,
            "O que é aprendizado de máquina?",
        ),
        (
            "spanish",
            Language::Spanish,
            "¿Qué es el aprendizaje automático?",
        ),
        ("german", Language::German, "Was ist maschinelles Lernen?"),
    ];

    for (name, lang, question) in languages.iter() {
        let context = QueryContext {
            original_query: question.to_string(),
            user_intent: format!("Learn about machine learning in {}", name),
            soundbites: vec![],
            current_date: Utc::now().date_naive(),
            detected_language: lang.clone(),
            detected_topic: TopicCategory::Technology,
        };

        group.bench_with_input(
            BenchmarkId::new("language", name),
            &context,
            |bencher, ctx| {
                bencher.iter(|| {
                    black_box(orchestrator.expand_query_parallel(ctx.original_query.as_str(), ctx))
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Com Contexto de Soundbites
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_with_soundbites_context(c: &mut Criterion) {
    let mut group = c.benchmark_group("soundbites_context");

    let orchestrator = PersonaOrchestrator::new();
    let question = "How can I improve this architecture?";

    // Diferentes tamanhos de soundbites
    let context_sizes = [0, 5, 10, 20, 50];

    for &size in &context_sizes {
        let soundbites: Vec<String> = (0..size)
            .map(|i| {
                format!(
                    "Soundbite {} with some technical content about systems design.",
                    i
                )
            })
            .collect();

        let context = create_context_with_soundbites(question, soundbites);

        group.bench_with_input(
            BenchmarkId::new("soundbites", size),
            &context,
            |bencher, ctx| {
                bencher.iter(|| {
                    black_box(orchestrator.expand_query_parallel(ctx.original_query.as_str(), ctx))
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Verificação de Qualidade das Queries
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_query_quality_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_quality");

    let orchestrator = PersonaOrchestrator::new();

    group.bench_function("verify_uniqueness", |bencher| {
        let context = create_test_context("What is the future of AI?");

        bencher.iter(|| {
            let queries = orchestrator.expand_query_parallel(&context.original_query, &context);

            // Verificar unicidade
            let mut unique_queries: Vec<&str> = Vec::new();
            let mut duplicates = 0;

            for wq in queries.iter() {
                if unique_queries.contains(&wq.query.q.as_str()) {
                    duplicates += 1;
                } else {
                    unique_queries.push(&wq.query.q);
                }
            }

            black_box((queries.len(), duplicates))
        })
    });

    group.bench_function("verify_weights", |bencher| {
        let context = create_test_context("Explain quantum computing.");

        bencher.iter(|| {
            let queries = orchestrator.expand_query_parallel(&context.original_query, &context);

            // Verificar que pesos estão em range válido
            let valid_weights = queries.iter().all(|wq| wq.weight > 0.0 && wq.weight <= 2.0);

            let total_weight: f32 = queries.iter().map(|wq| wq.weight).sum();

            black_box((valid_weights, total_weight))
        })
    });

    group.bench_function("verify_persona_coverage", |bencher| {
        let context = create_test_context("How does blockchain work?");

        bencher.iter(|| {
            let queries = orchestrator.expand_query_parallel(&context.original_query, &context);

            // Verificar que todas as personas contribuíram
            let personas: Vec<&str> = queries.iter().map(|wq| wq.source_persona).collect();

            let expected_count = orchestrator.persona_count();
            let actual_count = personas.len();

            black_box((expected_count, actual_count, personas))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_orchestrator_creation,
    bench_query_expansion,
    bench_parallelism,
    bench_topic_variations,
    bench_language_variations,
    bench_with_soundbites_context,
    bench_query_quality_verification,
);

criterion_main!(benches);
