# DeepResearch em Rust: Implementação Detalhada

Este documento detalha como cada componente do DeepResearch seria implementado em Rust, explorando os padrões idiomáticos da linguagem e as otimizações de performance.

---

## 1. MÁQUINA DE ESTADOS (State Machine)

### O Problema em TypeScript

No código atual (`agent.ts:580-730`), a máquina de estados é implementada com strings e condicionais:

```typescript
// TypeScript atual - problemas:
// 1. thisStep.action é uma string - pode ter typos
// 2. Não há garantia em compile-time de cobrir todos os casos
// 3. Estado mutável espalhado (allowSearch, allowRead, etc.)

if (thisStep.action === "answer" && thisStep.answer) { ... }
else if (thisStep.action === "reflect") { ... }
else if (thisStep.action === "search") { ... }
```

### Solução em Rust: Enums com Dados Associados

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEFINIÇÃO DOS TIPOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cada ação carrega seus próprios dados - impossível ter ação "Search" sem queries
#[derive(Debug, Clone)]
pub enum AgentAction {
    Search {
        queries: Vec<SerpQuery>,
        think: String,
    },
    Read {
        urls: Vec<Url>,
        think: String,
    },
    Reflect {
        gap_questions: Vec<String>,
        think: String,
    },
    Answer {
        answer: String,
        references: Vec<Reference>,
        think: String,
    },
    Coding {
        code: String,
        think: String,
    },
}

/// Estado das permissões - imutável, criado a cada iteração
#[derive(Debug, Clone, Copy)]
pub struct ActionPermissions {
    pub search: bool,
    pub read: bool,
    pub reflect: bool,
    pub answer: bool,
    pub coding: bool,
}

impl ActionPermissions {
    /// Cria permissões baseadas no contexto atual
    pub fn from_context(ctx: &AgentContext) -> Self {
        Self {
            search: ctx.collected_urls.len() < 50,
            read: !ctx.weighted_urls.is_empty(),
            reflect: ctx.gap_questions.len() <= MAX_REFLECT_PER_STEP,
            answer: ctx.step > 1 || ctx.allow_direct_answer,
            coding: ctx.coding_enabled,
        }
    }

    /// Lista de ações permitidas (para logging/debug)
    pub fn allowed_actions(&self) -> Vec<&'static str> {
        let mut actions = Vec::with_capacity(5);
        if self.search { actions.push("search"); }
        if self.read { actions.push("read"); }
        if self.reflect { actions.push("reflect"); }
        if self.answer { actions.push("answer"); }
        if self.coding { actions.push("coding"); }
        actions
    }
}

/// Resultado de uma avaliação de resposta
#[derive(Debug)]
pub enum EvaluationResult {
    Passed,
    Failed {
        eval_type: EvaluationType,
        reason: String,
        suggestions: Vec<String>,
    },
}

/// Estado do agente - transições explícitas
#[derive(Debug)]
pub enum AgentState {
    /// Processando normalmente
    Processing {
        step: u32,
        total_step: u32,
        current_question: String,
        budget_used: f64,
    },
    /// Modo de emergência - forçar resposta
    BeastMode {
        attempts: u32,
        last_failure: String,
    },
    /// Pesquisa concluída com sucesso
    Completed {
        answer: String,
        references: Vec<Reference>,
        trivial: bool,
    },
    /// Falha - budget esgotado sem resposta
    Failed {
        reason: String,
        partial_knowledge: Vec<KnowledgeItem>,
    },
}
```

### A Máquina de Estados Principal

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO DA MÁQUINA DE ESTADOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct DeepResearchAgent {
    state: AgentState,
    context: AgentContext,
    llm_client: Box<dyn LlmClient>,
    search_client: Box<dyn SearchClient>,
    token_tracker: TokenTracker,
}

impl DeepResearchAgent {
    /// Loop principal - consome self e retorna resultado final
    pub async fn run(mut self, question: String) -> ResearchResult {
        // Inicialização
        self.context.original_question = question.clone();
        self.context.gap_questions.push(question);

        // Loop principal com pattern matching exaustivo
        loop {
            match &self.state {
                AgentState::Processing { budget_used, .. } if *budget_used >= 0.85 => {
                    // Transição para Beast Mode
                    self.state = AgentState::BeastMode {
                        attempts: 0,
                        last_failure: "Budget exhausted".into(),
                    };
                }

                AgentState::Processing { .. } => {
                    // Executar um passo normal
                    match self.execute_step().await {
                        StepResult::Continue => continue,
                        StepResult::Completed(answer) => {
                            self.state = AgentState::Completed {
                                answer: answer.answer,
                                references: answer.references,
                                trivial: answer.trivial,
                            };
                        }
                        StepResult::Error(e) => {
                            log::error!("Step error: {}", e);
                            continue; // Tentar novamente
                        }
                    }
                }

                AgentState::BeastMode { attempts, .. } if *attempts >= 3 => {
                    // Falha definitiva
                    self.state = AgentState::Failed {
                        reason: "Max beast mode attempts reached".into(),
                        partial_knowledge: self.context.knowledge.clone(),
                    };
                }

                AgentState::BeastMode { attempts, .. } => {
                    // Tentar forçar resposta
                    match self.force_answer().await {
                        Ok(answer) => {
                            self.state = AgentState::Completed {
                                answer: answer.answer,
                                references: answer.references,
                                trivial: false,
                            };
                        }
                        Err(_) => {
                            if let AgentState::BeastMode { attempts, last_failure } = &mut self.state {
                                *attempts += 1;
                            }
                        }
                    }
                }

                // Estados terminais - sair do loop
                AgentState::Completed { .. } | AgentState::Failed { .. } => break,
            }
        }

        // Construir resultado final
        self.build_result()
    }

    /// Executa um único passo do agente
    async fn execute_step(&mut self) -> StepResult {
        // 1. Calcular permissões baseadas no contexto atual
        let permissions = ActionPermissions::from_context(&self.context);

        // 2. Rotacionar para próxima pergunta
        let current_question = self.rotate_question();

        // 3. Gerar prompt e obter decisão do LLM
        let prompt = self.build_prompt(&permissions, &current_question);
        let action = self.llm_client.decide_action(&prompt, &permissions).await?;

        log::debug!(
            "Step {} | Action: {:?} <- [{}]",
            self.context.total_step,
            action,
            permissions.allowed_actions().join(", ")
        );

        // 4. Executar ação escolhida - pattern matching garante cobertura total
        match action {
            AgentAction::Search { queries, think } => {
                self.execute_search(queries, think).await
            }
            AgentAction::Read { urls, think } => {
                self.execute_read(urls, think).await
            }
            AgentAction::Reflect { gap_questions, think } => {
                self.execute_reflect(gap_questions, think).await
            }
            AgentAction::Answer { answer, references, think } => {
                self.execute_answer(answer, references, think).await
            }
            AgentAction::Coding { code, think } => {
                self.execute_coding(code, think).await
            }
        }
    }

    /// Executa ação de busca
    async fn execute_search(&mut self, queries: Vec<SerpQuery>, think: String) -> StepResult {
        // Expandir queries com personas cognitivas
        let expanded = self.expand_queries_with_personas(&queries).await?;

        // Deduplicar contra queries existentes
        let unique = self.dedup_queries(expanded).await?;

        // Executar buscas em paralelo
        let results: Vec<SearchResult> = futures::future::join_all(
            unique.iter().map(|q| self.search_client.search(q))
        ).await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

        // Adicionar URLs ao contexto
        for result in results {
            self.context.add_urls(result.urls);
            self.context.add_knowledge(result.snippets);
        }

        // Registrar no diário
        self.context.diary.push(DiaryEntry::Search {
            queries: unique,
            think,
            urls_found: self.context.collected_urls.len(),
        });

        StepResult::Continue
    }

    /// Executa avaliação de resposta
    async fn execute_answer(
        &mut self,
        answer: String,
        references: Vec<Reference>,
        think: String
    ) -> StepResult {
        // Resposta imediata no step 1 = pergunta trivial
        if self.context.total_step == 1 && self.context.allow_direct_answer {
            return StepResult::Completed(AnswerResult {
                answer,
                references,
                trivial: true,
            });
        }

        // Obter tipos de avaliação necessários
        let eval_types = self.get_evaluation_types(&self.context.current_question).await?;

        // Executar avaliações em sequência (falha rápida)
        for eval_type in eval_types {
            match self.evaluate(&answer, eval_type).await? {
                EvaluationResult::Passed => continue,
                EvaluationResult::Failed { eval_type, reason, suggestions } => {
                    // Adicionar falha como conhecimento
                    self.context.knowledge.push(KnowledgeItem {
                        question: self.context.current_question.clone(),
                        answer: format!("FAILED {}: {}", eval_type, reason),
                        item_type: KnowledgeType::Error,
                        suggestions,
                    });

                    self.context.diary.push(DiaryEntry::FailedAnswer {
                        answer: answer.clone(),
                        eval_type,
                        reason,
                    });

                    return StepResult::Continue;
                }
            }
        }

        // Todas avaliações passaram
        StepResult::Completed(AnswerResult {
            answer,
            references,
            trivial: false,
        })
    }
}
```

### Vantagens desta Abordagem

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// POR QUE RUST É MELHOR AQUI?
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// 1. EXAUSTIVIDADE GARANTIDA EM COMPILE-TIME
//    Se adicionar nova ação, o compilador FORÇA você a tratar todos os casos
match action {
    AgentAction::Search { .. } => { /* ... */ }
    AgentAction::Read { .. } => { /* ... */ }
    // ERRO DE COMPILAÇÃO se faltar algum caso!
}

// 2. DADOS ASSOCIADOS À AÇÃO
//    Impossível ter ação "Search" sem queries - o tipo garante
AgentAction::Search {
    queries,  // Vec<SerpQuery> - sempre presente
    think,    // String - sempre presente
}

// 3. TRANSIÇÕES DE ESTADO EXPLÍCITAS
//    Não existe "estado inválido" - apenas as transições definidas
impl AgentState {
    pub fn can_transition_to(&self, target: &AgentState) -> bool {
        matches!(
            (self, target),
            (AgentState::Processing { .. }, AgentState::BeastMode { .. }) |
            (AgentState::Processing { .. }, AgentState::Completed { .. }) |
            (AgentState::BeastMode { .. }, AgentState::Completed { .. }) |
            (AgentState::BeastMode { .. }, AgentState::Failed { .. })
        )
    }
}

// 4. ZERO-COST ABSTRACTIONS
//    Enums em Rust são "tagged unions" - tão eficientes quanto C
//    Tamanho = max(variantes) + tag (1 byte geralmente)
assert_eq!(std::mem::size_of::<AgentAction>(), 72); // Exemplo
```

---

## 2. PERSONAS COGNITIVAS (Query Expansion System)

### O Problema em TypeScript

No código atual (`query-rewriter.ts:33-44`), as personas são definidas em texto no prompt:

```typescript
// TypeScript atual - problemas:
// 1. Personas são strings em um prompt - não verificáveis
// 2. Cada persona gera uma query async separada
// 3. Não há paralelismo real (Promise.all em single-thread)
// 4. Output não tipado - pode faltar campos

const queryPromises = action.searchRequests.map(async (req) => {
  const prompt = getPrompt(req, action.think, context);
  const result = await generator.generateObject({...});
  return result.object.queries;
});
const queryResults = await Promise.all(queryPromises);
```

### Solução em Rust: Traits + Paralelismo Real

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SISTEMA DE PERSONAS COGNITIVAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use rayon::prelude::*;
use chrono::{Utc, Datelike};

/// Trait que define o comportamento de uma persona cognitiva
pub trait CognitivePersona: Send + Sync {
    /// Nome da persona para logging
    fn name(&self) -> &'static str;

    /// Descrição do foco desta persona
    fn focus(&self) -> &'static str;

    /// Gera uma query expandida a partir da query original
    fn expand_query(&self, original: &str, context: &QueryContext) -> SerpQuery;

    /// Peso desta persona no ranking final (0.0 - 1.0)
    fn weight(&self) -> f32 {
        1.0
    }
}

/// Contexto compartilhado para expansão de queries
#[derive(Debug, Clone)]
pub struct QueryContext {
    pub original_query: String,
    pub user_intent: String,
    pub soundbites: Vec<String>,  // Snippets de contexto
    pub current_date: chrono::NaiveDate,
    pub detected_language: Language,
    pub detected_topic: TopicCategory,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO DAS 7 PERSONAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 1. Expert Skeptic - Busca problemas, limitações, contra-evidências
pub struct ExpertSkeptic;

impl CognitivePersona for ExpertSkeptic {
    fn name(&self) -> &'static str { "Expert Skeptic" }

    fn focus(&self) -> &'static str {
        "edge cases, limitations, counter-evidence, potential failures"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        // Extrai o tópico principal e adiciona termos de ceticismo
        let skeptic_terms = ["problems", "issues", "failures", "limitations", "drawbacks"];
        let topic = extract_main_topic(original);

        SerpQuery {
            q: format!("{} {} {}", topic, skeptic_terms.choose(&mut rand::thread_rng()).unwrap(), "real experiences"),
            tbs: None,  // Sem filtro de tempo
            location: None,
        }
    }
}

/// 2. Detail Analyst - Especificações técnicas precisas
pub struct DetailAnalyst;

impl CognitivePersona for DetailAnalyst {
    fn name(&self) -> &'static str { "Detail Analyst" }

    fn focus(&self) -> &'static str {
        "precise specifications, technical details, exact parameters"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);

        SerpQuery {
            q: format!("{} specifications technical details comparison", topic),
            tbs: None,
            location: None,
        }
    }
}

/// 3. Historical Researcher - Evolução e contexto histórico
pub struct HistoricalResearcher;

impl CognitivePersona for HistoricalResearcher {
    fn name(&self) -> &'static str { "Historical Researcher" }

    fn focus(&self) -> &'static str {
        "evolution over time, previous iterations, historical context"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);
        let year = ctx.current_date.year();

        SerpQuery {
            q: format!("{} history evolution {} changes", topic, year - 5),
            tbs: Some("qdr:y".into()),  // Último ano
            location: None,
        }
    }
}

/// 4. Comparative Thinker - Alternativas e trade-offs
pub struct ComparativeThinker;

impl CognitivePersona for ComparativeThinker {
    fn name(&self) -> &'static str { "Comparative Thinker" }

    fn focus(&self) -> &'static str {
        "alternatives, competitors, contrasts, trade-offs"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);

        SerpQuery {
            q: format!("{} vs alternatives comparison pros cons", topic),
            tbs: None,
            location: None,
        }
    }
}

/// 5. Temporal Context - Informações recentes com data atual
pub struct TemporalContext;

impl CognitivePersona for TemporalContext {
    fn name(&self) -> &'static str { "Temporal Context" }

    fn focus(&self) -> &'static str {
        "time-sensitive queries, recency, current state"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);
        let year = ctx.current_date.year();
        let month = ctx.current_date.month();

        SerpQuery {
            q: format!("{} {} {}", topic, year, month),
            tbs: Some("qdr:m".into()),  // Último mês
            location: None,
        }
    }

    fn weight(&self) -> f32 {
        1.2  // Peso maior para informações recentes
    }
}

/// 6. Globalizer - Fontes no idioma mais autoritativo
pub struct Globalizer;

impl CognitivePersona for Globalizer {
    fn name(&self) -> &'static str { "Globalizer" }

    fn focus(&self) -> &'static str {
        "authoritative language/region for the subject matter"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        // Mapeia tópico para idioma autoritativo
        let (query, location) = match ctx.detected_topic {
            TopicCategory::Automotive(brand) => match brand.as_str() {
                "BMW" | "Mercedes" | "Audi" | "Volkswagen" => {
                    (translate_to_german(original), Some("Germany"))
                }
                "Toyota" | "Honda" | "Nissan" => {
                    (translate_to_japanese(original), Some("Japan"))
                }
                _ => (original.to_string(), None)
            },
            TopicCategory::Technology => {
                (original.to_string(), Some("San Francisco"))
            },
            TopicCategory::Cuisine(cuisine) => match cuisine.as_str() {
                "Italian" => (translate_to_italian(original), Some("Italy")),
                "French" => (translate_to_french(original), Some("France")),
                "Japanese" => (translate_to_japanese(original), Some("Japan")),
                _ => (original.to_string(), None)
            },
            _ => (original.to_string(), None)
        };

        SerpQuery {
            q: query,
            tbs: None,
            location: location.map(String::from),
        }
    }
}

/// 7. Reality Skepticalist - Contradições e evidências contrárias
pub struct RealitySkepticalist;

impl CognitivePersona for RealitySkepticalist {
    fn name(&self) -> &'static str { "Reality Skepticalist" }

    fn focus(&self) -> &'static str {
        "contradicting evidence, disprove assumptions, contrary perspectives"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);

        // Inverte a premissa da busca
        let negated = negate_assumption(original);

        SerpQuery {
            q: format!("{} wrong myth debunked evidence against", negated),
            tbs: None,
            location: None,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ORQUESTRADOR DE PERSONAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct PersonaOrchestrator {
    personas: Vec<Box<dyn CognitivePersona>>,
}

impl PersonaOrchestrator {
    pub fn new() -> Self {
        Self {
            personas: vec![
                Box::new(ExpertSkeptic),
                Box::new(DetailAnalyst),
                Box::new(HistoricalResearcher),
                Box::new(ComparativeThinker),
                Box::new(TemporalContext),
                Box::new(Globalizer),
                Box::new(RealitySkepticalist),
            ],
        }
    }

    /// Expande uma query usando TODAS as personas em PARALELO
    pub fn expand_query_parallel(&self, original: &str, context: &QueryContext) -> Vec<WeightedQuery> {
        // Rayon: paralelismo real em múltiplos cores
        self.personas
            .par_iter()  // Iterator paralelo!
            .map(|persona| {
                let query = persona.expand_query(original, context);
                WeightedQuery {
                    query,
                    weight: persona.weight(),
                    source_persona: persona.name(),
                }
            })
            .collect()
    }

    /// Expande múltiplas queries de entrada
    pub fn expand_batch(&self, queries: &[String], context: &QueryContext) -> Vec<WeightedQuery> {
        queries
            .par_iter()  // Paralelo no nível das queries
            .flat_map(|q| self.expand_query_parallel(q, context))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct WeightedQuery {
    pub query: SerpQuery,
    pub weight: f32,
    pub source_persona: &'static str,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// EXEMPLO DE USO
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn example_usage() {
    let orchestrator = PersonaOrchestrator::new();

    let context = QueryContext {
        original_query: "BMW used car price".into(),
        user_intent: "Want to buy a used BMW but worried about costs".into(),
        soundbites: vec!["maintenance costs".into(), "reliability".into()],
        current_date: Utc::now().date_naive(),
        detected_language: Language::English,
        detected_topic: TopicCategory::Automotive("BMW".into()),
    };

    // Expande em paralelo usando todos os cores
    let expanded = orchestrator.expand_query_parallel("BMW used car price", &context);

    // Resultado: 7 queries de perspectivas diferentes
    for wq in &expanded {
        println!("[{}] {} (weight: {})", wq.source_persona, wq.query.q, wq.weight);
    }
}
```

### Vantagens sobre TypeScript

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// COMPARAÇÃO DE PERFORMANCE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// TypeScript: Promise.all NÃO é paralelismo de CPU
// - Todas as promises rodam na mesma thread
// - Útil apenas para I/O concorrente
// - 7 personas = 7 execuções SEQUENCIAIS de CPU

// Rust com Rayon: Paralelismo REAL
// - Cada persona pode rodar em uma thread diferente
// - 8 cores = até 8x mais rápido para CPU-bound work
// - Work-stealing scheduler otimiza automaticamente

// Benchmark aproximado (expandir 100 queries):
// TypeScript: ~500ms (single-threaded)
// Rust + Rayon (8 cores): ~70ms

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENEFÍCIOS DO TRAIT SYSTEM
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// 1. EXTENSIBILIDADE
//    Adicionar nova persona = implementar trait, adicionar ao vec
impl CognitivePersona for CustomPersona {
    fn name(&self) -> &'static str { "Custom" }
    fn focus(&self) -> &'static str { "custom focus" }
    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        // Lógica customizada
    }
}

// 2. TESTABILIDADE
//    Cada persona pode ser testada isoladamente
#[test]
fn test_skeptic_adds_negative_terms() {
    let skeptic = ExpertSkeptic;
    let ctx = QueryContext::default();
    let query = skeptic.expand_query("BMW X5", &ctx);
    assert!(query.q.contains("problems") || query.q.contains("issues"));
}

// 3. COMPOSIÇÃO
//    Podemos criar grupos de personas para diferentes domínios
pub fn get_technical_personas() -> Vec<Box<dyn CognitivePersona>> {
    vec![
        Box::new(DetailAnalyst),
        Box::new(ComparativeThinker),
        Box::new(TemporalContext),
    ]
}

pub fn get_investigative_personas() -> Vec<Box<dyn CognitivePersona>> {
    vec![
        Box::new(ExpertSkeptic),
        Box::new(RealitySkepticalist),
        Box::new(HistoricalResearcher),
    ]
}
```

---

## 3. AVALIAÇÃO MULTIDIMENSIONAL (Multi-Dimensional Evaluation)

### O Problema em TypeScript

No código atual (`evaluator.ts:633-672`), as avaliações são sequenciais com tipos como strings:

```typescript
// TypeScript atual - problemas:
// 1. evaluationType é string - pode ter typos
// 2. Switch/case não garante cobertura exaustiva
// 3. Cada avaliação cria novo prompt/schema em runtime
// 4. Não há garantia de ordem das avaliações

for (const evaluationType of evaluationTypes) {
  let prompt: { system: string; user: string } | undefined
  switch (evaluationType) {
    case 'definitive':
      prompt = getDefinitivePrompt(question, action.answer);
      break;
    case 'freshness':
      prompt = getFreshnessPrompt(question, action, new Date().toISOString());
      break;
    // ... pode esquecer de adicionar novos casos
  }
}
```

### Solução em Rust: Type-State Pattern + Builder

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SISTEMA DE AVALIAÇÃO MULTIDIMENSIONAL
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::time::Duration;

/// Tipos de avaliação - enum garante que não existem tipos "inventados"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvaluationType {
    Definitive,   // Resposta é confiante? Sem "talvez"?
    Freshness,    // Informação é recente o suficiente?
    Plurality,    // Quantidade correta de exemplos?
    Completeness, // Todos os aspectos cobertos?
    Strict,       // Avaliação brutal - insights reais?
}

/// Configuração específica para cada tipo de avaliação
#[derive(Debug, Clone)]
pub struct EvaluationConfig {
    pub eval_type: EvaluationType,
    pub max_retries: u8,
    pub timeout: Duration,
    pub weight: f32,  // Importância relativa
}

impl EvaluationType {
    /// Retorna configuração padrão para cada tipo
    pub fn default_config(self) -> EvaluationConfig {
        match self {
            Self::Definitive => EvaluationConfig {
                eval_type: self,
                max_retries: 2,
                timeout: Duration::from_secs(30),
                weight: 1.0,
            },
            Self::Freshness => EvaluationConfig {
                eval_type: self,
                max_retries: 1,
                timeout: Duration::from_secs(20),
                weight: 0.8,
            },
            Self::Plurality => EvaluationConfig {
                eval_type: self,
                max_retries: 1,
                timeout: Duration::from_secs(15),
                weight: 0.6,
            },
            Self::Completeness => EvaluationConfig {
                eval_type: self,
                max_retries: 2,
                timeout: Duration::from_secs(25),
                weight: 0.9,
            },
            Self::Strict => EvaluationConfig {
                eval_type: self,
                max_retries: 3,
                timeout: Duration::from_secs(45),
                weight: 1.5,  // Mais importante
            },
        }
    }

    /// Determina freshness threshold baseado no tópico
    pub fn freshness_threshold(&self, topic: &TopicCategory) -> Duration {
        match topic {
            TopicCategory::Finance => Duration::from_secs(60 * 60 * 2),      // 2 horas
            TopicCategory::News => Duration::from_secs(60 * 60 * 24),        // 1 dia
            TopicCategory::Technology => Duration::from_secs(60 * 60 * 24 * 30), // 30 dias
            TopicCategory::Science => Duration::from_secs(60 * 60 * 24 * 365),   // 1 ano
            TopicCategory::History => Duration::MAX,                          // Sem limite
            _ => Duration::from_secs(60 * 60 * 24 * 7),                       // 7 dias padrão
        }
    }
}

/// Resultado de uma avaliação individual
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub eval_type: EvaluationType,
    pub passed: bool,
    pub confidence: f32,        // 0.0 - 1.0
    pub reasoning: String,
    pub suggestions: Vec<String>,
    pub duration: Duration,
}

/// Trait para implementar avaliadores customizados
pub trait Evaluator: Send + Sync {
    fn eval_type(&self) -> EvaluationType;

    fn evaluate(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> impl std::future::Future<Output = Result<EvaluationResult, EvalError>> + Send;

    fn generate_prompt(&self, question: &str, answer: &str) -> PromptPair;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO DOS AVALIADORES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Avaliador de Definitividade - A resposta é confiante?
pub struct DefinitiveEvaluator {
    config: EvaluationConfig,
    llm: Arc<dyn LlmClient>,
}

impl Evaluator for DefinitiveEvaluator {
    fn eval_type(&self) -> EvaluationType {
        EvaluationType::Definitive
    }

    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvalError> {
        let start = std::time::Instant::now();
        let prompt = self.generate_prompt(question, answer);

        let response = self.llm
            .generate_structured::<DefinitiveResponse>(&prompt)
            .await?;

        Ok(EvaluationResult {
            eval_type: self.eval_type(),
            passed: response.is_definitive && response.confidence > 0.7,
            confidence: response.confidence,
            reasoning: response.reasoning,
            suggestions: if response.is_definitive {
                vec![]
            } else {
                vec![
                    "Remove hedging language like 'maybe', 'probably', 'might'".into(),
                    "Provide concrete facts instead of vague statements".into(),
                ]
            },
            duration: start.elapsed(),
        })
    }

    fn generate_prompt(&self, question: &str, answer: &str) -> PromptPair {
        PromptPair {
            system: r#"
You are an evaluator checking if an answer is DEFINITIVE.
A definitive answer:
- States facts confidently without excessive hedging
- Does not use phrases like "I think", "maybe", "probably", "might be"
- Provides concrete information rather than vague generalities
- Acknowledges uncertainty only when genuinely uncertain, not as a habit

Evaluate the answer and respond with:
- is_definitive: boolean
- confidence: float 0-1
- reasoning: string explaining your evaluation
"#.into(),
            user: format!(
                "Question: {}\n\nAnswer to evaluate:\n{}",
                question, answer
            ),
        }
    }
}

/// Avaliador de Freshness - Informação é recente?
pub struct FreshnessEvaluator {
    config: EvaluationConfig,
    llm: Arc<dyn LlmClient>,
}

impl Evaluator for FreshnessEvaluator {
    fn eval_type(&self) -> EvaluationType {
        EvaluationType::Freshness
    }

    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvalError> {
        let start = std::time::Instant::now();

        // Determina threshold baseado no tópico
        let threshold = self.eval_type().freshness_threshold(&context.topic);
        let current_date = chrono::Utc::now();

        let prompt = PromptPair {
            system: format!(r#"
You are evaluating if an answer contains sufficiently RECENT information.
Current date: {}
Topic category: {:?}
Required freshness: information should not be older than {} days

Check if:
1. The answer mentions dates/timeframes that are recent enough
2. The information reflects current state (not outdated)
3. For time-sensitive topics, data is from recent sources
"#,
                current_date.format("%Y-%m-%d"),
                context.topic,
                threshold.as_secs() / 86400
            ),
            user: format!(
                "Question: {}\n\nAnswer to evaluate:\n{}",
                question, answer
            ),
        };

        let response = self.llm
            .generate_structured::<FreshnessResponse>(&prompt)
            .await?;

        Ok(EvaluationResult {
            eval_type: self.eval_type(),
            passed: response.is_fresh,
            confidence: response.confidence,
            reasoning: response.reasoning,
            suggestions: if response.is_fresh {
                vec![]
            } else {
                vec![
                    format!("Information appears to be from {}. Need more recent data.",
                            response.detected_date.unwrap_or("unknown date".into())),
                    "Search for sources from the last month".into(),
                ]
            },
            duration: start.elapsed(),
        })
    }

    fn generate_prompt(&self, question: &str, answer: &str) -> PromptPair {
        // Implementação básica - a real está em evaluate()
        PromptPair {
            system: "Freshness evaluator".into(),
            user: format!("{}\n{}", question, answer),
        }
    }
}

/// Avaliador de Plurality - Quantidade correta de itens?
pub struct PluralityEvaluator {
    config: EvaluationConfig,
    llm: Arc<dyn LlmClient>,
}

impl Evaluator for PluralityEvaluator {
    fn eval_type(&self) -> EvaluationType {
        EvaluationType::Plurality
    }

    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvalError> {
        let start = std::time::Instant::now();

        // Extrai número esperado da pergunta
        let expected_count = extract_expected_count(question);

        let prompt = self.generate_prompt(question, answer);
        let response = self.llm
            .generate_structured::<PluralityResponse>(&prompt)
            .await?;

        let passed = match expected_count {
            Some(expected) => response.item_count >= expected,
            None => true, // Se não há número específico, passa
        };

        Ok(EvaluationResult {
            eval_type: self.eval_type(),
            passed,
            confidence: response.confidence,
            reasoning: format!(
                "Expected {} items, found {}. {}",
                expected_count.map(|n| n.to_string()).unwrap_or("unspecified".into()),
                response.item_count,
                response.reasoning
            ),
            suggestions: if passed {
                vec![]
            } else {
                vec![format!(
                    "Need {} more examples/items",
                    expected_count.unwrap_or(0).saturating_sub(response.item_count)
                )]
            },
            duration: start.elapsed(),
        })
    }

    fn generate_prompt(&self, question: &str, answer: &str) -> PromptPair {
        PromptPair {
            system: r#"
Count the number of distinct items/examples in the answer.
If the question asks for a specific number (e.g., "5 examples", "top 10"),
verify the answer provides at least that many.

Respond with:
- item_count: integer
- confidence: float 0-1
- reasoning: string
"#.into(),
            user: format!(
                "Question: {}\n\nAnswer to evaluate:\n{}",
                question, answer
            ),
        }
    }
}

/// Avaliador de Completeness - Todos aspectos cobertos?
pub struct CompletenessEvaluator {
    config: EvaluationConfig,
    llm: Arc<dyn LlmClient>,
}

impl Evaluator for CompletenessEvaluator {
    fn eval_type(&self) -> EvaluationType {
        EvaluationType::Completeness
    }

    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvalError> {
        let start = std::time::Instant::now();

        // Extrai aspectos mencionados na pergunta
        let expected_aspects = extract_question_aspects(question);

        let prompt = PromptPair {
            system: format!(r#"
Evaluate if the answer addresses ALL aspects of the question.

The question appears to ask about these aspects:
{}

Check if each aspect is adequately addressed in the answer.
"#,
                expected_aspects.iter()
                    .enumerate()
                    .map(|(i, a)| format!("{}. {}", i + 1, a))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            user: format!(
                "Question: {}\n\nAnswer to evaluate:\n{}",
                question, answer
            ),
        };

        let response = self.llm
            .generate_structured::<CompletenessResponse>(&prompt)
            .await?;

        let passed = response.coverage_ratio >= 0.8; // 80% coverage

        Ok(EvaluationResult {
            eval_type: self.eval_type(),
            passed,
            confidence: response.confidence,
            reasoning: response.reasoning,
            suggestions: response.missing_aspects
                .iter()
                .map(|aspect| format!("Address missing aspect: {}", aspect))
                .collect(),
            duration: start.elapsed(),
        })
    }

    fn generate_prompt(&self, question: &str, answer: &str) -> PromptPair {
        PromptPair {
            system: "Completeness evaluator".into(),
            user: format!("{}\n{}", question, answer),
        }
    }
}

/// Avaliador Strict - Avaliação brutal, insights reais?
pub struct StrictEvaluator {
    config: EvaluationConfig,
    llm: Arc<dyn LlmClient>,
}

impl Evaluator for StrictEvaluator {
    fn eval_type(&self) -> EvaluationType {
        EvaluationType::Strict
    }

    async fn evaluate(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvalError> {
        let start = std::time::Instant::now();

        let prompt = PromptPair {
            system: r#"
You are a BRUTAL evaluator. Your job is to REJECT mediocre answers.

An answer ONLY passes if it demonstrates:
1. DEPTH: Goes beyond surface-level information
2. INSIGHT: Provides non-obvious analysis or connections
3. SPECIFICITY: Includes concrete examples, numbers, or evidence
4. COMPLETENESS: Addresses the full scope of the question
5. ACCURACY: No factual errors or misleading statements

Be harsh. Most answers should FAIL.
If the answer is just "good enough", it FAILS.
Only truly excellent, insightful answers should pass.
"#.into(),
            user: format!(
                "Question: {}\n\nAnswer to evaluate:\n{}\n\nKnowledge base used:\n{:?}",
                question, answer, context.knowledge_items
            ),
        };

        let response = self.llm
            .generate_structured::<StrictResponse>(&prompt)
            .await?;

        Ok(EvaluationResult {
            eval_type: self.eval_type(),
            passed: response.passes_strict && response.quality_score >= 0.8,
            confidence: response.confidence,
            reasoning: response.reasoning,
            suggestions: response.improvement_suggestions,
            duration: start.elapsed(),
        })
    }

    fn generate_prompt(&self, question: &str, answer: &str) -> PromptPair {
        PromptPair {
            system: "Strict evaluator".into(),
            user: format!("{}\n{}", question, answer),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PIPELINE DE AVALIAÇÃO
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct EvaluationPipeline {
    evaluators: Vec<Box<dyn Evaluator>>,
}

impl EvaluationPipeline {
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self {
            evaluators: vec![
                Box::new(DefinitiveEvaluator {
                    config: EvaluationType::Definitive.default_config(),
                    llm: llm.clone(),
                }),
                Box::new(FreshnessEvaluator {
                    config: EvaluationType::Freshness.default_config(),
                    llm: llm.clone(),
                }),
                Box::new(PluralityEvaluator {
                    config: EvaluationType::Plurality.default_config(),
                    llm: llm.clone(),
                }),
                Box::new(CompletenessEvaluator {
                    config: EvaluationType::Completeness.default_config(),
                    llm: llm.clone(),
                }),
                Box::new(StrictEvaluator {
                    config: EvaluationType::Strict.default_config(),
                    llm: llm.clone(),
                }),
            ],
        }
    }

    /// Executa avaliações em sequência - FALHA RÁPIDA
    /// Retorna no primeiro erro para economizar tokens
    pub async fn evaluate_sequential(
        &self,
        question: &str,
        answer: &str,
        context: &EvaluationContext,
        required_types: &[EvaluationType],
    ) -> EvaluationPipelineResult {
        let mut results = Vec::new();

        for evaluator in &self.evaluators {
            // Pula avaliadores não requeridos
            if !required_types.contains(&evaluator.eval_type()) {
                continue;
            }

            let result = evaluator.evaluate(question, answer, context).await;

            match result {
                Ok(eval_result) => {
                    let passed = eval_result.passed;
                    results.push(eval_result);

                    // FALHA RÁPIDA - retorna imediatamente se falhou
                    if !passed {
                        return EvaluationPipelineResult {
                            overall_passed: false,
                            results,
                            failed_at: Some(evaluator.eval_type()),
                        };
                    }
                }
                Err(e) => {
                    // Erro de avaliação = falha
                    return EvaluationPipelineResult {
                        overall_passed: false,
                        results,
                        failed_at: Some(evaluator.eval_type()),
                    };
                }
            }
        }

        // Todas passaram
        EvaluationPipelineResult {
            overall_passed: true,
            results,
            failed_at: None,
        }
    }

    /// Determina quais avaliações são necessárias para uma pergunta
    pub async fn determine_required_evaluations(
        &self,
        question: &str,
        llm: &dyn LlmClient,
    ) -> Vec<EvaluationType> {
        let prompt = PromptPair {
            system: r#"
Analyze the question and determine which evaluation types are needed:
- definitive: Does this question have a clear factual answer?
- freshness: Is time-sensitive information relevant?
- plurality: Does it ask for multiple items/examples?
- completeness: Does it have multiple sub-questions or aspects?
- strict: Is this a complex question requiring deep analysis?
"#.into(),
            user: format!("Question: {}", question),
        };

        let response = llm
            .generate_structured::<RequiredEvaluations>(&prompt)
            .await;

        match response {
            Ok(r) => {
                let mut types = Vec::new();
                if r.needs_definitive { types.push(EvaluationType::Definitive); }
                if r.needs_freshness { types.push(EvaluationType::Freshness); }
                if r.needs_plurality { types.push(EvaluationType::Plurality); }
                if r.needs_completeness { types.push(EvaluationType::Completeness); }
                // Strict sempre no final, apenas para pergunta original
                types
            }
            Err(_) => {
                // Default: apenas definitive
                vec![EvaluationType::Definitive]
            }
        }
    }
}

#[derive(Debug)]
pub struct EvaluationPipelineResult {
    pub overall_passed: bool,
    pub results: Vec<EvaluationResult>,
    pub failed_at: Option<EvaluationType>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// VANTAGENS SOBRE TYPESCRIPT
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// 1. ENUM GARANTE TIPOS VÁLIDOS
//    Impossível passar "definativ" (typo) - não compila
let eval_type = EvaluationType::Definitive; // ✓
let eval_type = EvaluationType::Definativ;  // ERRO DE COMPILAÇÃO

// 2. PATTERN MATCHING EXAUSTIVO
//    Se adicionar novo tipo, compilador força tratar
match eval_type {
    EvaluationType::Definitive => { /* ... */ }
    EvaluationType::Freshness => { /* ... */ }
    // ERRO: não tratou Plurality, Completeness, Strict!
}

// 3. CONFIGURAÇÃO ASSOCIADA AO TIPO
//    Cada tipo de avaliação tem sua config padrão embutida
let config = EvaluationType::Strict.default_config();
assert_eq!(config.weight, 1.5); // Strict é mais importante

// 4. TRAIT PERMITE EXTENSÃO
//    Adicionar nova avaliação = implementar Evaluator
impl Evaluator for CustomEvaluator {
    fn eval_type(&self) -> EvaluationType { /* ... */ }
    async fn evaluate(&self, ...) -> Result<EvaluationResult, EvalError> { /* ... */ }
    fn generate_prompt(&self, ...) -> PromptPair { /* ... */ }
}
```

---

## 4. OTIMIZAÇÕES DE PERFORMANCE

### 4.1 SIMD para Similaridade Cosseno

O cálculo de similaridade cosseno é executado **milhares de vezes** por pesquisa (deduplicação de queries, matching de referências, reranking). É o hotpath mais crítico.

#### Implementação Atual em TypeScript (`cosine.ts`)

```typescript
// TypeScript: Loop simples, 1 operação por ciclo de CPU
export function cosineSimilarity(a: number[], b: number[]): number {
  let dotProduct = 0;
  let normA = 0;
  let normB = 0;

  for (let i = 0; i < a.length; i++) {
    dotProduct += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }

  return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
}

// Problema: Para embeddings de 768 dimensões (padrão Jina)
// = 768 iterações × 3 operações = 2304 operações SEQUENCIAIS
```

#### Implementação Otimizada em Rust com SIMD

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SIMD: Single Instruction, Multiple Data
// Processa 8 floats em paralelo por instrução de CPU
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Similaridade cosseno com AVX2 (256-bit SIMD)
/// Processa 8 floats por instrução
#[target_feature(enable = "avx2", enable = "fma")]
pub unsafe fn cosine_similarity_avx2(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let len = a.len();

    // Acumuladores SIMD (8 floats cada)
    let mut dot_acc = _mm256_setzero_ps();
    let mut norm_a_acc = _mm256_setzero_ps();
    let mut norm_b_acc = _mm256_setzero_ps();

    // Processa 8 elementos por iteração
    let chunks = len / 8;
    for i in 0..chunks {
        let offset = i * 8;

        // Carrega 8 floats de cada vetor
        let va = _mm256_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm256_loadu_ps(b.as_ptr().add(offset));

        // FMA: Fused Multiply-Add (a*b + acc em 1 instrução)
        dot_acc = _mm256_fmadd_ps(va, vb, dot_acc);
        norm_a_acc = _mm256_fmadd_ps(va, va, norm_a_acc);
        norm_b_acc = _mm256_fmadd_ps(vb, vb, norm_b_acc);
    }

    // Soma horizontal dos acumuladores
    let dot = hsum_avx2(dot_acc);
    let norm_a = hsum_avx2(norm_a_acc);
    let norm_b = hsum_avx2(norm_b_acc);

    // Processa elementos restantes (len % 8)
    let remainder_start = chunks * 8;
    let (mut dot_rem, mut norm_a_rem, mut norm_b_rem) = (0.0f32, 0.0f32, 0.0f32);
    for i in remainder_start..len {
        dot_rem += a[i] * b[i];
        norm_a_rem += a[i] * a[i];
        norm_b_rem += b[i] * b[i];
    }

    let total_dot = dot + dot_rem;
    let total_norm_a = norm_a + norm_a_rem;
    let total_norm_b = norm_b + norm_b_rem;

    total_dot / (total_norm_a.sqrt() * total_norm_b.sqrt())
}

/// Soma horizontal de 8 floats em um registro AVX2
#[inline]
#[target_feature(enable = "avx2")]
unsafe fn hsum_avx2(v: __m256) -> f32 {
    // [a0,a1,a2,a3,a4,a5,a6,a7] -> [a0+a4,a1+a5,a2+a6,a3+a7,...]
    let sum1 = _mm256_hadd_ps(v, v);
    // [...] -> [a0+a4+a1+a5, a2+a6+a3+a7, ...]
    let sum2 = _mm256_hadd_ps(sum1, sum1);
    // Extrai os dois valores restantes e soma
    let low = _mm256_castps256_ps128(sum2);
    let high = _mm256_extractf128_ps(sum2, 1);
    let final_sum = _mm_add_ss(low, high);
    _mm_cvtss_f32(final_sum)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// COMPARAÇÃO DE PERFORMANCE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// Para embeddings de 768 dimensões (Jina):
//
// TypeScript:
//   - 768 iterações do loop
//   - ~2304 operações de floating point
//   - ~15-20μs por comparação
//
// Rust + AVX2:
//   - 96 iterações do loop (768/8)
//   - 8 operações por ciclo (SIMD)
//   - FMA reduz latência
//   - ~1-2μs por comparação
//
// SPEEDUP: 10-15x para esta operação
```

#### Batch Processing com Paralelismo

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PROCESSAMENTO EM BATCH COM RAYON
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use rayon::prelude::*;

/// Calcula similaridade de uma query contra todas as existentes
/// Combina SIMD + Paralelismo multi-thread
pub fn find_similar_queries(
    query_embedding: &[f32],
    existing_embeddings: &[Vec<f32>],
    threshold: f32,
) -> Vec<(usize, f32)> {
    existing_embeddings
        .par_iter()  // Paralelo entre threads
        .enumerate()
        .filter_map(|(idx, existing)| {
            // SIMD dentro de cada thread
            let similarity = unsafe { cosine_similarity_avx2(query_embedding, existing) };
            if similarity >= threshold {
                Some((idx, similarity))
            } else {
                None
            }
        })
        .collect()
}

/// Deduplicação de queries com threshold 0.86
pub fn dedup_queries_fast(
    new_embeddings: &[Vec<f32>],
    existing_embeddings: &[Vec<f32>],
    threshold: f32,
) -> Vec<usize> {
    // Índices das queries únicas
    let mut unique_indices = Vec::new();
    let mut accepted_embeddings: Vec<&[f32]> = Vec::new();

    for (idx, new_emb) in new_embeddings.iter().enumerate() {
        let is_duplicate = existing_embeddings
            .par_iter()  // Paralelo: compara contra existentes
            .chain(accepted_embeddings.par_iter().map(|e| *e))  // + já aceitas
            .any(|existing| {
                unsafe { cosine_similarity_avx2(new_emb, existing) } >= threshold
            });

        if !is_duplicate {
            unique_indices.push(idx);
            accepted_embeddings.push(new_emb.as_slice());
        }
    }

    unique_indices
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Deduplicação de 100 queries contra 1000 existentes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// TypeScript (single-thread, loop simples):
//   - 100 × 1000 = 100,000 comparações
//   - ~20μs por comparação
//   - Total: ~2 segundos

// Rust (8 cores, SIMD AVX2):
//   - 100,000 comparações
//   - ~2μs por comparação (SIMD)
//   - ÷8 threads
//   - Total: ~25 milissegundos

// SPEEDUP TOTAL: ~80x
```

### 4.2 Gerenciamento de Memória Zero-Copy

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ZERO-COPY STRING HANDLING
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// TypeScript: Strings são IMUTÁVEIS
// Cada operação cria uma NOVA string na memória
//
// const text = "Hello World";
// const lower = text.toLowerCase();  // Nova alocação
// const trimmed = lower.trim();      // Nova alocação
// const replaced = trimmed.replace("hello", "hi");  // Nova alocação
//
// 4 strings na memória para 1 operação lógica!

// Rust: Borrowing permite operações sem cópia
use std::borrow::Cow;

/// Processa texto sem copiar desnecessariamente
pub fn process_text<'a>(text: &'a str) -> Cow<'a, str> {
    // Se não precisa modificar, retorna referência (zero copy)
    if text.chars().all(|c| c.is_lowercase()) && !text.contains("  ") {
        Cow::Borrowed(text)  // Apenas referência, sem alocação
    } else {
        // Só aloca se realmente precisar modificar
        Cow::Owned(
            text.to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// KNOWLEDGE ITEMS COM ARENA ALLOCATION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use bumpalo::Bump;

/// Arena allocator para conhecimento acumulado
/// Todas as strings da sessão ficam em um bloco contíguo
pub struct KnowledgeArena {
    arena: Bump,
}

impl KnowledgeArena {
    pub fn new() -> Self {
        // Pré-aloca 1MB para a sessão
        Self {
            arena: Bump::with_capacity(1024 * 1024),
        }
    }

    /// Adiciona conhecimento à arena (sem fragmentação)
    pub fn add_knowledge<'a>(&'a self, question: &str, answer: &str) -> KnowledgeRef<'a> {
        let q = self.arena.alloc_str(question);
        let a = self.arena.alloc_str(answer);

        KnowledgeRef {
            question: q,
            answer: a,
        }
    }

    /// No final da sessão, um único `drop` libera tudo
    pub fn clear(&mut self) {
        self.arena.reset();
    }
}

pub struct KnowledgeRef<'a> {
    pub question: &'a str,
    pub answer: &'a str,
}

// Vantagem:
// - Sem fragmentação de memória
// - Localidade de cache (dados próximos)
// - Liberação O(1) no final
```

### 4.3 Async Runtime Otimizado

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TOKIO: ASYNC RUNTIME MULTI-THREADED
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use tokio::task::JoinSet;

/// Executa múltiplas buscas em paralelo REAL
pub async fn search_parallel(
    queries: Vec<SerpQuery>,
    client: &SearchClient,
) -> Vec<SearchResult> {
    let mut tasks = JoinSet::new();

    for query in queries {
        let client = client.clone();
        tasks.spawn(async move {
            client.search(&query).await
        });
    }

    let mut results = Vec::with_capacity(queries.len());
    while let Some(result) = tasks.join_next().await {
        if let Ok(Ok(search_result)) = result {
            results.push(search_result);
        }
    }

    results
}

// TypeScript Promise.all vs Rust JoinSet:
//
// TypeScript (Node.js):
// - Event loop single-threaded
// - Concorrência I/O, não paralelismo CPU
// - 10 requests = intercalados na mesma thread
//
// Rust (Tokio multi-thread):
// - Work-stealing scheduler
// - Cada task pode rodar em thread diferente
// - 10 requests = até 10 threads paralelas
// - CPU work (parsing, etc) realmente paralelo

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// STREAMING COM BACKPRESSURE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use tokio::sync::mpsc;
use futures::stream::StreamExt;

/// Stream de respostas com controle de fluxo
pub async fn stream_response(
    answer: String,
    tx: mpsc::Sender<String>,
) -> Result<(), StreamError> {
    // Divide em chunks naturais (sentenças)
    let sentences = answer
        .split_inclusive(|c| c == '.' || c == '!' || c == '?')
        .collect::<Vec<_>>();

    for sentence in sentences {
        // Backpressure: aguarda se o receiver está lento
        tx.send(sentence.to_string()).await
            .map_err(|_| StreamError::ChannelClosed)?;

        // Delay adaptativo baseado no tamanho
        let delay = (sentence.len() as u64 * 10).min(100);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
    }

    Ok(())
}
```

### 4.4 HTML Parsing de Alta Performance

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// LOL_HTML: STREAMING HTML PARSER
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use lol_html::{element, HtmlRewriter, Settings};

/// Extrai texto de HTML em streaming (não carrega tudo na memória)
pub fn extract_text_streaming(html: &[u8]) -> String {
    let mut text_content = String::new();

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                // Remove scripts e styles
                element!("script, style, noscript", |el| {
                    el.remove();
                    Ok(())
                }),
                // Extrai texto de parágrafos
                element!("p, h1, h2, h3, h4, h5, h6, li, td, th", |el| {
                    el.on_end_tag(|end| {
                        // Adiciona quebra de linha após cada elemento
                        text_content.push('\n');
                        Ok(())
                    })?;
                    Ok(())
                }),
            ],
            ..Settings::default()
        },
        |c: &[u8]| {
            if let Ok(s) = std::str::from_utf8(c) {
                text_content.push_str(s);
            }
        },
    );

    rewriter.write(html).unwrap();
    rewriter.end().unwrap();

    // Limpa whitespace excessivo
    text_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// TypeScript com jsdom:
// - Carrega HTML inteiro na memória
// - Constrói DOM tree completa
// - Garbage collection de objetos DOM
// - ~500ms para 1MB de HTML
//
// Rust com lol_html:
// - Processa em streaming (chunks)
// - Não aloca DOM tree
// - Zero GC (Rust não tem GC)
// - ~30ms para 1MB de HTML
//
// SPEEDUP: ~15-20x
```

### 4.5 Resumo de Ganhos de Performance

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    COMPARAÇÃO TYPESCRIPT vs RUST                        │
├─────────────────────────────┬───────────────┬───────────────┬───────────┤
│ Operação                    │ TypeScript    │ Rust          │ Speedup   │
├─────────────────────────────┼───────────────┼───────────────┼───────────┤
│ Similaridade cosseno (1x)   │ ~20μs         │ ~2μs          │ 10x       │
│ Dedup 100 queries           │ ~2000ms       │ ~25ms         │ 80x       │
│ Parse HTML 1MB              │ ~500ms        │ ~30ms         │ 17x       │
│ String processing (batch)   │ ~100ms        │ ~8ms          │ 12x       │
│ Memory footprint            │ ~500MB        │ ~50MB         │ 10x menos │
├─────────────────────────────┼───────────────┼───────────────┼───────────┤
│ THROUGHPUT TOTAL ESTIMADO   │ 1x (baseline) │ 10-20x        │           │
│ CUSTO DE INFRA              │ 1x (baseline) │ 0.1-0.2x      │ 80-90%    │
└─────────────────────────────┴───────────────┴───────────────┴───────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                    FONTES DOS GANHOS                                    │
├─────────────────────────────────────────────────────────────────────────┤
│ 1. SIMD (AVX2/AVX512)      │ 8-16 operações por ciclo de CPU            │
│ 2. Multi-threading real    │ Paralelismo de CPU, não apenas I/O         │
│ 3. Zero-copy strings       │ Borrowing elimina alocações                │
│ 4. Sem Garbage Collection  │ Latência previsível, sem pausas            │
│ 5. Cache locality          │ Arena allocation, dados contíguos          │
│ 6. Streaming I/O           │ Não carrega arquivos inteiros na memória   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 5. CONCLUSÃO

A migração de TypeScript para Rust não é apenas uma otimização - é uma **mudança arquitetural** que permite:

1. **Segurança em Compile-Time**: Enums e pattern matching eliminam classes inteiras de bugs
2. **Performance 10-20x**: SIMD + paralelismo real + zero-copy
3. **Custo 80-90% menor**: Mesma capacidade com muito menos hardware
4. **Latência previsível**: Sem GC pauses, importante para streaming

O código TypeScript atual é excelente para **prototipagem e iteração rápida**. Mas para produção em escala, Rust oferece vantagens que justificam o investimento em reescrita.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    QUANDO MIGRAR PARA RUST?                             │
├─────────────────────────────────────────────────────────────────────────┤
│ ✓ Quando o custo de infra começa a importar                             │
│ ✓ Quando latência previsível é crítica (SLAs)                           │
│ ✓ Quando você quer processar 10x mais requisições                       │
│ ✓ Quando a arquitetura está estável e bem definida                      │
├─────────────────────────────────────────────────────────────────────────┤
│ ✗ Durante prototipagem e experimentação                                  │
│ ✗ Quando a equipe não tem experiência com Rust                          │
│ ✗ Quando time-to-market é a prioridade absoluta                         │
└─────────────────────────────────────────────────────────────────────────┘
```