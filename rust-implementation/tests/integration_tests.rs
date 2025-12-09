//! # Testes de Integração
//!
//! Este módulo contém testes de integração que validam o fluxo completo do sistema:
//! - Persona → Search: Personas geram queries que podem ser usadas em buscas
//! - Search → Evaluation: Resultados de busca podem ser avaliados
//! - Full Pipeline: Fluxo completo de pergunta até resposta com evidências

use deep_research::evaluation::{determine_required_evaluations, EvaluationType};
use deep_research::evidence::{
    EvaluationEvidence, EvaluationEvidenceReport, SearchEvidenceReport, SearchQueryEvidence,
    UrlEvidence,
};
use deep_research::personas::{
    PersonaEvidence, PersonaEvidenceReport, PersonaRegistry, QueryContext,
};
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// TESTE 1: Persona → Search
// Verifica que personas geram queries válidas que podem ser usadas em buscas
// ============================================================================

#[test]
fn test_persona_to_search_integration() {
    // 1. Criar contexto de query
    let execution_id = Uuid::new_v4();
    let original_query = "Quais são as melhores práticas de segurança em Rust?";

    let context = QueryContext {
        execution_id,
        original_query: original_query.to_string(),
        user_intent: "Find security best practices".to_string(),
        soundbites: vec![],
        current_date: chrono::Utc::now().date_naive(),
        detected_language: deep_research::types::Language::Portuguese,
        detected_topic: deep_research::types::TopicCategory::Technology,
    };

    // 2. Criar registry e obter personas
    let registry = PersonaRegistry::with_defaults();
    let personas_available = registry.count();

    assert!(personas_available > 0, "Registry should have personas");

    // 3. Expandir queries usando todas as personas
    let results = registry.expand_query_all(&context.original_query, &context);

    // 4. Verificar resultados
    assert!(!results.is_empty(), "Should generate at least one query");

    // 5. Criar relatório de evidências de busca simulando uso das queries
    let mut search_report = SearchEvidenceReport::new(execution_id);

    for weighted_query in &results {
        // Simular uso da query em busca
        let mut search_evidence =
            SearchQueryEvidence::new(weighted_query.query.clone(), "https://api.serper.dev/search");

        // Simular persona de origem
        search_evidence = search_evidence.with_persona(weighted_query.source_persona);

        // Simular conclusão da busca
        search_evidence.complete(200, 10, 5000);

        // Adicionar URLs encontradas
        search_evidence.add_url(UrlEvidence::new(
            format!("https://example.com/result-{}", weighted_query.query.q.len()),
            "example.com",
        ));

        search_report.add_query(search_evidence);
    }

    search_report.finalize();

    // 6. Verificações finais
    assert!(
        search_report.queries_sent.len() >= 1,
        "Should have at least 1 search query"
    );
    assert!(
        search_report.success_rate > 0.0,
        "Should have some successful searches"
    );
    assert!(
        search_report.total_urls_discovered > 0,
        "Should discover some URLs"
    );

    println!("✅ test_persona_to_search_integration PASSED");
    println!("   - Personas available: {}", personas_available);
    println!("   - Queries generated: {}", results.len());
    println!("   - URLs discovered: {}", search_report.total_urls_discovered);
}

// ============================================================================
// TESTE 2: Search → Evaluation
// Verifica que resultados de busca podem ser avaliados corretamente
// ============================================================================

#[test]
fn test_search_to_eval_integration() {
    // 1. Criar pergunta e determinar tipos de avaliação necessários
    let question = "What are the top 5 programming languages for web development in 2024?";
    let eval_types = determine_required_evaluations(question);

    // Deve precisar de: Definitive (sempre), Freshness (2024), Plurality (top 5)
    assert!(
        eval_types.contains(&EvaluationType::Definitive),
        "Should need Definitive"
    );
    assert!(
        eval_types.contains(&EvaluationType::Plurality),
        "Should need Plurality for 'top 5'"
    );

    // 2. Simular resposta gerada a partir de resultados de busca
    let answer = r#"
        Based on current trends and job market data, the top 5 programming languages 
        for web development in 2024 are:
        
        1. JavaScript - Remains the dominant language for frontend development
        2. TypeScript - Growing rapidly due to type safety benefits
        3. Python - Popular for backend with Django and FastAPI
        4. Rust - Emerging choice for performance-critical backends
        5. Go - Favored for microservices and APIs
        
        Each language has its strengths depending on the specific use case.
    "#;

    // 3. Criar relatório de evidências de avaliação
    let execution_id = Uuid::new_v4();
    let mut eval_report = EvaluationEvidenceReport::new(execution_id, question, answer.len());
    eval_report.set_required_evaluations(eval_types.clone());

    // 4. Simular execução de cada avaliação
    for eval_type in &eval_types {
        let mut evidence = EvaluationEvidence::new(*eval_type);

        // Simular geração de prompt
        evidence.prompt_generated(2000);

        // Simular chamada ao LLM
        evidence.llm_called(Duration::from_millis(300), 500);

        // Simular resultado (neste caso, passamos em tudo)
        // Em produção, isso viria do LLM
        let passed = true;
        let confidence = 0.9;
        evidence.set_result(passed, confidence, 150, 0);

        eval_report.add_evaluation(evidence);
    }

    eval_report.finalize();

    // 5. Verificações
    assert!(
        eval_report.final_verdict,
        "All evaluations should pass for good answer"
    );
    assert!(
        eval_report.evaluations_executed.len() >= 2,
        "Should have at least 2 evaluations"
    );
    assert!(
        eval_report.total_llm_tokens > 0,
        "Should have used LLM tokens"
    );

    println!("✅ test_search_to_eval_integration PASSED");
    println!("   - Evaluation types: {:?}", eval_types);
    println!("   - Final verdict: {}", eval_report.final_verdict);
    println!("   - Total tokens: {}", eval_report.total_llm_tokens);
}

// ============================================================================
// TESTE 3: Full Pipeline
// Verifica o fluxo completo: Question → Personas → Search → Evaluation → Answer
// ============================================================================

#[test]
fn test_full_pipeline_integration() {
    let execution_id = Uuid::new_v4();
    let question = "Explain the differences between async and sync programming in Rust";

    // =============================
    // FASE 1: Expansão de Queries (Personas)
    // =============================

    let context = QueryContext {
        execution_id,
        original_query: question.to_string(),
        user_intent: "Understand async vs sync differences".to_string(),
        soundbites: vec![],
        current_date: chrono::Utc::now().date_naive(),
        detected_language: deep_research::types::Language::English,
        detected_topic: deep_research::types::TopicCategory::Technology,
    };

    let registry = PersonaRegistry::with_defaults();
    let expanded_queries = registry.expand_query_all(question, &context);

    // Criar relatório de personas
    let mut persona_report = PersonaEvidenceReport::new(execution_id, question.to_string());

    for wq in &expanded_queries {
        // Criar evidência da persona a partir do WeightedQuery
        let evidence = PersonaEvidence {
            persona_name: wq.source_persona,
            focus: wq.source_persona, // Usando nome como focus para simplificar
            weight: wq.weight,
            input_received: question.to_string(),
            output_generated: wq.query.clone(),
            execution_time: Duration::from_micros(100), // Simulado
            was_applicable: true,
            input_tokens: question.len() / 4, // Estimativa
            memory_allocated: 0,              // Não medido
        };
        persona_report.add_evidence(evidence);
    }

    persona_report.finalize();

    assert!(persona_report.total_queries_generated > 0);

    // =============================
    // FASE 2: Busca (Simulada)
    // =============================

    let mut search_report = SearchEvidenceReport::new(execution_id);

    for (i, wq) in expanded_queries.iter().enumerate() {
        let mut evidence = SearchQueryEvidence::new(wq.query.clone(), "https://api.jina.ai/search");

        evidence = evidence.with_persona(wq.source_persona);

        // Simular busca bem sucedida
        evidence.complete(200, 8, 3000 + i * 100);

        // Adicionar algumas URLs
        for j in 0..3 {
            let url = UrlEvidence::new(
                format!("https://rust-lang.org/doc-{}-{}", i, j),
                "rust-lang.org",
            )
            .with_boosts(1.5, 1.2);
            evidence.add_url(url);
        }

        search_report.add_query(evidence);
    }

    search_report.finalize();

    assert!(search_report.success_rate >= 0.9);
    assert!(search_report.total_urls_discovered > 0);

    // =============================
    // FASE 3: Geração de Resposta (Simulada)
    // =============================

    let simulated_answer = r#"
        Async vs Sync Programming in Rust:
        
        **Synchronous Programming:**
        - Code executes sequentially, blocking until each operation completes
        - Simpler mental model and easier debugging
        - Can waste CPU time waiting for I/O operations
        
        **Asynchronous Programming:**
        - Uses async/await syntax introduced in Rust 1.39
        - Non-blocking operations allow concurrent execution
        - Requires a runtime like Tokio or async-std
        - Better resource utilization for I/O-bound applications
        
        **Key Differences:**
        1. Execution model: sync blocks, async suspends
        2. Performance: async scales better for I/O-heavy workloads
        3. Complexity: async adds state machine complexity
        4. Ecosystem: both have strong library support
        
        Choose sync for CPU-bound tasks and simpler code.
        Choose async for I/O-bound tasks and high concurrency needs.
    "#;

    // =============================
    // FASE 4: Avaliação
    // =============================

    let eval_types = determine_required_evaluations(question);

    let mut eval_report =
        EvaluationEvidenceReport::new(execution_id, question, simulated_answer.len());
    eval_report.set_required_evaluations(eval_types.clone());

    for eval_type in &eval_types {
        let mut evidence = EvaluationEvidence::new(*eval_type);
        evidence.prompt_generated(1800);
        evidence.llm_called(Duration::from_millis(250), 400);

        // A resposta simulada é boa, então passa em tudo
        evidence.set_result(true, 0.92, 120, 0);

        eval_report.add_evaluation(evidence);
    }

    eval_report.finalize();

    // =============================
    // VERIFICAÇÕES FINAIS
    // =============================

    // Verificar que todas as fases funcionaram
    assert!(
        persona_report.total_queries_generated > 0,
        "Personas should generate queries"
    );
    assert!(
        search_report.success_rate > 0.5,
        "Search should be mostly successful"
    );
    assert!(
        eval_report.final_verdict,
        "Good answer should pass evaluation"
    );

    // Verificar coerência de IDs
    assert_eq!(
        persona_report.execution_id, search_report.execution_id,
        "Same execution ID across phases"
    );
    assert_eq!(
        search_report.execution_id, eval_report.execution_id,
        "Same execution ID across phases"
    );

    println!("✅ test_full_pipeline_integration PASSED");
    println!("   ┌─────────────────────────────────────────────");
    println!("   │ PIPELINE COMPLETO");
    println!("   ├─────────────────────────────────────────────");
    println!(
        "   │ 1. Personas: {} queries geradas",
        persona_report.total_queries_generated
    );
    println!(
        "   │ 2. Busca: {} URLs descobertas ({:.0}% sucesso)",
        search_report.total_urls_discovered,
        search_report.success_rate * 100.0
    );
    println!(
        "   │ 3. Avaliação: {} tipos ({} total tokens)",
        eval_report.evaluations_executed.len(),
        eval_report.total_llm_tokens
    );
    println!(
        "   │ 4. Veredicto: {}",
        if eval_report.final_verdict {
            "APROVADO ✓"
        } else {
            "REJEITADO ✗"
        }
    );
    println!("   └─────────────────────────────────────────────");
}

// ============================================================================
// TESTE 4: Pipeline com Early-Fail
// Verifica que o sistema para cedo quando uma avaliação falha
// ============================================================================

#[test]
fn test_pipeline_early_fail_integration() {
    let execution_id = Uuid::new_v4();
    let question = "What is the current price of Bitcoin?";

    // Esta pergunta precisa de Freshness
    let eval_types = determine_required_evaluations(question);
    assert!(eval_types.contains(&EvaluationType::Freshness));

    // Simular resposta desatualizada
    let outdated_answer = "As of January 2023, Bitcoin is trading around $16,500.";

    let mut eval_report =
        EvaluationEvidenceReport::new(execution_id, question, outdated_answer.len());
    eval_report.set_required_evaluations(eval_types.clone());

    // Simular avaliações
    for eval_type in &eval_types {
        let mut evidence = EvaluationEvidence::new(*eval_type);
        evidence.prompt_generated(1500);
        evidence.llm_called(Duration::from_millis(200), 350);

        // Freshness vai falhar porque a resposta é antiga
        if *eval_type == EvaluationType::Freshness {
            evidence.set_result(false, 0.15, 80, 2);
        } else {
            evidence.set_result(true, 0.9, 100, 0);
        }

        let passed = evidence.result_passed;
        eval_report.add_evaluation(evidence);

        // Em produção, pararia aqui com early-fail
        if !passed {
            break;
        }
    }

    eval_report.finalize();

    // Verificar que falhou corretamente
    assert!(
        !eval_report.final_verdict,
        "Should fail due to outdated info"
    );
    assert!(
        eval_report.early_fail_reason.is_some(),
        "Should have early fail reason"
    );

    println!("✅ test_pipeline_early_fail_integration PASSED");
    println!(
        "   - Early fail reason: {}",
        eval_report.early_fail_reason.as_ref().unwrap()
    );
}

// ============================================================================
// TESTE 5: Persona Uniqueness
// Verifica que diferentes personas geram queries únicas
// ============================================================================

#[test]
fn test_persona_uniqueness_integration() {
    let execution_id = Uuid::new_v4();
    let question = "Best practices for microservices architecture";

    let context = QueryContext {
        execution_id,
        original_query: question.to_string(),
        user_intent: "Learn microservices best practices".to_string(),
        soundbites: vec![],
        current_date: chrono::Utc::now().date_naive(),
        detected_language: deep_research::types::Language::English,
        detected_topic: deep_research::types::TopicCategory::Technology,
    };

    let registry = PersonaRegistry::with_defaults();
    let queries = registry.expand_query_all(question, &context);

    // Coletar queries únicas
    let unique_queries: std::collections::HashSet<String> =
        queries.iter().map(|wq| wq.query.q.clone()).collect();

    // Verificar unicidade
    // Nota: algumas personas podem gerar queries similares em alguns casos,
    // mas devemos ter pelo menos algumas únicas
    let uniqueness_ratio = unique_queries.len() as f32 / queries.len() as f32;

    assert!(
        uniqueness_ratio >= 0.5,
        "At least 50% of queries should be unique, got {}%",
        uniqueness_ratio * 100.0
    );

    println!("✅ test_persona_uniqueness_integration PASSED");
    println!(
        "   - Total queries: {}, Unique: {} ({:.0}%)",
        queries.len(),
        unique_queries.len(),
        uniqueness_ratio * 100.0
    );
}

// ============================================================================
// TESTE 6: Evaluation Type Selection
// Verifica que o sistema seleciona corretamente os tipos de avaliação
// ============================================================================

#[test]
fn test_evaluation_type_selection_integration() {
    // Casos de teste com tipos esperados
    let test_cases = vec![
        // (pergunta, deve ter definitive, deve ter freshness, deve ter plurality, deve ter completeness)
        ("What is Rust?", true, false, false, false),
        ("Current price of gold", true, true, false, false),
        ("List 5 benefits of exercise", true, false, true, false),
        ("Compare Python and JavaScript", true, false, false, true),
        (
            "What are the latest 3 AI trends for 2025?",
            true,
            true,
            true,
            false,
        ),
    ];

    for (question, expect_def, expect_fresh, expect_plur, expect_comp) in test_cases {
        let types = determine_required_evaluations(question);

        if expect_def {
            assert!(
                types.contains(&EvaluationType::Definitive),
                "Q: '{}' should need Definitive",
                question
            );
        }
        if expect_fresh {
            assert!(
                types.contains(&EvaluationType::Freshness),
                "Q: '{}' should need Freshness",
                question
            );
        }
        if expect_plur {
            assert!(
                types.contains(&EvaluationType::Plurality),
                "Q: '{}' should need Plurality",
                question
            );
        }
        if expect_comp {
            assert!(
                types.contains(&EvaluationType::Completeness),
                "Q: '{}' should need Completeness",
                question
            );
        }
    }

    println!("✅ test_evaluation_type_selection_integration PASSED");
    println!("   - All 5 test cases validated correctly");
}

