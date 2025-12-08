//! Benchmarks End-to-End (E2E) do sistema completo.
//!
//! Testa performance de:
//! - Fluxo completo de pesquisa (simulado)
//! - Integração entre componentes
//! - Overhead de orquestração
//! - Métricas de latência por step
//!
//! Executar: `cargo bench --bench e2e_bench`

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use deep_research::agent::{
    ActionPermissions, AgentAction, AgentPrompt, AgentState, DiaryEntry, ResearchResult, TokenUsage,
};
use deep_research::evaluation::{EvaluationResult, EvaluationType};
use deep_research::performance::simd::{cosine_similarity, dedup_queries};
use deep_research::personas::{PersonaOrchestrator, QueryContext};
use deep_research::search::SearchResult;
use deep_research::types::{
    BoostedSearchSnippet, KnowledgeItem, KnowledgeType, Language, Reference, SerpQuery,
    TopicCategory,
};
use std::time::Duration;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPERS - Simulação de Componentes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn generate_mock_embedding(dim: usize) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

fn generate_mock_search_results(count: usize) -> SearchResult {
    let urls: Vec<BoostedSearchSnippet> = (0..count)
        .map(|i| BoostedSearchSnippet {
            url: format!("https://example{}.com/page/{}", i % 5, i),
            title: format!("Result {} - Documentation", i),
            description: format!("Description for result {}", i),
            weight: 1.0,
            freq_boost: 1.0,
            hostname_boost: 1.0,
            path_boost: 1.0,
            jina_rerank_boost: 1.0,
            final_score: 0.9 - (i as f32 * 0.05),
            score: 0.9 - (i as f32 * 0.05),
            merged: String::new(),
        })
        .collect();

    let snippets: Vec<String> = urls.iter().map(|u| u.description.clone()).collect();

    SearchResult {
        urls,
        snippets,
        total_results: count as u64 * 10,
    }
}

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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Simulação de Step Completo
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_single_step_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_step");
    group.sample_size(50);

    let orchestrator = PersonaOrchestrator::new();

    // Step 1: Expansão de Query
    group.bench_function("step1_query_expansion", |bencher| {
        let context = create_test_context("What are the best practices for Rust web development?");

        bencher.iter(|| {
            black_box(orchestrator.expand_query_parallel(&context.original_query, &context))
        })
    });

    // Step 2: Deduplicação (simulada com embeddings)
    group.bench_function("step2_deduplication", |bencher| {
        let new_embeddings: Vec<Vec<f32>> = (0..10).map(|_| generate_mock_embedding(768)).collect();
        let existing_embeddings: Vec<Vec<f32>> =
            (0..50).map(|_| generate_mock_embedding(768)).collect();

        bencher.iter(|| black_box(dedup_queries(&new_embeddings, &existing_embeddings, 0.86)))
    });

    // Step 3: Processamento de resultados de busca
    group.bench_function("step3_process_search_results", |bencher| {
        let search_results = generate_mock_search_results(20);

        bencher.iter(|| {
            let mut sorted = search_results.urls.clone();
            sorted.sort_by(|a, b| {
                b.final_score
                    .partial_cmp(&a.final_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let top_urls: Vec<_> = sorted.into_iter().take(5).collect();
            black_box(top_urls)
        })
    });

    // Step 4: Criação de entrada no diário
    group.bench_function("step4_diary_entry", |bencher| {
        bencher.iter(|| {
            let entry = DiaryEntry::Search {
                queries: vec![
                    SerpQuery {
                        q: "rust web framework".to_string(),
                        tbs: None,
                        location: None,
                    },
                    SerpQuery {
                        q: "actix vs axum".to_string(),
                        tbs: None,
                        location: None,
                    },
                ],
                think: "Searching for Rust web frameworks comparison".to_string(),
                urls_found: 15,
            };
            black_box(entry.format())
        })
    });

    // Step 5: Atualização de estado
    group.bench_function("step5_state_update", |bencher| {
        bencher.iter(|| {
            let state = AgentState::Processing {
                step: 3,
                total_step: 15,
                current_question: "What are the best Rust web frameworks?".to_string(),
                budget_used: 0.35,
            };
            black_box((state.is_terminal(), state.budget_used(), state.total_step()))
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Simulação de Pipeline de Avaliação
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_evaluation_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluation_pipeline");
    group.sample_size(50);

    let eval_types = vec![
        EvaluationType::Definitive,
        EvaluationType::Freshness,
        EvaluationType::Plurality,
        EvaluationType::Completeness,
        EvaluationType::Strict,
    ];

    // Pipeline completo (todas as avaliações passam)
    group.bench_function("all_pass", |bencher| {
        bencher.iter(|| {
            let results: Vec<EvaluationResult> = eval_types
                .iter()
                .map(|&t| {
                    EvaluationResult::success(t, format!("{} check passed", t.as_str()), 0.9)
                        .with_duration(Duration::from_millis(50))
                })
                .collect();

            let all_passed = results.iter().all(|r| r.passed);
            let avg_confidence: f32 =
                results.iter().map(|r| r.confidence).sum::<f32>() / results.len() as f32;

            black_box((all_passed, avg_confidence))
        })
    });

    // Pipeline com falha rápida
    group.bench_function("early_fail", |bencher| {
        bencher.iter(|| {
            let mut results = Vec::new();

            for (i, &eval_type) in eval_types.iter().enumerate() {
                let result = if i < 2 {
                    EvaluationResult::success(eval_type, "Pass".to_string(), 0.9)
                } else {
                    EvaluationResult::failure(
                        eval_type,
                        "Failed check".to_string(),
                        vec!["Suggestion for improvement".to_string()],
                        0.4,
                    )
                };

                let passed = result.passed;
                results.push(result);

                if !passed {
                    break; // Early exit
                }
            }

            black_box(results)
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Simulação de Pesquisa Completa (Multi-Step)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_multi_step_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_step");
    group.sample_size(30);

    let orchestrator = PersonaOrchestrator::new();

    // Simula N steps de pesquisa
    for num_steps in [3, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::new("steps", num_steps),
            num_steps,
            |bencher, &num_steps| {
                bencher.iter(|| {
                    let mut diary = Vec::new();
                    let mut knowledge = Vec::new();
                    let mut total_tokens: u64 = 0;

                    for step in 0..num_steps {
                        // 1. Decidir ação (simulado)
                        let action = if step < num_steps - 1 {
                            AgentAction::Search {
                                queries: vec![SerpQuery {
                                    q: format!("query step {}", step),
                                    tbs: None,
                                    location: None,
                                }],
                                think: format!("Step {} reasoning", step),
                            }
                        } else {
                            AgentAction::Answer {
                                answer: "Final answer...".to_string(),
                                references: vec![],
                                think: "Ready to answer".to_string(),
                            }
                        };

                        // 2. Executar ação
                        match &action {
                            AgentAction::Search { queries, think, .. } => {
                                // Expandir queries
                                let context = create_test_context(&queries[0].q);
                                let _expanded = orchestrator
                                    .expand_query_parallel(&context.original_query, &context);

                                // Registrar no diário
                                diary.push(DiaryEntry::Search {
                                    queries: queries.clone(),
                                    think: think.clone(),
                                    urls_found: 10,
                                });

                                // Simular conhecimento adquirido
                                knowledge.push(KnowledgeItem {
                                    question: queries[0].q.clone(),
                                    answer: format!("Info from step {}", step),
                                    item_type: KnowledgeType::Qa,
                                    references: vec![],
                                });

                                total_tokens += 500;
                            }
                            AgentAction::Answer { .. } => {
                                total_tokens += 1000;
                            }
                            _ => {}
                        }
                    }

                    black_box((diary.len(), knowledge.len(), total_tokens))
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Criação de Resultado Final
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_final_result(c: &mut Criterion) {
    let mut group = c.benchmark_group("final_result");

    // Diferentes tamanhos de resultado
    for ref_count in [0, 5, 10, 20].iter() {
        let references: Vec<Reference> = (0..*ref_count)
            .map(|i| Reference {
                url: format!("https://source{}.com", i),
                title: format!("Source {}", i),
                exact_quote: Some(format!("Quote from source {}", i)),
                relevance_score: Some(0.9 - (i as f32 * 0.02)),
            })
            .collect();

        let visited_urls: Vec<String> = (0..(*ref_count * 2))
            .map(|i| format!("https://visited{}.com", i))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("success", ref_count),
            ref_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(ResearchResult {
                        success: true,
                        answer: Some("Comprehensive answer based on research...".to_string()),
                        references: references.clone(),
                        trivial: false,
                        token_usage: TokenUsage {
                            prompt_tokens: 25000,
                            completion_tokens: 5000,
                            total_tokens: 30000,
                        },
                        visited_urls: visited_urls.clone(),
                        error: None,
                        total_time_ms: 8000,
                        search_time_ms: 2500,
                        read_time_ms: 1200,
                        llm_time_ms: 4300,
                    })
                })
            },
        );
    }

    group.bench_function("failure", |bencher| {
        bencher.iter(|| {
            black_box(ResearchResult {
                success: false,
                answer: None,
                references: vec![],
                trivial: false,
                token_usage: TokenUsage {
                    prompt_tokens: 50000,
                    completion_tokens: 10000,
                    total_tokens: 60000,
                },
                visited_urls: vec![],
                error: Some("Budget exhausted without satisfactory answer".to_string()),
                total_time_ms: 45000,
                search_time_ms: 15000,
                read_time_ms: 8000,
                llm_time_ms: 22000,
            })
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Overhead de Orquestração
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_orchestration_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("orchestration_overhead");

    // Criação de prompt com diferentes tamanhos de diário
    for diary_size in [5, 15, 30, 50].iter() {
        let diary: Vec<DiaryEntry> = (0..*diary_size)
            .map(|i| match i % 4 {
                0 => DiaryEntry::Search {
                    queries: vec![SerpQuery {
                        q: format!("query {}", i),
                        tbs: None,
                        location: None,
                    }],
                    think: format!("Search {}", i),
                    urls_found: 10,
                },
                1 => DiaryEntry::Read {
                    urls: vec![format!("https://url{}.com", i)],
                    think: format!("Read {}", i),
                },
                2 => DiaryEntry::Reflect {
                    questions: vec![format!("Gap question {}", i)],
                    think: format!("Reflect {}", i),
                },
                _ => DiaryEntry::FailedAnswer {
                    answer: format!("Failed answer {}", i),
                    eval_type: EvaluationType::Completeness,
                    reason: "Missing info".to_string(),
                },
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("prompt_creation", diary_size),
            diary_size,
            |bencher, _| {
                bencher.iter(|| {
                    let prompt = AgentPrompt {
                        system: "You are a research assistant...".to_string(),
                        user: "Research question...".to_string(),
                        diary: diary.clone(),
                    };

                    // Simular formatação do diário
                    let formatted: Vec<String> = prompt.diary.iter().map(|e| e.format()).collect();

                    black_box((prompt, formatted.len()))
                })
            },
        );
    }

    // Verificação de permissões
    group.bench_function("permissions_check", |bencher| {
        let all_perms = ActionPermissions::all_enabled();
        let beast_perms = ActionPermissions::beast_mode();

        bencher.iter(|| {
            black_box((
                all_perms.search && all_perms.read && all_perms.reflect,
                beast_perms.search || beast_perms.reflect,
                all_perms.answer && all_perms.coding,
            ))
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Similaridade em Contexto E2E
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_e2e_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e_similarity");
    group.sample_size(50);

    // Simula comparação de query contra knowledge base
    let dim = 768;
    let query_embedding = generate_mock_embedding(dim);

    for kb_size in [10, 50, 100, 200].iter() {
        let kb_embeddings: Vec<Vec<f32>> = (0..*kb_size)
            .map(|_| generate_mock_embedding(dim))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("find_relevant", kb_size),
            kb_size,
            |bencher, _| {
                bencher.iter(|| {
                    let similarities: Vec<(usize, f32)> = kb_embeddings
                        .iter()
                        .enumerate()
                        .map(|(i, emb)| (i, cosine_similarity(&query_embedding, emb)))
                        .filter(|(_, sim)| *sim >= 0.7)
                        .collect();
                    black_box(similarities)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_step_simulation,
    bench_evaluation_pipeline,
    bench_multi_step_simulation,
    bench_final_result,
    bench_orchestration_overhead,
    bench_e2e_similarity,
);

criterion_main!(benches);
