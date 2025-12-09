// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEEP RESEARCH AGENT - MÁQUINA DE ESTADOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod state;
mod actions;
mod context;
mod permissions;

pub use state::*;
pub use actions::*;
pub use context::*;
pub use permissions::*;

use std::sync::Arc;
use crate::types::*;
use crate::llm::LlmClient;
use crate::search::SearchClient;
use crate::utils::TokenTracker;

/// Máximo de queries por passo (reservado para expansão futura)
#[allow(dead_code)]
const MAX_QUERIES_PER_STEP: usize = 5;
/// Máximo de URLs por passo
const MAX_URLS_PER_STEP: usize = 5;

/// Agente principal de pesquisa profunda
pub struct DeepResearchAgent {
    state: AgentState,
    context: AgentContext,
    llm_client: Arc<dyn LlmClient>,
    search_client: Arc<dyn SearchClient>,
    token_tracker: TokenTracker,
}

impl DeepResearchAgent {
    /// Cria um novo agente com os clientes fornecidos
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        search_client: Arc<dyn SearchClient>,
        token_budget: Option<u64>,
    ) -> Self {
        Self {
            state: AgentState::Processing {
                step: 0,
                total_step: 0,
                current_question: String::new(),
                budget_used: 0.0,
            },
            context: AgentContext::new(),
            llm_client,
            search_client,
            token_tracker: TokenTracker::new(token_budget),
        }
    }

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

                AgentState::BeastMode { attempts: _attempts, .. } => {
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
                            if let AgentState::BeastMode { attempts, .. } = &mut self.state {
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
        let action = match self.llm_client.decide_action(&prompt, &permissions).await {
            Ok(a) => a,
            Err(e) => return StepResult::Error(format!("LLM error: {}", e)),
        };

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
            // Ações de integração Paytour - delegar para handlers específicos
            AgentAction::PaytourListarPasseios { ref think, .. }
            | AgentAction::PaytourDetalharPasseio { ref think, .. }
            | AgentAction::PaytourVerificarDisponibilidade { ref think, .. }
            | AgentAction::PaytourObterHorarios { ref think, .. } => {
                self.execute_integration_action(&action, think).await
            }
            // Ações de integração Digisac - delegar para handlers específicos
            AgentAction::DigisacEnviarMensagem { ref think, .. }
            | AgentAction::DigisacListarWebhooks { ref think, .. }
            | AgentAction::DigisacCriarWebhook { ref think, .. } => {
                self.execute_integration_action(&action, think).await
            }
        }
    }

    /// Executa ações de integração (Paytour/Digisac)
    async fn execute_integration_action(&mut self, action: &AgentAction, think: &str) -> StepResult {
        // Registrar no diário baseado no tipo de ação
        if action.is_paytour() {
            self.context.diary.push(DiaryEntry::PaytourQuery {
                query_type: action.name().to_string(),
                think: think.to_string(),
                results_count: 0, // Atualizado após execução real
            });
        } else if action.is_digisac() {
            self.context.diary.push(DiaryEntry::DigisacAction {
                action_type: action.name().to_string(),
                think: think.to_string(),
                success: true, // Atualizado após execução real
            });
        }

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Rotaciona para a próxima pergunta na fila
    fn rotate_question(&mut self) -> String {
        let idx = self.context.total_step % self.context.gap_questions.len();
        self.context.gap_questions[idx].clone()
    }

    /// Constrói o prompt para o LLM decidir a próxima ação
    fn build_prompt(&self, permissions: &ActionPermissions, question: &str) -> AgentPrompt {
        AgentPrompt {
            system: self.build_system_prompt(permissions),
            user: format!("Current question: {}\n\nKnowledge so far:\n{}",
                question,
                self.format_knowledge()
            ),
            diary: self.context.diary.clone(),
        }
    }

    fn build_system_prompt(&self, permissions: &ActionPermissions) -> String {
        let mut prompt = String::from("You are a research agent. Choose the best action:\n\n");

        if permissions.search {
            prompt.push_str("- SEARCH: Search the web for information\n");
        }
        if permissions.read {
            prompt.push_str("- READ: Read a URL in depth\n");
        }
        if permissions.reflect {
            prompt.push_str("- REFLECT: Generate gap-closing sub-questions\n");
        }
        if permissions.answer {
            prompt.push_str("- ANSWER: Provide the final answer\n");
        }
        if permissions.coding {
            prompt.push_str("- CODING: Execute code for data processing\n");
        }

        prompt
    }

    fn format_knowledge(&self) -> String {
        self.context.knowledge
            .iter()
            .map(|k| format!("Q: {}\nA: {}", k.question, k.answer))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Executa ação de busca
    async fn execute_search(&mut self, queries: Vec<SerpQuery>, think: String) -> StepResult {
        use crate::personas::PersonaOrchestrator;

        // Expandir queries com personas cognitivas
        let orchestrator = PersonaOrchestrator::new();
        let context = self.build_query_context();
        let expanded = orchestrator.expand_batch(
            &queries.iter().map(|q| q.q.clone()).collect::<Vec<_>>(),
            &context
        );

        // Deduplicar contra queries existentes
        let unique = self.dedup_queries(
            expanded.iter().map(|wq| wq.query.clone()).collect()
        ).await;

        // Executar buscas em paralelo
        let results = self.search_client.search_batch(&unique).await;

        // Adicionar URLs ao contexto
        for result in results {
            if let Ok(r) = result {
                self.context.add_urls(r.urls);
                self.context.add_snippets(r.snippets);
            }
        }

        // Registrar no diário
        self.context.diary.push(DiaryEntry::Search {
            queries: unique,
            think,
            urls_found: self.context.collected_urls.len(),
        });

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Executa ação de leitura de URL
    async fn execute_read(&mut self, urls: Vec<Url>, think: String) -> StepResult {
        for url in urls.iter().take(MAX_URLS_PER_STEP) {
            match self.search_client.read_url(url).await {
                Ok(content) => {
                    self.context.add_knowledge(KnowledgeItem {
                        question: self.context.current_question().to_string(),
                        answer: content.text,
                        item_type: KnowledgeType::Url,
                        references: vec![Reference {
                            url: url.to_string(),
                            title: content.title,
                            exact_quote: None,
                            relevance_score: None,
                        }],
                    });
                    self.context.visited_urls.push(url.clone());
                }
                Err(e) => {
                    log::warn!("Failed to read URL {}: {}", url, e);
                    self.context.bad_urls.push(url.clone());
                }
            }
        }

        self.context.diary.push(DiaryEntry::Read {
            urls: urls.clone(),
            think,
        });

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Executa ação de reflexão
    async fn execute_reflect(&mut self, gap_questions: Vec<String>, think: String) -> StepResult {
        // Deduplicar novas perguntas
        let unique_questions = self.dedup_questions(gap_questions).await;

        // Adicionar ao gap_questions (máximo MAX_REFLECT_PER_STEP)
        for q in unique_questions.into_iter().take(MAX_REFLECT_PER_STEP) {
            if !self.context.gap_questions.contains(&q) {
                self.context.gap_questions.push(q);
            }
        }

        self.context.diary.push(DiaryEntry::Reflect {
            questions: self.context.gap_questions.clone(),
            think,
        });

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Executa avaliação de resposta
    async fn execute_answer(
        &mut self,
        answer: String,
        references: Vec<Reference>,
        _think: String
    ) -> StepResult {
        use crate::evaluation::{EvaluationPipeline, EvaluationType};

        // Resposta imediata no step 1 = pergunta trivial
        if self.context.total_step == 1 && self.context.allow_direct_answer {
            return StepResult::Completed(AnswerResult {
                answer,
                references,
                trivial: true,
            });
        }

        // Obter tipos de avaliação necessários
        let pipeline = EvaluationPipeline::new(self.llm_client.clone());
        let eval_types = pipeline
            .determine_required_evaluations(&self.context.original_question, &*self.llm_client)
            .await;

        // Executar avaliações
        let eval_context = self.build_evaluation_context();
        let result = pipeline
            .evaluate_sequential(&self.context.original_question, &answer, &eval_context, &eval_types)
            .await;

        if result.overall_passed {
            StepResult::Completed(AnswerResult {
                answer,
                references,
                trivial: false,
            })
        } else {
            // Adicionar falha como conhecimento
            let failed_type = result.failed_at.unwrap_or(EvaluationType::Definitive);
            let reasoning = result.results
                .last()
                .map(|r| r.reasoning.clone())
                .unwrap_or_default();

            self.context.knowledge.push(KnowledgeItem {
                question: self.context.original_question.clone(),
                answer: format!("FAILED {}: {}", failed_type, reasoning),
                item_type: KnowledgeType::Error,
                references: vec![],
            });

            self.context.diary.push(DiaryEntry::FailedAnswer {
                answer,
                eval_type: failed_type,
                reason: reasoning,
            });

            self.context.total_step += 1;
            StepResult::Continue
        }
    }

    /// Executa código em sandbox
    /// Executa código em sandbox
    async fn execute_coding(&mut self, code: String, think: String) -> StepResult {
        // Executar código em sandbox seguro
        match self.execute_sandbox(&code).await {
            Ok(output) => {
                self.context.knowledge.push(KnowledgeItem {
                    question: self.context.current_question().to_string(),
                    answer: output,
                    item_type: KnowledgeType::Coding,
                    references: vec![],
                });
            }
            Err(e) => {
                log::warn!("Sandbox execution failed: {}", e);
            }
        }

        self.context.diary.push(DiaryEntry::Coding {
            code,
            think,
        });

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Força uma resposta em Beast Mode
    async fn force_answer(&mut self) -> Result<AnswerResult, AgentError> {
        let prompt = AgentPrompt {
            system: "You MUST provide an answer now. No more searching or reflecting. \
                     Be pragmatic and use what you know.".into(),
            user: format!(
                "Question: {}\n\nKnowledge:\n{}\n\nProvide your best answer.",
                self.context.original_question,
                self.format_knowledge()
            ),
            diary: self.context.diary.clone(),
        };

        let response = self.llm_client
            .generate_answer(&prompt, 0.7) // Higher temperature
            .await
            .map_err(|e| AgentError::LlmError(e.to_string()))?;

        Ok(AnswerResult {
            answer: response.answer,
            references: response.references,
            trivial: false,
        })
    }

    /// Constrói o resultado final
    fn build_result(self) -> ResearchResult {
        match self.state {
            AgentState::Completed { answer, references, trivial } => {
                ResearchResult {
                    success: true,
                    answer: Some(answer),
                    references,
                    trivial,
                    token_usage: self.token_tracker.get_total_usage(),
                    visited_urls: self.context.visited_urls,
                    error: None,
                }
            }
            AgentState::Failed { reason, partial_knowledge: _partial_knowledge } => {
                ResearchResult {
                    success: false,
                    answer: None,
                    references: vec![],
                    trivial: false,
                    token_usage: self.token_tracker.get_total_usage(),
                    visited_urls: self.context.visited_urls,
                    error: Some(reason),
                }
            }
            _ => unreachable!("build_result called in non-terminal state"),
        }
    }

    // Métodos auxiliares...

    async fn dedup_queries(&self, queries: Vec<SerpQuery>) -> Vec<SerpQuery> {
        // Implementação usando embeddings
        queries // Simplificado
    }

    async fn dedup_questions(&self, questions: Vec<String>) -> Vec<String> {
        // Implementação usando embeddings
        questions // Simplificado
    }

    fn build_query_context(&self) -> crate::personas::QueryContext {
        crate::personas::QueryContext {
            original_query: self.context.original_question.clone(),
            user_intent: String::new(),
            soundbites: self.context.snippets.clone(),
            current_date: chrono::Utc::now().date_naive(),
            detected_language: Language::English,
            detected_topic: TopicCategory::General,
        }
    }

    fn build_evaluation_context(&self) -> crate::evaluation::EvaluationContext {
        crate::evaluation::EvaluationContext {
            topic: TopicCategory::General,
            knowledge_items: self.context.knowledge.clone(),
        }
    }

    async fn execute_sandbox(&self, _code: &str) -> Result<String, AgentError> {
        // Implementação de sandbox
        Ok("Sandbox output".into())
    }
}
