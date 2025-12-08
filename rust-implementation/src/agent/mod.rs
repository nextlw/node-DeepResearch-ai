// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEEP RESEARCH AGENT - MÁQUINA DE ESTADOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod actions;
mod context;
mod permissions;
mod state;

pub use actions::*;
pub use context::*;
pub use permissions::*;
pub use state::*;

use crate::llm::LlmClient;
use crate::search::SearchClient;
use crate::types::*;
use crate::utils::{ActionTimer, TimingStats, TokenTracker, TrackerStats};
use std::sync::Arc;

/// Evento de progresso do agente para callbacks em tempo real
#[derive(Debug, Clone)]
pub enum AgentProgress {
    /// Log informativo
    Info(String),
    /// Log de sucesso
    Success(String),
    /// Log de aviso
    Warning(String),
    /// Log de erro
    Error(String),
    /// Atualiza step atual
    Step(usize),
    /// Atualiza ação atual
    Action(String),
    /// Atualiza raciocínio atual
    Think(String),
    /// Atualiza contagem de URLs (total, visitadas)
    Urls(usize, usize),
    /// Atualiza tokens usados
    Tokens(u64),
    /// Atualiza stats de persona (nome, searches, reads, answers, tokens, is_active)
    Persona {
        name: String,
        searches: usize,
        reads: usize,
        answers: usize,
        tokens: u64,
        is_active: bool,
    },
}

/// Tipo do callback de progresso
pub type ProgressCallback = Arc<dyn Fn(AgentProgress) + Send + Sync>;

/// Máximo de queries por passo (reservado para expansão futura)
#[allow(dead_code)]
const MAX_QUERIES_PER_STEP: usize = 5;
/// Máximo de URLs por passo
const MAX_URLS_PER_STEP: usize = 5;
/// Máximo de steps antes de forçar resposta (reservado para expansão futura)
#[allow(dead_code)]
const MAX_STEPS_BEFORE_ANSWER: usize = 15;

/// Agente principal de pesquisa profunda
pub struct DeepResearchAgent {
    state: AgentState,
    context: AgentContext,
    llm_client: Arc<dyn LlmClient>,
    search_client: Arc<dyn SearchClient>,
    /// Rastreador de tokens para controle de budget
    token_tracker: TokenTracker,
    timing_stats: TimingStats,
    start_time: std::time::Instant,
    /// Callback opcional para progresso em tempo real
    progress_callback: Option<ProgressCallback>,
    /// Contador de buscas realizadas
    search_count: usize,
    /// Contador de leituras realizadas
    read_count: usize,
    /// Contador de respostas geradas
    answer_count: usize,
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
            timing_stats: TimingStats::new(),
            start_time: std::time::Instant::now(),
            progress_callback: None,
            search_count: 0,
            read_count: 0,
            answer_count: 0,
        }
    }

    /// Configura callback de progresso para updates em tempo real
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Envia evento de progresso se callback configurado
    fn emit(&self, event: AgentProgress) {
        if let Some(cb) = &self.progress_callback {
            cb(event);
        }
    }

    /// Emite estatísticas atuais do agente como "persona"
    fn emit_persona_stats(&self, is_active: bool) {
        self.emit(AgentProgress::Persona {
            name: "Agente".to_string(),
            searches: self.search_count,
            reads: self.read_count,
            answers: self.answer_count,
            tokens: self.token_tracker.total_tokens(),
            is_active,
        });
    }

    /// Loop principal - consome self e retorna resultado final
    pub async fn run(mut self, question: String) -> ResearchResult {
        // Inicialização
        self.context.original_question = question.clone();
        self.context.gap_questions.push(question.clone());

        // Emitir início
        self.emit(AgentProgress::Info(format!("Iniciando pesquisa: {}", question)));
        self.emit(AgentProgress::Step(0));
        self.emit(AgentProgress::Action("Inicializando...".into()));

        // Loop principal com pattern matching exaustivo
        loop {
            match &self.state {
                AgentState::Processing { .. } if self.token_tracker.should_enter_beast_mode() => {
                    // Transição para Beast Mode (>= 85% do budget de tokens)
                    let msg = format!(
                        "Budget de tokens em {:.1}% - entrando em Beast Mode",
                        self.token_tracker.budget_used_percentage() * 100.0
                    );
                    log::warn!("⚠️ {}", msg);
                    self.emit(AgentProgress::Warning(msg));
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

                AgentState::BeastMode {
                    attempts: _attempts,
                    ..
                } => {
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

        // 3. Gerar prompt e obter decisão do LLM (com timing)
        let prompt = self.build_prompt(&permissions, &current_question);

        // Capturar tokens antes da chamada
        let tokens_before = self.llm_client.get_total_tokens();

        let llm_timer = ActionTimer::start("LLM decide_action");
        let action = match self.llm_client.decide_action(&prompt, &permissions).await {
            Ok(a) => a,
            Err(e) => return StepResult::Error(format!("LLM error: {}", e)),
        };
        let llm_time = llm_timer.stop();
        self.timing_stats.add_llm_time(llm_time);

        // Rastrear tokens usados nesta operação
        let tokens_after = self.llm_client.get_total_tokens();
        let prompt_used = self.llm_client.get_prompt_tokens().saturating_sub(tokens_before);
        let completion_used = tokens_after.saturating_sub(tokens_before).saturating_sub(prompt_used);
        self.token_tracker.track(
            self.context.total_step,
            &format!("decide_action:{}", action.name()),
            prompt_used,
            completion_used,
        );

        // Atualizar budget_used no estado
        self.update_budget_used();

        log::debug!("⏱️  LLM decision: {}ms | 🎟️ Tokens: {} ({:.1}% budget)",
            llm_time,
            tokens_after - tokens_before,
            self.token_tracker.budget_used_percentage() * 100.0
        );

        log::info!(
            "📍 Step {} | Action: {} | Think: {}",
            self.context.total_step,
            action.name(),
            action.think().chars().take(150).collect::<String>()
        );

        // Emitir progresso para TUI
        self.emit(AgentProgress::Step(self.context.total_step));
        self.emit(AgentProgress::Action(action.name().to_string()));
        self.emit(AgentProgress::Think(action.think().chars().take(200).collect()));
        self.emit(AgentProgress::Tokens(self.token_tracker.total_tokens()));
        self.emit(AgentProgress::Urls(
            self.context.collected_urls.len(),
            self.context.visited_urls.len(),
        ));

        // 4. Executar ação escolhida - pattern matching garante cobertura total
        match action {
            AgentAction::Search { queries, think } => {
                let query_list: Vec<_> = queries.iter().map(|q| q.q.clone()).collect();
                self.emit(AgentProgress::Info(format!("🔍 Buscando: {}", query_list.join(", "))));
                log::debug!(
                    "🔍 Queries: {:?}",
                    queries.iter().map(|q| &q.q).collect::<Vec<_>>()
                );
                self.execute_search(queries, think).await
            }
            AgentAction::Read { urls, think } => {
                // Filtrar URLs já visitadas ou ruins
                let mut new_urls: Vec<_> = urls
                    .into_iter()
                    .filter(|u| !self.context.is_url_visited(u) && !self.context.is_url_bad(u))
                    .collect();

                // Se LLM escolheu URLs já visitadas, pegar as próximas não visitadas automaticamente
                if new_urls.is_empty() {
                    let msg = "LLM escolheu URLs já visitadas, selecionando próximas disponíveis...";
                    log::warn!("⚠️ {}", msg);
                    self.emit(AgentProgress::Warning(msg.into()));
                    new_urls = self
                        .context
                        .collected_urls
                        .iter()
                        .filter(|u| {
                            !self.context.is_url_visited(&u.url) && !self.context.is_url_bad(&u.url)
                        })
                        .take(MAX_URLS_PER_STEP)
                        .map(|u| u.url.clone())
                        .collect();
                }

                // Se ainda não há URLs disponíveis, tentar responder
                if new_urls.is_empty() {
                    let msg = "Nenhuma URL disponível! Tentando gerar resposta...";
                    log::warn!("⚠️ {}", msg);
                    self.emit(AgentProgress::Warning(msg.into()));
                    self.context.total_step += 1;
                    // Forçar tentativa de resposta
                    return StepResult::Continue;
                }

                self.emit(AgentProgress::Info(format!(
                    "📖 Lendo {} URLs: {}",
                    new_urls.len(),
                    new_urls.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
                )));
                log::info!(
                    "📖 URLs selecionadas ({} novas): {:?}",
                    new_urls.len(),
                    new_urls.iter().take(3).collect::<Vec<_>>()
                );
                self.execute_read(new_urls, think).await
            }
            AgentAction::Reflect {
                gap_questions,
                think,
            } => {
                self.emit(AgentProgress::Info(format!(
                    "🤔 Refletindo: {} novas perguntas",
                    gap_questions.len()
                )));
                log::debug!("🤔 Gap questions: {:?}", gap_questions);
                self.execute_reflect(gap_questions, think).await
            }
            AgentAction::Answer {
                answer,
                references,
                think,
            } => {
                self.emit(AgentProgress::Success(format!(
                    "✍️ Gerando resposta ({} chars, {} refs)",
                    answer.len(),
                    references.len()
                )));
                log::info!(
                    "✍️ Resposta proposta ({} chars, {} refs)",
                    answer.len(),
                    references.len()
                );
                log::info!("💭 Raciocínio: {}", think);
                log::info!("📝 Resposta:\n{}", answer);
                if !references.is_empty() {
                    log::info!("📚 Referências:");
                    for r in &references {
                        log::info!("   - {} ({})", r.title, r.url);
                    }
                }
                self.execute_answer(answer, references, think).await
            }
            AgentAction::Coding { code, think } => self.execute_coding(code, think).await,
        }
    }

    /// Rotaciona para a próxima pergunta na fila
    fn rotate_question(&mut self) -> String {
        let idx = self.context.total_step % self.context.gap_questions.len();
        self.context.gap_questions[idx].clone()
    }

    /// Constrói o prompt para o LLM decidir a próxima ação
    fn build_prompt(&self, permissions: &ActionPermissions, question: &str) -> AgentPrompt {
        // Listar URLs disponíveis (não visitadas)
        let available_urls: Vec<_> = self
            .context
            .collected_urls
            .iter()
            .filter(|u| !self.context.is_url_visited(&u.url) && !self.context.is_url_bad(&u.url))
            .take(10)
            .map(|u| format!("- {} ({})", u.url, u.title))
            .collect();

        let urls_section = if available_urls.is_empty() {
            "No unvisited URLs available.".to_string()
        } else {
            format!(
                "Available URLs to read (pick different ones each time!):\n{}",
                available_urls.join("\n")
            )
        };

        AgentPrompt {
            system: self.build_system_prompt(permissions),
            user: format!(
                "Current question: {}\n\n{}\n\nAlready visited URLs: {}\n\nKnowledge so far:\n{}",
                question,
                urls_section,
                self.context.visited_urls.len(),
                self.format_knowledge()
            ),
            diary: self.context.diary.clone(),
        }
    }

    fn build_system_prompt(&self, permissions: &ActionPermissions) -> String {
        let mut prompt = String::from(
            r#"You are a research agent. Your goal is to find accurate information efficiently.

CRITICAL RULES:
1. NEVER read the same URL twice - pick DIFFERENT URLs from the available list
2. After reading 3-5 different URLs, try to ANSWER the question
3. If you have enough information, use ANSWER action immediately
4. Only use SEARCH if you need completely different information

Available actions:
"#,
        );

        if permissions.search {
            prompt.push_str("- SEARCH: Search the web (only if current URLs are insufficient)\n");
        }
        if permissions.read {
            prompt.push_str(
                "- READ: Read URLs from the available list (MUST pick different URLs each time!)\n",
            );
            prompt.push_str("  → Supports: web pages, PDFs, JSON, XML, TXT, Markdown files\n");
            prompt.push_str(
                "  → Files (.pdf, .json, etc.) are downloaded and extracted automatically\n",
            );
        }
        if permissions.reflect {
            prompt.push_str("- REFLECT: Generate sub-questions (use sparingly)\n");
        }
        if permissions.answer {
            prompt.push_str(
                "- ANSWER: Provide the final answer (USE THIS when you have enough info!)\n",
            );
        }
        if permissions.coding {
            prompt.push_str("- CODING: Execute code for data processing\n");
        }

        // Adicionar info sobre URLs visitadas
        if !self.context.visited_urls.is_empty() {
            prompt.push_str(&format!(
                "\n⚠️ You have already visited {} URLs. Pick NEW ones or ANSWER!\n",
                self.context.visited_urls.len()
            ));
        }

        // Forçar resposta após muitos steps
        if self.context.total_step >= 5 {
            prompt.push_str("\n🔴 You have done many steps. Consider using ANSWER now!\n");
        }

        prompt
    }

    fn format_knowledge(&self) -> String {
        self.context
            .knowledge
            .iter()
            .map(|k| format!("Q: {}\nA: {}", k.question, k.answer))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Executa ação de busca
    async fn execute_search(&mut self, queries: Vec<SerpQuery>, think: String) -> StepResult {
        use crate::personas::PersonaOrchestrator;
        let search_timer = ActionTimer::start("Search");

        // Expandir queries com personas cognitivas
        let orchestrator = PersonaOrchestrator::new();
        let context = self.build_query_context();
        let expanded = orchestrator.expand_batch(
            &queries.iter().map(|q| q.q.clone()).collect::<Vec<_>>(),
            &context,
        );

        // Deduplicar contra queries existentes
        let unique = self
            .dedup_queries(expanded.iter().map(|wq| wq.query.clone()).collect())
            .await;

        // Executar buscas em paralelo
        let results = self.search_client.search_batch(&unique).await;

        // Adicionar URLs ao contexto
        for result in results {
            if let Ok(r) = result {
                self.context.add_urls(r.urls);
                self.context.add_snippets(r.snippets);
            }
        }

        let search_time = search_timer.stop();
        self.timing_stats.add_search_time(search_time);

        // Incrementar contador de buscas
        self.search_count += 1;

        log::info!(
            "🔍 Busca concluída: {} URLs encontradas ({}ms)",
            self.context.collected_urls.len(),
            search_time
        );

        // Emitir log e atualizar persona
        self.emit(AgentProgress::Success(format!(
            "🔍 Busca #{}: {} URLs encontradas",
            self.search_count,
            self.context.collected_urls.len()
        )));
        self.emit_persona_stats(true);

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
        use crate::utils::{FileReader, FileType};

        let read_timer = ActionTimer::start("Read URLs");
        log::info!("📖 Lendo {} URLs...", urls.len().min(MAX_URLS_PER_STEP));
        let file_reader = FileReader::new();

        for url in urls.iter().take(MAX_URLS_PER_STEP) {
            // Detectar se é arquivo para download direto (PDF, JSON, etc.)
            let file_type = FileType::from_url(url);
            let is_file = matches!(
                file_type,
                FileType::Pdf
                    | FileType::Json
                    | FileType::Xml
                    | FileType::Text
                    | FileType::Markdown
            );

            if is_file {
                // Usar FileReader para arquivos
                log::info!("📥 Detectado arquivo {:?}: {}", file_type, url);
                match file_reader.read_url(url).await {
                    Ok(file_content) => {
                        log::info!(
                            "✅ Arquivo lido: {} palavras | {} bytes",
                            file_content.word_count,
                            file_content.size_bytes
                        );

                        self.context.add_knowledge(KnowledgeItem {
                            question: self.context.current_question().to_string(),
                            answer: file_content.text,
                            item_type: KnowledgeType::Url,
                            references: vec![Reference {
                                url: url.to_string(),
                                title: file_content
                                    .title
                                    .unwrap_or_else(|| format!("Arquivo {:?}", file_type)),
                                exact_quote: None,
                                relevance_score: None,
                            }],
                        });
                        self.context.visited_urls.push(url.clone());
                    }
                    Err(e) => {
                        log::warn!("❌ Falha ao ler arquivo {}: {}", url, e);
                        self.context.bad_urls.push(url.clone());
                    }
                }
            } else {
                // Usar Jina Reader para páginas web
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
        }

        let read_time = read_timer.stop();
        self.timing_stats.add_read_time(read_time);

        // Incrementar contador de leituras
        self.read_count += 1;

        log::info!("📖 Leitura concluída ({}ms)", read_time);

        // Emitir log e atualizar persona
        self.emit(AgentProgress::Success(format!(
            "📖 Leitura #{}: {} URLs processadas",
            self.read_count,
            urls.len().min(MAX_URLS_PER_STEP)
        )));
        self.emit_persona_stats(true);

        self.context.diary.push(DiaryEntry::Read {
            urls: urls.clone(),
            think,
        });

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Executa ação de reflexão
    async fn execute_reflect(&mut self, gap_questions: Vec<String>, think: String) -> StepResult {
        log::info!("🤔 Refletindo... {} novas perguntas", gap_questions.len());

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
        _think: String,
    ) -> StepResult {
        use crate::evaluation::{EvaluationPipeline, EvaluationType};

        log::info!("✍️  Avaliando resposta...");

        // Resposta imediata no step 1 = pergunta trivial
        if self.context.total_step == 1 && self.context.allow_direct_answer {
            self.answer_count += 1;
            self.emit(AgentProgress::Success("✅ Resposta trivial gerada".into()));
            self.emit_persona_stats(false);
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
            .evaluate_sequential(
                &self.context.original_question,
                &answer,
                &eval_context,
                &eval_types,
            )
            .await;

        if result.overall_passed {
            // Incrementar contador de respostas
            self.answer_count += 1;

            log::info!("✅ Resposta aprovada na avaliação!");

            // Emitir sucesso e atualizar persona (não ativa pois vai finalizar)
            self.emit(AgentProgress::Success(format!(
                "✅ Resposta #{} aprovada!",
                self.answer_count
            )));
            self.emit_persona_stats(false);

            StepResult::Completed(AnswerResult {
                answer,
                references,
                trivial: false,
            })
        } else {
            log::info!("❌ Resposta reprovada, continuando pesquisa...");
            // Adicionar falha como conhecimento
            let failed_type = result.failed_at.unwrap_or(EvaluationType::Definitive);
            let reasoning = result
                .results
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

        self.context.diary.push(DiaryEntry::Coding { code, think });

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Força uma resposta em Beast Mode
    async fn force_answer(&mut self) -> Result<AnswerResult, AgentError> {
        let prompt = AgentPrompt {
            system: "You MUST provide an answer now. No more searching or reflecting. \
                     Be pragmatic and use what you know."
                .into(),
            user: format!(
                "Question: {}\n\nKnowledge:\n{}\n\nProvide your best answer.",
                self.context.original_question,
                self.format_knowledge()
            ),
            diary: self.context.diary.clone(),
        };

        let response = self
            .llm_client
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
        // Usar tokens do tracker (rastreados durante execução)
        let token_usage = self.token_tracker.get_total_usage();

        log::info!(
            "📊 Token usage final: {} prompt + {} completion = {} total ({:.1}% do budget)",
            token_usage.prompt_tokens,
            token_usage.completion_tokens,
            token_usage.total_tokens,
            self.token_tracker.budget_used_percentage() * 100.0
        );

        // Calcular tempos
        let total_time_ms = self.start_time.elapsed().as_millis();
        let search_time_ms: u128 = self.timing_stats.search_times.iter().sum();
        let read_time_ms: u128 = self.timing_stats.read_times.iter().sum();
        let llm_time_ms: u128 = self.timing_stats.llm_times.iter().sum();

        match self.state {
            AgentState::Completed {
                answer,
                references,
                trivial,
            } => ResearchResult {
                success: true,
                answer: Some(answer),
                references,
                trivial,
                token_usage,
                visited_urls: self.context.visited_urls,
                error: None,
                total_time_ms,
                search_time_ms,
                read_time_ms,
                llm_time_ms,
            },
            AgentState::Failed {
                reason,
                partial_knowledge: _partial_knowledge,
            } => ResearchResult {
                success: false,
                answer: None,
                references: vec![],
                trivial: false,
                token_usage,
                visited_urls: self.context.visited_urls,
                error: Some(reason),
                total_time_ms,
                search_time_ms,
                read_time_ms,
                llm_time_ms,
            },
            _ => unreachable!("build_result called in non-terminal state"),
        }
    }

    // Métodos auxiliares...

    /// Atualiza o budget_used no estado baseado no token_tracker
    fn update_budget_used(&mut self) {
        if let AgentState::Processing { budget_used, .. } = &mut self.state {
            *budget_used = self.token_tracker.budget_used_percentage();
        }
    }

    /// Retorna estatísticas do token tracker
    pub fn get_token_stats(&self) -> TrackerStats {
        self.token_tracker.stats()
    }

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
