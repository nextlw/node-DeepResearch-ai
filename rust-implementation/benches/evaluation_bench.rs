//! Benchmarks do sistema de Avaliação Multidimensional.
//!
//! Testa performance de:
//! - Configuração de tipos de avaliação
//! - Criação de resultados de avaliação
//! - Pipeline de avaliação (mock)
//! - Freshness threshold calculations
//!
//! Executar: `cargo bench --bench evaluation_bench`

use criterion::{
    black_box, criterion_group, criterion_main,
    BenchmarkId, Criterion,
};
use deep_research::evaluation::{
    EvaluationType, EvaluationConfig, EvaluationResult, EvaluationContext,
};
use deep_research::types::{TopicCategory, KnowledgeItem, KnowledgeType, Reference};
use std::time::Duration;

/// Cria contexto de avaliação para testes
fn create_eval_context(topic: TopicCategory, knowledge_count: usize) -> EvaluationContext {
    let knowledge_items: Vec<KnowledgeItem> = (0..knowledge_count)
        .map(|i| KnowledgeItem {
            question: format!("Question {}", i),
            answer: format!("Answer {} with some detailed content here.", i),
            item_type: KnowledgeType::Qa,
            references: vec![Reference {
                url: format!("https://example.com/source{}", i),
                title: format!("Source {}", i),
                exact_quote: None,
                relevance_score: Some(0.8),
            }],
        })
        .collect();

    EvaluationContext {
        topic,
        knowledge_items,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Configuração de Tipos
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_evaluation_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluation_config");

    let eval_types = [
        ("definitive", EvaluationType::Definitive),
        ("freshness", EvaluationType::Freshness),
        ("plurality", EvaluationType::Plurality),
        ("completeness", EvaluationType::Completeness),
        ("strict", EvaluationType::Strict),
    ];

    for (name, eval_type) in eval_types.iter() {
        group.bench_with_input(
            BenchmarkId::new("default_config", name),
            eval_type,
            |bencher, &eval_type| {
                bencher.iter(|| {
                    black_box(eval_type.default_config())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("as_str", name),
            eval_type,
            |bencher, &eval_type| {
                bencher.iter(|| {
                    black_box(eval_type.as_str())
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Freshness Thresholds
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_freshness_threshold(c: &mut Criterion) {
    let mut group = c.benchmark_group("freshness_threshold");

    let eval_type = EvaluationType::Freshness;

    let topics = [
        ("finance", TopicCategory::Finance),
        ("news", TopicCategory::News),
        ("technology", TopicCategory::Technology),
        ("science", TopicCategory::Science),
        ("history", TopicCategory::History),
        ("general", TopicCategory::General),
    ];

    for (name, topic) in topics.iter() {
        group.bench_with_input(
            BenchmarkId::new("threshold", name),
            topic,
            |bencher, topic| {
                bencher.iter(|| {
                    black_box(eval_type.freshness_threshold(topic))
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Criação de Resultados
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_evaluation_result(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluation_result");

    group.bench_function("create_success", |bencher| {
        bencher.iter(|| {
            black_box(EvaluationResult::success(
                EvaluationType::Definitive,
                "The answer is confident and well-supported.".to_string(),
                0.95,
            ))
        })
    });

    group.bench_function("create_failure", |bencher| {
        bencher.iter(|| {
            black_box(EvaluationResult::failure(
                EvaluationType::Completeness,
                "The answer is missing key aspects.".to_string(),
                vec![
                    "Add information about X".to_string(),
                    "Include examples of Y".to_string(),
                    "Mention Z for context".to_string(),
                ],
                0.45,
            ))
        })
    });

    group.bench_function("with_duration", |bencher| {
        let result = EvaluationResult::success(
            EvaluationType::Strict,
            "Good answer.".to_string(),
            0.9,
        );

        bencher.iter(|| {
            black_box(result.clone().with_duration(Duration::from_millis(150)))
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Contexto de Avaliação
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_evaluation_context(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluation_context");

    // Diferentes tamanhos de knowledge base
    for knowledge_count in [0, 5, 10, 25, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("create", knowledge_count),
            knowledge_count,
            |bencher, &count| {
                bencher.iter(|| {
                    black_box(create_eval_context(TopicCategory::Technology, count))
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Simulação de Pipeline Completo
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_pipeline_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_simulation");
    group.sample_size(50);

    // Simula pipeline com diferentes configs
    let eval_types = vec![
        EvaluationType::Definitive,
        EvaluationType::Freshness,
        EvaluationType::Plurality,
        EvaluationType::Completeness,
        EvaluationType::Strict,
    ];

    group.bench_function("get_all_configs", |bencher| {
        bencher.iter(|| {
            let configs: Vec<EvaluationConfig> = eval_types
                .iter()
                .map(|t| t.default_config())
                .collect();
            black_box(configs)
        })
    });

    group.bench_function("simulate_all_pass", |bencher| {
        bencher.iter(|| {
            let results: Vec<EvaluationResult> = eval_types
                .iter()
                .map(|&t| {
                    EvaluationResult::success(
                        t,
                        format!("{} evaluation passed", t.as_str()),
                        0.85 + (0.1 * rand::random::<f32>()),
                    ).with_duration(Duration::from_millis(50))
                })
                .collect();

            // Verificar se todos passaram
            let all_passed = results.iter().all(|r| r.passed);
            black_box((results, all_passed))
        })
    });

    group.bench_function("simulate_early_fail", |bencher| {
        bencher.iter(|| {
            let mut results = Vec::new();

            for (i, &eval_type) in eval_types.iter().enumerate() {
                let result = if i < 2 {
                    EvaluationResult::success(
                        eval_type,
                        "Passed".to_string(),
                        0.9,
                    )
                } else {
                    EvaluationResult::failure(
                        eval_type,
                        "Failed at early check".to_string(),
                        vec!["Fix this issue".to_string()],
                        0.4,
                    )
                };

                results.push(result.clone());

                // Early exit on failure
                if !result.passed {
                    break;
                }
            }

            black_box(results)
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Agregação de Resultados
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_result_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_aggregation");

    // Preparar resultados de avaliação
    let eval_types = vec![
        EvaluationType::Definitive,
        EvaluationType::Freshness,
        EvaluationType::Plurality,
        EvaluationType::Completeness,
        EvaluationType::Strict,
    ];

    let results: Vec<EvaluationResult> = eval_types
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            if i % 2 == 0 {
                EvaluationResult::success(t, "Pass".to_string(), 0.9)
            } else {
                EvaluationResult::failure(
                    t,
                    "Fail".to_string(),
                    vec!["Suggestion".to_string()],
                    0.4,
                )
            }
            .with_duration(Duration::from_millis(50 + i as u64 * 10))
        })
        .collect();

    group.bench_function("calculate_weighted_score", |bencher| {
        bencher.iter(|| {
            let configs: Vec<EvaluationConfig> = eval_types
                .iter()
                .map(|t| t.default_config())
                .collect();

            let total_weight: f32 = configs.iter().map(|c| c.weight).sum();
            let weighted_score: f32 = results
                .iter()
                .zip(configs.iter())
                .map(|(r, c)| {
                    if r.passed { r.confidence * c.weight } else { 0.0 }
                })
                .sum();

            black_box(weighted_score / total_weight)
        })
    });

    group.bench_function("collect_suggestions", |bencher| {
        bencher.iter(|| {
            let suggestions: Vec<&str> = results
                .iter()
                .filter(|r| !r.passed)
                .flat_map(|r| r.suggestions.iter().map(|s| s.as_str()))
                .collect();

            black_box(suggestions)
        })
    });

    group.bench_function("total_duration", |bencher| {
        bencher.iter(|| {
            let total: Duration = results
                .iter()
                .map(|r| r.duration)
                .sum();

            black_box(total)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_evaluation_config,
    bench_freshness_threshold,
    bench_evaluation_result,
    bench_evaluation_context,
    bench_pipeline_simulation,
    bench_result_aggregation,
);

criterion_main!(benches);
