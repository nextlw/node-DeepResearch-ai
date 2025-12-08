//! Benchmarks do Agente de Pesquisa.
//!
//! Testa performance de:
//! - Transições de estado
//! - Criação e manipulação de ações
//! - Contexto do agente
//! - Diário de execução
//!
//! Executar: `cargo bench --bench agent_bench`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use deep_research::agent::{
    ActionPermissions, AgentAction, AgentPrompt, AgentState, AnswerResult, DiaryEntry,
    ResearchResult, TokenUsage,
};
use deep_research::types::{KnowledgeItem, KnowledgeType, Reference, SerpQuery};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Estados do Agente
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_agent_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_state");

    // Criação de estados
    group.bench_function("create_processing", |bencher| {
        bencher.iter(|| {
            black_box(AgentState::Processing {
                step: 1,
                total_step: 5,
                current_question: "What is Rust?".to_string(),
                budget_used: 0.25,
            })
        })
    });

    group.bench_function("create_beast_mode", |bencher| {
        bencher.iter(|| {
            black_box(AgentState::BeastMode {
                attempts: 2,
                last_failure: "Completeness check failed".to_string(),
            })
        })
    });

    group.bench_function("create_completed", |bencher| {
        let refs = vec![Reference {
            url: "https://rust-lang.org".to_string(),
            title: "Rust Programming Language".to_string(),
            exact_quote: Some("Memory safety without garbage collection".to_string()),
            relevance_score: Some(0.95),
            answer_chunk: Some("Rust provides memory safety...".to_string()),
            answer_position: Some((0, 50)),
        }];

        bencher.iter(|| {
            black_box(AgentState::Completed {
                answer: "Rust is a systems programming language...".to_string(),
                references: refs.clone(),
                trivial: false,
            })
        })
    });

    group.bench_function("create_failed", |bencher| {
        let knowledge = vec![KnowledgeItem {
            question: "What is Rust?".to_string(),
            answer: "Partial answer found...".to_string(),
            item_type: KnowledgeType::Qa,
            references: vec![],
        }];

        bencher.iter(|| {
            black_box(AgentState::Failed {
                reason: "Budget exhausted without satisfactory answer".to_string(),
                partial_knowledge: knowledge.clone(),
            })
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Verificações de Estado
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_state_checks(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_checks");

    let processing = AgentState::Processing {
        step: 3,
        total_step: 10,
        current_question: "Test question".to_string(),
        budget_used: 0.5,
    };

    let completed = AgentState::Completed {
        answer: "Test answer".to_string(),
        references: vec![],
        trivial: false,
    };

    let beast_mode = AgentState::BeastMode {
        attempts: 1,
        last_failure: "Test".to_string(),
    };

    group.bench_function("is_terminal", |bencher| {
        bencher.iter(|| black_box((processing.is_terminal(), completed.is_terminal())))
    });

    group.bench_function("is_processing", |bencher| {
        bencher.iter(|| black_box(processing.is_processing()))
    });

    group.bench_function("is_beast_mode", |bencher| {
        bencher.iter(|| black_box(beast_mode.is_beast_mode()))
    });

    group.bench_function("can_transition_to", |bencher| {
        bencher.iter(|| {
            black_box((
                processing.can_transition_to(&beast_mode),
                processing.can_transition_to(&completed),
                beast_mode.can_transition_to(&completed),
                completed.can_transition_to(&processing), // should be false
            ))
        })
    });

    group.bench_function("budget_used", |bencher| {
        bencher.iter(|| black_box(processing.budget_used()))
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Ações do Agente
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_agent_actions(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_actions");

    group.bench_function("create_search", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::Search {
                queries: vec![
                    SerpQuery {
                        q: "rust programming".to_string(),
                        tbs: None,
                        location: None,
                    },
                    SerpQuery {
                        q: "rust vs go".to_string(),
                        tbs: Some("qdr:m".to_string()),
                        location: None,
                    },
                ],
                think: "Need to find information about Rust programming language.".to_string(),
            })
        })
    });

    group.bench_function("create_read", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::Read {
                urls: vec![
                    "https://rust-lang.org".to_string(),
                    "https://doc.rust-lang.org/book/".to_string(),
                ],
                think: "Reading official documentation for authoritative information.".to_string(),
            })
        })
    });

    group.bench_function("create_reflect", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::Reflect {
                gap_questions: vec![
                    "What are Rust's main advantages over C++?".to_string(),
                    "How does Rust handle memory safety?".to_string(),
                ],
                think: "Identified gaps in current knowledge.".to_string(),
            })
        })
    });

    group.bench_function("create_answer", |bencher| {
        let refs = vec![
            Reference {
                url: "https://rust-lang.org".to_string(),
                title: "Rust".to_string(),
                exact_quote: None,
                relevance_score: Some(0.9),
                answer_chunk: None,
                answer_position: None,
            },
        ];

        bencher.iter(|| {
            black_box(AgentAction::Answer {
                answer: "Rust is a systems programming language that provides memory safety guarantees without using a garbage collector. It achieves this through its ownership system and borrow checker.".to_string(),
                references: refs.clone(),
                think: "Have sufficient information to provide a comprehensive answer.".to_string(),
            })
        })
    });

    group.bench_function("create_coding", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::Coding {
                code: "const data = JSON.parse(input); return data.map(x => x * 2);".to_string(),
                think: "Processing data with JavaScript.".to_string(),
            })
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Verificações de Ações
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_action_checks(c: &mut Criterion) {
    let mut group = c.benchmark_group("action_checks");

    let search = AgentAction::Search {
        queries: vec![SerpQuery {
            q: "test".to_string(),
            tbs: None,
            location: None,
        }],
        think: "Testing".to_string(),
    };

    let answer = AgentAction::Answer {
        answer: "Test answer".to_string(),
        references: vec![],
        think: "Testing".to_string(),
    };

    group.bench_function("name", |bencher| {
        bencher.iter(|| black_box((search.name(), answer.name())))
    });

    group.bench_function("think", |bencher| {
        bencher.iter(|| black_box((search.think(), answer.think())))
    });

    group.bench_function("is_search", |bencher| {
        bencher.iter(|| black_box((search.is_search(), answer.is_search())))
    });

    group.bench_function("is_answer", |bencher| {
        bencher.iter(|| black_box((search.is_answer(), answer.is_answer())))
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Permissões de Ações
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_action_permissions(c: &mut Criterion) {
    let mut group = c.benchmark_group("action_permissions");

    group.bench_function("all_enabled", |bencher| {
        bencher.iter(|| black_box(ActionPermissions::all_enabled()))
    });

    group.bench_function("beast_mode", |bencher| {
        bencher.iter(|| black_box(ActionPermissions::beast_mode()))
    });

    group.bench_function("check_permissions", |bencher| {
        let perms = ActionPermissions::all_enabled();
        bencher.iter(|| {
            black_box((
                perms.search,
                perms.read,
                perms.reflect,
                perms.answer,
                perms.coding,
            ))
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Diário de Execução
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_diary_entry(c: &mut Criterion) {
    let mut group = c.benchmark_group("diary_entry");

    group.bench_function("create_search_entry", |bencher| {
        bencher.iter(|| {
            black_box(DiaryEntry::Search {
                queries: vec![SerpQuery {
                    q: "rust".to_string(),
                    tbs: None,
                    location: None,
                }],
                think: "Searching for Rust information".to_string(),
                urls_found: 15,
            })
        })
    });

    group.bench_function("create_failed_answer", |bencher| {
        bencher.iter(|| {
            black_box(DiaryEntry::FailedAnswer {
                answer: "Previous failed answer...".to_string(),
                eval_type: deep_research::evaluation::EvaluationType::Completeness,
                reason: "Missing key aspects".to_string(),
            })
        })
    });

    // Format benchmarks
    let search_entry = DiaryEntry::Search {
        queries: vec![SerpQuery {
            q: "test".to_string(),
            tbs: None,
            location: None,
        }],
        think: "Testing".to_string(),
        urls_found: 10,
    };

    let read_entry = DiaryEntry::Read {
        urls: vec!["https://example.com".to_string()],
        think: "Reading".to_string(),
    };

    group.bench_function("format_search", |bencher| {
        bencher.iter(|| black_box(search_entry.format()))
    });

    group.bench_function("format_read", |bencher| {
        bencher.iter(|| black_box(read_entry.format()))
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Prompt do Agente
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_agent_prompt(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_prompt");

    // Diferentes tamanhos de diário
    for diary_size in [0, 5, 10, 25, 50].iter() {
        let diary: Vec<DiaryEntry> = (0..*diary_size)
            .map(|i| {
                if i % 3 == 0 {
                    DiaryEntry::Search {
                        queries: vec![SerpQuery {
                            q: format!("query {}", i),
                            tbs: None,
                            location: None,
                        }],
                        think: format!("Search iteration {}", i),
                        urls_found: 10,
                    }
                } else if i % 3 == 1 {
                    DiaryEntry::Read {
                        urls: vec![format!("https://example.com/{}", i)],
                        think: format!("Read iteration {}", i),
                    }
                } else {
                    DiaryEntry::Reflect {
                        questions: vec![format!("Question from step {}", i)],
                        think: format!("Reflect iteration {}", i),
                    }
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("create_with_diary", diary_size),
            diary_size,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(AgentPrompt {
                        system: "You are a research assistant...".to_string(),
                        user: "Find information about Rust programming".to_string(),
                        diary: diary.clone(),
                    })
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Resultados
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_results(c: &mut Criterion) {
    let mut group = c.benchmark_group("results");

    group.bench_function("create_answer_result", |bencher| {
        bencher.iter(|| {
            black_box(AnswerResult {
                answer: "Comprehensive answer about Rust...".to_string(),
                references: vec![Reference {
                    url: "https://rust-lang.org".to_string(),
                    title: "Rust".to_string(),
                    exact_quote: None,
                    relevance_score: Some(0.95),
                    answer_chunk: Some("Comprehensive answer about Rust...".to_string()),
                    answer_position: Some((0, 35)),
                }],
                trivial: false,
            })
        })
    });

    group.bench_function("create_research_result_success", |bencher| {
        bencher.iter(|| {
            black_box(ResearchResult {
                success: true,
                answer: Some("Final answer...".to_string()),
                references: vec![],
                trivial: false,
                token_usage: TokenUsage {
                    prompt_tokens: 15000,
                    completion_tokens: 3000,
                    total_tokens: 18000,
                },
                visited_urls: vec![
                    "https://example.com/1".to_string(),
                    "https://example.com/2".to_string(),
                ],
                error: None,
                total_time_ms: 5000,
                search_time_ms: 1500,
                read_time_ms: 800,
                llm_time_ms: 2700,
            })
        })
    });

    group.bench_function("create_research_result_failure", |bencher| {
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
                error: Some("Budget exhausted".to_string()),
                total_time_ms: 30000,
                search_time_ms: 10000,
                read_time_ms: 5000,
                llm_time_ms: 15000,
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_agent_state,
    bench_state_checks,
    bench_agent_actions,
    bench_action_checks,
    bench_action_permissions,
    bench_diary_entry,
    bench_agent_prompt,
    bench_results,
);

criterion_main!(benches);
