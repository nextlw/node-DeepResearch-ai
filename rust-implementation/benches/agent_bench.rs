//! Benchmarks do Agente de Pesquisa.
//!
//! Testa performance de:
//! - Transições de estado
//! - Criação e manipulação de ações
//! - Contexto do agente
//! - Diário de execução
//! - Sistema de interação usuário-agente
//! - Sandbox multilinguagem (JavaScript/Python)
//!
//! Executar: `cargo bench --bench agent_bench`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use deep_research::agent::{
    ActionPermissions, AgentAction, AgentPrompt, AgentState, AnswerResult, DiaryEntry,
    InteractionHub, PendingQuestion, QuestionType, ResearchResult, SandboxContext,
    SandboxLanguage, TokenUsage, UserResponse,
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

    group.bench_function("create_coding_js", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::Coding {
                problem: "Processar array de dados JSON e dobrar cada valor numérico".to_string(),
                context_vars: None,
                language: Some("javascript".to_string()),
                think: "Processing data with JavaScript.".to_string(),
            })
        })
    });

    group.bench_function("create_coding_python", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::Coding {
                problem: "Calcular estatísticas descritivas: média, desvio padrão, percentis".to_string(),
                context_vars: Some(vec!["data".to_string(), "values".to_string()]),
                language: Some("python".to_string()),
                think: "Using Python for statistical analysis.".to_string(),
            })
        })
    });

    group.bench_function("create_coding_auto", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::Coding {
                problem: "Processar dados conforme necessário".to_string(),
                context_vars: None,
                language: None, // Auto - LLM escolhe
                think: "Let LLM decide the best language.".to_string(),
            })
        })
    });

    group.bench_function("create_ask_user", |bencher| {
        bencher.iter(|| {
            black_box(AgentAction::AskUser {
                question_type: QuestionType::Clarification,
                question: "Você poderia esclarecer qual período de tempo você gostaria que eu analisasse?".to_string(),
                options: Some(vec![
                    "Últimos 7 dias".to_string(),
                    "Último mês".to_string(),
                    "Último ano".to_string(),
                ]),
                is_blocking: true,
                think: "Need user input to proceed with analysis.".to_string(),
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Sistema de Interação Usuário-Agente
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_interaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("interaction");

    group.bench_function("create_pending_question_clarification", |bencher| {
        bencher.iter(|| {
            black_box(PendingQuestion::clarification(
                "Qual período você gostaria de analisar?",
                "Need to clarify time period",
            ))
        })
    });

    group.bench_function("create_pending_question_preference", |bencher| {
        bencher.iter(|| {
            black_box(PendingQuestion::preference(
                "Qual formato de relatório você prefere?",
                vec![
                    "PDF".to_string(),
                    "Excel".to_string(),
                    "CSV".to_string(),
                ],
                "Need user preference for output format",
            ))
        })
    });

    group.bench_function("create_user_response_to_question", |bencher| {
        bencher.iter(|| {
            black_box(UserResponse::to_question(
                "question-uuid-123",
                "Gostaria de analisar os últimos 7 dias",
            ))
        })
    });

    group.bench_function("create_user_response_spontaneous", |bencher| {
        bencher.iter(|| {
            black_box(UserResponse::spontaneous(
                "Também gostaria de saber sobre o crescimento mensal",
            ))
        })
    });

    group.bench_function("question_type_checks", |bencher| {
        let types = vec![
            QuestionType::Clarification,
            QuestionType::Confirmation,
            QuestionType::Preference,
            QuestionType::Suggestion,
        ];
        bencher.iter(|| {
            for qt in &types {
                black_box(qt.is_blocking_by_default());
            }
        })
    });

    group.bench_function("interaction_hub_create", |bencher| {
        bencher.iter(|| {
            black_box(InteractionHub::new())
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Sandbox Multilinguagem
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_sandbox(c: &mut Criterion) {
    let mut group = c.benchmark_group("sandbox");

    group.bench_function("sandbox_language_checks", |bencher| {
        let langs = [
            SandboxLanguage::JavaScript,
            SandboxLanguage::Python,
            SandboxLanguage::Auto,
        ];
        bencher.iter(|| {
            for lang in &langs {
                black_box((
                    lang.display_name(),
                    lang.extension(),
                ));
            }
        })
    });

    group.bench_function("sandbox_context_create_empty", |bencher| {
        bencher.iter(|| {
            black_box(SandboxContext::new())
        })
    });

    group.bench_function("sandbox_context_from_knowledge", |bencher| {
        let knowledge = vec![
            KnowledgeItem {
                question: "What is Rust?".to_string(),
                answer: "Rust is a systems programming language...".to_string(),
                item_type: KnowledgeType::Qa,
                references: vec![],
            },
            KnowledgeItem {
                question: "Sales data".to_string(),
                answer: "[100, 200, 300, 400, 500]".to_string(),
                item_type: KnowledgeType::UserProvided,
                references: vec![],
            },
        ];

        bencher.iter(|| {
            black_box(SandboxContext::from_knowledge(&knowledge))
        })
    });

    group.bench_function("sandbox_context_set_variable", |bencher| {
        bencher.iter(|| {
            let mut context = SandboxContext::new();
            context.set_variable("test_var", "[1, 2, 3, 4, 5]");
            black_box(context)
        })
    });

    group.bench_function("sandbox_context_describe_for_llm", |bencher| {
        let mut context = SandboxContext::new();
        context.set_variable("numbers", "[1, 2, 3, 4, 5]");
        context.set_variable("data", r#"{"key": "value"}"#);
        context.set_variable("text", "Hello world");

        bencher.iter(|| {
            black_box(context.describe_for_llm())
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Estados de InputRequired
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_input_required_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_required_state");

    group.bench_function("create_input_required", |bencher| {
        bencher.iter(|| {
            black_box(AgentState::InputRequired {
                question_id: "uuid-1234-5678".to_string(),
                question: "Qual período você gostaria de analisar?".to_string(),
                question_type: QuestionType::Clarification,
                options: Some(vec![
                    "Últimos 7 dias".to_string(),
                    "Último mês".to_string(),
                    "Último ano".to_string(),
                ]),
            })
        })
    });

    let input_required = AgentState::InputRequired {
        question_id: "test-id".to_string(),
        question: "Test question".to_string(),
        question_type: QuestionType::Confirmation,
        options: None,
    };

    let processing = AgentState::Processing {
        step: 5,
        total_step: 10,
        current_question: "Test".to_string(),
        budget_used: 0.3,
    };

    group.bench_function("is_input_required", |bencher| {
        bencher.iter(|| {
            black_box((
                input_required.is_input_required(),
                processing.is_input_required(),
            ))
        })
    });

    group.bench_function("can_transition_from_input_required", |bencher| {
        let completed = AgentState::Completed {
            answer: "Answer".to_string(),
            references: vec![],
            trivial: false,
        };
        bencher.iter(|| {
            black_box((
                input_required.can_transition_to(&processing),
                input_required.can_transition_to(&completed),
            ))
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
    bench_interaction,
    bench_sandbox,
    bench_input_required_state,
);

criterion_main!(benches);
