// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEEP RESEARCH AGENT - MÁQUINA DE ESTADOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod actions;
/// Módulo para análise e diagnóstico do agente durante a execução.
/// Útil para debugging e geração de relatórios sobre decisões e erros do agente.
pub mod agent_analyzer;
/// Módulo para integração com plataformas de chatbot externas.
/// Define a trait ChatbotAdapter para DigiSac, Suri, Parrachos, etc.
pub mod chatbot;
mod context;
/// Módulo para acesso ao histórico de sessões anteriores.
/// Suporta múltiplos backends: local (JSON), PostgreSQL, Qdrant.
pub mod history;
/// Módulo para interação bidirecional usuário-agente.
/// Compatível com OpenAI Responses API (input_required state).
pub mod interaction;
mod permissions;
/// Módulo para execução segura de código JavaScript via Boa Engine.
/// Permite ao agente gerar e executar código em sandbox isolado.
pub mod sandbox;
mod state;

pub use actions::*;
pub use agent_analyzer::AgentAnalysis;
pub use chatbot::{
    ButtonType, ChatbotAdapter, ChatbotError, ConnectionStatus, MessageButton, MockChatbotAdapter,
    RichMessage, UserMetadata,
};
pub use context::*;
pub use history::{HistoryQuery, HistorySearchResult, HistoryService, SessionSummary};
pub use interaction::{
    create_interaction_channels, InteractionError, InteractionHub, PendingQuestion, QuestionType,
    UserResponse,
};
pub use permissions::*;
pub use sandbox::{
    CodeSandbox, PythonSandbox, SandboxContext, SandboxError, SandboxLanguage, SandboxResult,
    UnifiedSandbox,
};
pub use state::*;

use crate::llm::LlmClient;
use crate::search::SearchClient;
use crate::types::*;
use crate::utils::{
    ActionTimer, ReferenceBuilder, ReferenceBuilderConfig, TimingStats, TokenTracker, TrackerStats,
};
use std::sync::Arc;
use tokio::sync::mpsc;

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
        /// Nome da persona ativa
        name: String,
        /// Número de buscas realizadas
        searches: usize,
        /// Número de leituras realizadas
        reads: usize,
        /// Número de respostas geradas
        answers: usize,
        /// Total de tokens consumidos
        tokens: u64,
        /// Se a persona está ativa no momento
        is_active: bool,
    },
    /// URL visitada com sucesso
    VisitedUrl(String),
    /// Início de um batch paralelo
    BatchStart {
        /// ID único do batch
        batch_id: String,
        /// Tipo do batch (ex: "WebRead", "Search")
        batch_type: String,
        /// Número de tarefas no batch
        task_count: usize,
    },
    /// Atualização de tarefa paralela
    TaskUpdate {
        /// ID único da tarefa
        task_id: String,
        /// ID do batch pai
        batch_id: String,
        /// Tipo da tarefa
        task_type: String,
        /// Descrição/URL
        description: String,
        /// Informações sobre dados alocados
        data_info: String,
        /// Status: "pending", "running", "completed", "failed:msg"
        status: String,
        /// Tempo de execução em ms
        elapsed_ms: u128,
        /// ID da thread (se disponível)
        thread_id: Option<String>,
        /// Progresso em porcentagem (0-100)
        progress: u8,
        /// Método de leitura: "jina", "rust_local", "file", "unknown"
        read_method: String,
        /// Bytes processados
        bytes_processed: usize,
        /// Total de bytes esperado
        bytes_total: usize,
    },
    /// Fim de um batch paralelo
    BatchEnd {
        /// ID do batch finalizado
        batch_id: String,
        /// Tempo total em ms
        total_ms: u128,
        /// Tarefas bem sucedidas
        success_count: usize,
        /// Tarefas que falharam
        fail_count: usize,
    },
    /// Query expandida por uma persona específica
    PersonaQuery {
        /// Nome da persona que gerou a query
        persona: String,
        /// Query original
        original: String,
        /// Query expandida
        expanded: String,
        /// Peso da query (prioridade)
        weight: f32,
    },
    /// Resultado de deduplicação de queries
    Dedup {
        /// Queries originais (antes da dedup)
        original_count: usize,
        /// Queries únicas (após dedup)
        unique_count: usize,
        /// Queries removidas (duplicadas)
        removed_count: usize,
        /// Threshold de similaridade usado
        threshold: f32,
    },
    /// Início de validação fast-fail
    ValidationStart {
        /// Tipos de validação que serão executados
        eval_types: Vec<String>,
    },
    /// Resultado de uma etapa de validação
    ValidationStep {
        /// Tipo de avaliação
        eval_type: String,
        /// Se passou (true) ou falhou (false)
        passed: bool,
        /// Confiança (0.0 - 1.0)
        confidence: f32,
        /// Razão/explicação
        reasoning: String,
        /// Duração em ms
        duration_ms: u128,
    },
    /// Fim de validação (com resultado geral)
    ValidationEnd {
        /// Se todas as validações passaram
        overall_passed: bool,
        /// Em qual tipo falhou (se falhou)
        failed_at: Option<String>,
        /// Total de validações executadas
        total_evals: usize,
        /// Total de validações aprovadas
        passed_evals: usize,
    },
    /// Análise de erros iniciada em background (AgentAnalyzer)
    AgentAnalysisStarted {
        /// Número de falhas consecutivas que dispararam a análise
        failures_count: usize,
        /// Número de entradas no diário sendo analisadas
        diary_entries: usize,
    },
    /// Análise de erros concluída (AgentAnalyzer)
    AgentAnalysisCompleted {
        /// Resumo cronológico das ações
        recap: String,
        /// Identificação do problema
        blame: String,
        /// Sugestões de melhoria
        improvement: String,
        /// Tempo de execução em ms
        duration_ms: u128,
    },
    /// Agente fez uma pergunta ao usuário
    ///
    /// Compatível com OpenAI Responses API (input_required).
    AgentQuestion {
        /// ID único da pergunta
        question_id: String,
        /// Tipo da pergunta
        question_type: String,
        /// Texto da pergunta
        question: String,
        /// Opções de resposta (se aplicável)
        options: Option<Vec<String>>,
        /// Se é blocking (agente pausado)
        is_blocking: bool,
    },
    /// Resposta do usuário recebida
    UserResponseReceived {
        /// ID da pergunta respondida (None se espontânea)
        question_id: Option<String>,
        /// Conteúdo da resposta
        response: String,
        /// Se foi espontânea
        was_spontaneous: bool,
    },
    /// Agente retomando após receber input do usuário
    ResumedAfterInput {
        /// ID da pergunta que foi respondida
        question_id: String,
    },
    /// Sandbox iniciou execução
    SandboxStart {
        /// Problema/tarefa sendo resolvido
        problem: String,
        /// Máximo de tentativas configurado
        max_attempts: usize,
        /// Timeout em ms
        timeout_ms: u64,
        /// Linguagem de programação (JavaScript, Python, Auto)
        language: String,
    },
    /// Sandbox - tentativa de execução
    SandboxAttempt {
        /// Número da tentativa atual (1-based)
        attempt: usize,
        /// Máximo de tentativas
        max_attempts: usize,
        /// Código gerado (truncado para exibição)
        code_preview: String,
        /// Status: "generating", "executing", "success", "error"
        status: String,
        /// Mensagem de erro (se aplicável)
        error: Option<String>,
    },
    /// Sandbox concluiu execução
    SandboxComplete {
        /// Se foi bem-sucedido
        success: bool,
        /// Output da execução (se sucesso)
        output: Option<String>,
        /// Erro (se falha)
        error: Option<String>,
        /// Número total de tentativas
        attempts: usize,
        /// Tempo total de execução em ms
        execution_time_ms: u64,
        /// Código final executado (truncado)
        code_preview: String,
        /// Linguagem de programação usada
        language: String,
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
    /// Habilita modo de comparação Jina vs Rust+OpenAI
    enable_comparative_read: bool,
    /// Contagem de vitórias Jina
    jina_wins: usize,
    /// Contagem de vitórias Rust
    rust_wins: usize,
    /// Empates
    ties: usize,
    /// Idioma das respostas
    response_language: crate::types::Language,
    /// Contador de falhas consecutivas de validação
    consecutive_failures: usize,
    /// Contador de análises realizadas (máximo 3 por sessão)
    analysis_count: usize,
    /// Canal para receber análises do AgentAnalyzer (non-blocking)
    analysis_rx: Option<mpsc::Receiver<AgentAnalysis>>,
    /// Hub de interação para comunicação bidirecional com usuário
    ///
    /// Compatível com OpenAI Responses API (input_required state).
    interaction_hub: InteractionHub,
    /// Canal para enviar respostas do usuário para o hub
    user_response_tx: Option<mpsc::Sender<UserResponse>>,
}

impl DeepResearchAgent {
    /// Cria um novo agente com os clientes fornecidos
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        search_client: Arc<dyn SearchClient>,
        token_budget: Option<u64>,
    ) -> Self {
        // Carregar idioma da variável de ambiente
        let response_language = std::env::var("RESPONSE_LANGUAGE")
            .map(|s| crate::types::Language::from_str(&s))
            .unwrap_or(crate::types::Language::Portuguese); // Padrão: Português

        log::info!("🌐 Idioma de resposta: {}", response_language.display_name());

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
            enable_comparative_read: false,
            jina_wins: 0,
            rust_wins: 0,
            ties: 0,
            response_language,
            consecutive_failures: 0,
            analysis_count: 0,
            analysis_rx: None,
            interaction_hub: InteractionHub::new(),
            user_response_tx: None,
        }
    }

    /// Define o idioma das respostas
    pub fn with_response_language(mut self, language: crate::types::Language) -> Self {
        self.response_language = language;
        log::info!("🌐 Idioma de resposta definido: {}", language.display_name());
        self
    }

    /// Configura callback de progresso para updates em tempo real
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Habilita modo de comparação Jina vs Rust local
    pub fn with_comparative_read(mut self, enable: bool) -> Self {
        self.enable_comparative_read = enable;
        if enable {
            log::info!("🔬 Modo de comparação ATIVADO: Jina vs Rust local");
        }
        self
    }

    /// Configura canais de interação para comunicação com usuário
    ///
    /// Retorna um sender para enviar respostas do usuário e um receiver
    /// para receber perguntas do agente.
    ///
    /// # Returns
    /// - `mpsc::Sender<UserResponse>`: Para enviar respostas do usuário
    /// - `mpsc::Receiver<PendingQuestion>`: Para receber perguntas do agente
    pub fn with_interaction_channels(
        mut self,
        buffer_size: usize,
    ) -> (Self, mpsc::Sender<UserResponse>, mpsc::Receiver<PendingQuestion>) {
        let (response_tx, question_rx, hub) = create_interaction_channels(buffer_size);
        self.interaction_hub = hub;
        self.user_response_tx = Some(response_tx.clone());
        (self, response_tx, question_rx)
    }

    /// Envia uma resposta do usuário para o agente
    ///
    /// Usado por interfaces externas (TUI, Chatbot) para enviar
    /// respostas ou mensagens espontâneas.
    pub async fn send_user_response(&mut self, response: UserResponse) {
        self.interaction_hub.receive_response(response);
    }

    /// Verifica se o agente está aguardando input do usuário
    pub fn is_waiting_for_user(&self) -> bool {
        self.state.is_input_required()
    }

    /// Retorna a pergunta pendente atual (se houver)
    pub fn get_pending_question(&self) -> Option<&PendingQuestion> {
        self.interaction_hub.get_blocking_question()
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
                        StepResult::InputRequired {
                            question_id,
                            question,
                            question_type,
                            options,
                        } => {
                            // Agente precisa de input do usuário - entrar em estado de espera
                            log::info!("⏸️ Agente entrando em estado de espera por input do usuário");
                            self.state = AgentState::InputRequired {
                                question_id,
                                question,
                                question_type,
                                options,
                            };
                            // CONTINUAR o loop - o próximo match de AgentState::InputRequired
                            // vai aguardar a resposta via canal
                            continue;
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

                // Estado de espera por input do usuário
                AgentState::InputRequired { ref question_id, .. } => {
                    // Aguardar resposta do usuário via canal
                    log::debug!("Agente em estado InputRequired - aguardando resposta via canal");

                    // Primeiro fazer polling para ver se já chegou algo
                    self.interaction_hub.poll_responses();

                    // Verificar se já temos resposta para esta pergunta
                    if let Some(response) = self.interaction_hub.find_response_for(question_id) {
                        log::info!("📥 Resposta encontrada: {}", response.content);
                        self.process_user_response(response).await;
                        continue;
                    }

                    // Esperar resposta com timeout de 60 segundos
                    match self.interaction_hub.wait_for_response(question_id, Some(60)).await {
                        Ok(response) => {
                            log::info!("📥 Resposta recebida: {}", response.content);
                            self.process_user_response(response).await;
                            // Continuar o loop após processar resposta
                            continue;
                        }
                        Err(InteractionError::Timeout) => {
                            // Timeout - fazer polling novamente e continuar esperando
                            log::debug!("Timeout aguardando resposta, continuando...");
                            continue;
                        }
                        Err(InteractionError::ChannelClosed) => {
                            // Canal fechado - interface provavelmente foi fechada
                            log::warn!("Canal de resposta fechado - abortando");
                            self.state = AgentState::Failed {
                                reason: "Canal de comunicação com interface fechado".into(),
                                partial_knowledge: self.context.knowledge.clone(),
                            };
                        }
                        Err(e) => {
                            log::error!("Erro aguardando resposta: {:?}", e);
                            continue;
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
        // 0a. Processar mensagens assíncronas do usuário (non-blocking)
        self.poll_user_messages();

        // 0b. Verificar se há análise do AgentAnalyzer pronta (non-blocking)
        if let Some(rx) = &mut self.analysis_rx {
            match rx.try_recv() {
                Ok(analysis) => {
                    log::info!(
                        "🔬 AgentAnalyzer: Análise recebida em {}ms",
                        analysis.duration_ms.unwrap_or(0)
                    );

                    // Adicionar hint de melhoria ao contexto
                    self.context.add_improvement_hint(analysis.improvement.clone());
                    self.context.set_agent_analysis(analysis.clone());

                    // Emitir evento de conclusão
                    self.emit(AgentProgress::AgentAnalysisCompleted {
                        recap: analysis.recap.clone(),
                        blame: analysis.blame.clone(),
                        improvement: analysis.improvement.clone(),
                        duration_ms: analysis.duration_ms.unwrap_or(0),
                    });

                    self.emit(AgentProgress::Success(format!(
                        "🔬 Análise concluída: {}",
                        analysis.improvement.chars().take(80).collect::<String>()
                    )));

                    // Limpar canal após consumir
                    self.analysis_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // Análise ainda em andamento, continuar normalmente
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Canal fechado (análise falhou ou timeout), limpar
                    self.analysis_rx = None;
                }
            }
        }

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

                // Se LLM escolheu URLs já visitadas, usar RERANK para selecionar as melhores
                if new_urls.is_empty() {
                    let msg = "Usando Jina Rerank para selecionar URLs mais relevantes...";
                    log::info!("🔄 {}", msg);
                    self.emit(AgentProgress::Info(msg.into()));

                    // Pegar URLs disponíveis (não visitadas, não ruins)
                    let available_snippets: Vec<_> = self
                        .context
                        .collected_urls
                        .iter()
                        .filter(|u| {
                            !self.context.is_url_visited(&u.url) && !self.context.is_url_bad(&u.url)
                        })
                        .cloned()
                        .collect();

                    if !available_snippets.is_empty() {
                        // Usar rerank para ordenar por relevância à pergunta
                        let query = &self.context.original_question;
                        let reranked = self.search_client.rerank(query, &available_snippets).await;

                        // Pegar as top N URLs mais relevantes
                        new_urls = reranked
                            .iter()
                            .take(MAX_URLS_PER_STEP)
                            .map(|s| s.url.clone())
                            .collect();

                        if !new_urls.is_empty() {
                            self.emit(AgentProgress::Success(format!(
                                "✅ Rerank selecionou {} URLs (top score: {:.0}%)",
                                new_urls.len(),
                                reranked.first().map(|s| s.jina_rerank_boost * 100.0).unwrap_or(0.0)
                            )));
                        }
                    }
                }

                // Fallback: pegar próximas disponíveis sem rerank
                if new_urls.is_empty() {
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
                // Tentar construir referências semânticas usando embeddings
                let (final_answer, final_references) =
                    self.build_semantic_references(&answer, references).await;

                self.emit(AgentProgress::Success(format!(
                    "✍️ Gerando resposta ({} chars, {} refs)",
                    final_answer.len(),
                    final_references.len()
                )));
                log::info!(
                    "✍️ Resposta proposta ({} chars, {} refs)",
                    final_answer.len(),
                    final_references.len()
                );
                log::info!("💭 Raciocínio: {}", think);
                log::info!("📝 Resposta:\n{}", final_answer);
                if !final_references.is_empty() {
                    log::info!("📚 Referências:");
                    for r in &final_references {
                        log::info!("   - {} ({})", r.title, r.url);
                    }
                }
                self.execute_answer(final_answer, final_references, think).await
            }
            AgentAction::Coding { problem, context_vars: _, language, think } => {
                self.execute_coding(problem, language, think).await
            }
            AgentAction::History {
                count,
                filter,
                think,
            } => self.execute_history(count, filter, think).await,
            AgentAction::AskUser {
                question_type,
                question,
                options,
                is_blocking,
                think,
            } => {
                self.emit(AgentProgress::Info(format!("❓ Perguntando ao usuário: {}", question)));
                log::info!("❓ Perguntando ao usuário: {}", question);
                log::debug!("💭 Raciocínio: {}", think);
                self.execute_ask_user(question_type, question, options, is_blocking, think).await
            }
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
        let language_instruction = self.response_language.llm_instruction();

        let mut prompt = format!(
            r#"You are a research agent. Your goal is to find accurate information efficiently.

{}

CRITICAL RULES:
1. NEVER read the same URL twice - pick DIFFERENT URLs from the available list
2. After reading 3-5 different URLs, try to ANSWER the question
3. If you have enough information, use ANSWER action immediately
4. Only use SEARCH if you need completely different information

Available actions:
"#,
            language_instruction
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
        if permissions.history {
            prompt.push_str("- HISTORY: Access previous research sessions for context\n");
            prompt.push_str("  → Use when user asks about 'what was researched before' or 'summarize previous'\n");
            prompt.push_str("  → Loads summaries of past questions/answers to provide context\n");
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

        // Adicionar hints de melhoria do AgentAnalyzer (se disponíveis)
        if self.context.has_improvement_hints() {
            prompt.push_str("\n## 💡 IMPROVEMENT HINTS (from previous error analysis):\n");
            prompt.push_str("Based on analysis of your previous attempts, consider these improvements:\n");
            for (i, hint) in self.context.improvement_hints.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, hint));
            }
            prompt.push_str("\nPlease take these suggestions into account in your next actions.\n");
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

    /// Executa ação de busca (em paralelo)
    async fn execute_search(&mut self, queries: Vec<SerpQuery>, think: String) -> StepResult {
        use crate::personas::PersonaOrchestrator;
        let search_timer = ActionTimer::start("Search");

        // Expandir queries com personas cognitivas
        let orchestrator = PersonaOrchestrator::new();
        let context = self.build_query_context();
        let original_queries: Vec<String> = queries.iter().map(|q| q.q.clone()).collect();
        let expanded = orchestrator.expand_batch(&original_queries, &context);

        // 🎭 Emitir eventos para cada query expandida por persona
        self.emit(AgentProgress::Info(format!(
            "🎭 Expandindo {} queries com {} personas...",
            original_queries.len(),
            orchestrator.persona_count()
        )));

        for wq in &expanded {
            // Encontrar a query original correspondente (aproximada)
            let original = original_queries
                .iter()
                .find(|oq| wq.query.q.contains(oq.as_str()) || oq.contains(&wq.query.q))
                .map(|s| s.clone())
                .unwrap_or_else(|| original_queries.first().cloned().unwrap_or_default());

            self.emit(AgentProgress::PersonaQuery {
                persona: wq.source_persona.to_string(),
                original: original.chars().take(50).collect(),
                expanded: wq.query.q.chars().take(80).collect(),
                weight: wq.weight,
            });
        }

        // Deduplicar contra queries existentes usando SIMD + embeddings
        let original_count = expanded.len();
        let (unique, removed_count, new_embeddings) = self
            .dedup_queries_with_embeddings(expanded.iter().map(|wq| wq.query.clone()).collect())
            .await;
        let unique_count = unique.len();

        // 🔄 Emitir evento de deduplicação
        self.emit(AgentProgress::Dedup {
            original_count,
            unique_count,
            removed_count,
            threshold: 0.86, // Threshold SIMD
        });

        if removed_count > 0 {
            self.emit(AgentProgress::Info(format!(
                "🔄 SIMD Dedup: {} → {} queries ({} duplicadas semânticas)",
                original_count, unique_count, removed_count
            )));
        }

        let num_queries = unique.len();
        log::info!("🔍 Executando {} buscas em PARALELO...", num_queries);
        self.emit(AgentProgress::Info(format!(
            "⚡ Iniciando {} buscas paralelas",
            num_queries
        )));

        // Executar buscas em paralelo
        let results = self.search_client.search_batch(&unique).await;

        // Salvar embeddings das queries executadas para futuras deduplicações
        let executed_query_texts: Vec<String> = unique.iter().map(|q| q.q.clone()).collect();
        self.context.add_executed_queries(executed_query_texts, new_embeddings);

        // Contar sucessos e erros
        let mut success_count = 0;
        let mut error_count = 0;
        let mut total_urls = 0;

        // Adicionar URLs ao contexto
        for result in results {
            match result {
                Ok(r) => {
                    total_urls += r.urls.len();
                self.context.add_urls(r.urls);
                self.context.add_snippets(r.snippets);
                    success_count += 1;
                }
                Err(_) => {
                    error_count += 1;
                }
            }
        }

        let search_time = search_timer.stop();
        self.timing_stats.add_search_time(search_time);

        // Incrementar contador de buscas
        self.search_count += 1;

        // Calcular tempo médio por query (demonstra eficiência do paralelismo)
        let avg_time_per_query = if num_queries > 0 {
            search_time as f64 / num_queries as f64
        } else {
            0.0
        };

        log::info!(
            "🔍 Busca PARALELA concluída: {} queries em {}ms (média: {:.1}ms/query) | {} URLs encontradas | ✅ {} ok | ❌ {} erros",
            num_queries,
            search_time,
            avg_time_per_query,
            total_urls,
            success_count,
            error_count
        );

        // Emitir log detalhado sobre paralelismo
        self.emit(AgentProgress::Success(format!(
            "⚡ Busca #{} paralela: {} queries em {:.2}s ({:.0}ms/q) | {} URLs | ✅{} ❌{}",
            self.search_count,
            num_queries,
            search_time as f64 / 1000.0,
            avg_time_per_query,
            total_urls,
            success_count,
            error_count
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

    /// Executa ação de leitura de URL (em paralelo)
    async fn execute_read(&mut self, urls: Vec<Url>, think: String) -> StepResult {
        use crate::utils::{FileReader, FileType};
        use futures::future::join_all;
        use uuid::Uuid;

        let read_timer = ActionTimer::start("Read URLs");
        let urls_to_read: Vec<_> = urls.into_iter().take(MAX_URLS_PER_STEP).collect();
        let num_urls = urls_to_read.len();

        // Gerar batch ID único
        let batch_id = Uuid::new_v4().to_string();
        let batch_type = "WebRead".to_string();

        log::info!("📖 Lendo {} URLs em PARALELO (Batch: {})...", num_urls, &batch_id[..8]);

        // Emitir início do batch
        self.emit(AgentProgress::BatchStart {
            batch_id: batch_id.clone(),
            batch_type: batch_type.clone(),
            task_count: num_urls,
        });
        self.emit(AgentProgress::Info(format!(
            "⚡ Batch {} iniciado: {} tarefas paralelas",
            &batch_id[..8], num_urls
        )));

        // Separar URLs de arquivo e URLs web
        let file_reader = FileReader::new();
        let mut file_urls = Vec::new();
        let mut web_urls = Vec::new();

        for url in &urls_to_read {
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
                file_urls.push((url.clone(), file_type));
            } else {
                web_urls.push(url.clone());
            }
        }

        let mut success_count = 0;
        let mut error_count = 0;

        // Ler arquivos em paralelo
        if !file_urls.is_empty() {
            log::info!("📥 Lendo {} arquivos em paralelo...", file_urls.len());

            // Criar task_ids para arquivos antes de iniciar
            let file_task_ids: std::collections::HashMap<String, String> = file_urls
                .iter()
                .map(|(url, _)| (url.clone(), Uuid::new_v4().to_string()))
                .collect();

            // Atualizar status para running com progresso inicial
            for (url, _) in &file_urls {
                if let Some(task_id) = file_task_ids.get(url) {
                    self.emit(AgentProgress::TaskUpdate {
                        task_id: task_id.clone(),
                        batch_id: batch_id.clone(),
                        task_type: "FileRead".to_string(),
                        description: url.to_string(),
                        data_info: "Lendo arquivo...".to_string(),
                        status: "running".to_string(),
                        elapsed_ms: 0,
                        thread_id: Some(format!("{:?}", std::thread::current().id())),
                        progress: 10,
                        read_method: "file".to_string(),
                        bytes_processed: 0,
                        bytes_total: 0,
                    });
                }
            }

            let file_read_start = std::time::Instant::now();
            let file_futures: Vec<_> = file_urls
                .iter()
                .map(|(url, _)| file_reader.read_url(url))
                .collect();

            let file_results = join_all(file_futures).await;
            let file_read_time = file_read_start.elapsed().as_millis();
            let avg_file_time = file_read_time / file_urls.len().max(1) as u128;

            for (result, (url, file_type)) in file_results.into_iter().zip(file_urls.iter()) {
                let task_id = file_task_ids.get(url).cloned().unwrap_or_default();
                match result {
                    Ok(file_content) => {
                        log::info!(
                            "✅ Arquivo lido: {} palavras | {} bytes | {}ms",
                            file_content.word_count,
                            file_content.size_bytes,
                            avg_file_time
                        );
                        // Emitir tarefa completada
                        self.emit(AgentProgress::TaskUpdate {
                            task_id: task_id.clone(),
                            batch_id: batch_id.clone(),
                            task_type: "FileRead".to_string(),
                            description: url.to_string(),
                            data_info: format!("{} palavras | {} bytes | {}ms", file_content.word_count, file_content.size_bytes, avg_file_time),
                            status: "completed".to_string(),
                            elapsed_ms: avg_file_time,
                            thread_id: Some(format!("{:?}", std::thread::current().id())),
                            progress: 100,
                            read_method: "file".to_string(),
                            bytes_processed: file_content.size_bytes as usize,
                            bytes_total: file_content.size_bytes as usize,
                        });
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
                                answer_chunk: None,
                                answer_position: None,
                            }],
                        });
                        self.context.visited_urls.push(url.clone());
                        self.emit(AgentProgress::VisitedUrl(url.to_string()));
                        success_count += 1;
                    }
                    Err(e) => {
                        log::warn!("❌ Falha ao ler arquivo {}: {}", url, e);
                        // Emitir tarefa falha
                        self.emit(AgentProgress::TaskUpdate {
                            task_id: task_id.clone(),
                            batch_id: batch_id.clone(),
                            task_type: "FileRead".to_string(),
                            description: url.to_string(),
                            data_info: format!("Erro: {}", e),
                            status: "failed".to_string(),
                            elapsed_ms: avg_file_time,
                            thread_id: Some(format!("{:?}", std::thread::current().id())),
                            progress: 100,
                            read_method: "file".to_string(),
                            bytes_processed: 0,
                            bytes_total: 0,
                        });
                        self.context.bad_urls.push(url.clone());
                        error_count += 1;
                    }
                }
            }
        }

        // Ler URLs web em paralelo
        if !web_urls.is_empty() {
            // Criar task_ids para cada URL (para rastrear)
            let url_task_ids: std::collections::HashMap<String, String> = web_urls
                .iter()
                .map(|url| (url.clone(), Uuid::new_v4().to_string()))
                .collect();

            // Emitir tarefas pendentes para URLs web
            let read_method_str = if self.enable_comparative_read { "jina+rust" } else { "jina" };
            for (url, task_id) in &url_task_ids {
                self.emit(AgentProgress::TaskUpdate {
                    task_id: task_id.clone(),
                    batch_id: batch_id.clone(),
                    task_type: "WebRead".to_string(),
                    description: url.to_string(),
                    data_info: format!("Iniciando via {}...", read_method_str),
                    status: "running".to_string(),
                    elapsed_ms: 0,
                    thread_id: Some(format!("{:?}", std::thread::current().id())),
                    progress: 0,
                    read_method: read_method_str.to_string(),
                    bytes_processed: 0,
                    bytes_total: 0,
                });
            }

            if self.enable_comparative_read {
                // Modo de comparação: Jina vs Rust local
                log::info!("🔬 Lendo {} URLs web em modo COMPARATIVO (Jina vs Rust)...", web_urls.len());
                self.emit(AgentProgress::Info(format!(
                    "🔬 Comparando Jina vs Rust local para {} URLs",
                    web_urls.len()
                )));

                let comparative_results = self.search_client.read_urls_comparative_batch(&web_urls).await;

                for result in comparative_results {
                    let task_id = url_task_ids.get(&result.url).cloned().unwrap_or_default();
                    let elapsed_ms = result.jina_time_ms.max(result.rust_time_ms) as u128;

                    // Usar o resultado mais rápido que não tenha erro
                    let (content, source) = match &result.faster {
                        crate::search::ReadMethod::RustLocal if result.rust_result.is_some() => {
                            self.rust_wins += 1;
                            (result.rust_result.clone().unwrap(), "rust_local")
                        }
                        crate::search::ReadMethod::Jina if result.jina_result.is_some() => {
                            self.jina_wins += 1;
                            (result.jina_result.clone().unwrap(), "jina")
                        }
                        crate::search::ReadMethod::Tie => {
                            self.ties += 1;
                            // Em caso de empate, preferir Jina
                            if let Some(jina) = result.jina_result.clone() {
                                (jina, "jina")
                            } else if let Some(rust) = result.rust_result.clone() {
                                (rust, "rust_local")
                            } else {
                                // Emitir TaskUpdate de falha
                                self.emit(AgentProgress::TaskUpdate {
                                    task_id: task_id.clone(),
                                    batch_id: batch_id.clone(),
                                    task_type: "WebRead".to_string(),
                                    description: result.url.clone(),
                                    data_info: "Ambos métodos falharam".to_string(),
                                    status: "failed".to_string(),
                                    elapsed_ms,
                                    thread_id: None,
                                    progress: 100,
                                    read_method: "failed".to_string(),
                                    bytes_processed: 0,
                                    bytes_total: 0,
                                });
                                log::warn!("❌ Ambos métodos falharam para {}", result.url);
                                self.context.bad_urls.push(result.url.clone());
                                error_count += 1;
                                continue;
                            }
                        }
                        _ => {
                            // Fallback para o que estiver disponível
                            if let Some(jina) = result.jina_result.clone() {
                                self.jina_wins += 1;
                                (jina, "jina")
                            } else if let Some(rust) = result.rust_result.clone() {
                                self.rust_wins += 1;
                                (rust, "rust_local")
                            } else {
                                // Emitir TaskUpdate de falha
                                self.emit(AgentProgress::TaskUpdate {
                                    task_id: task_id.clone(),
                                    batch_id: batch_id.clone(),
                                    task_type: "WebRead".to_string(),
                                    description: result.url.clone(),
                                    data_info: "Nenhum método disponível".to_string(),
                                    status: "failed".to_string(),
                                    elapsed_ms,
                                    thread_id: None,
                                    progress: 100,
                                    read_method: "failed".to_string(),
                                    bytes_processed: 0,
                                    bytes_total: 0,
                                });
                                log::warn!("❌ Ambos métodos falharam para {}", result.url);
                                self.context.bad_urls.push(result.url.clone());
                                error_count += 1;
                                continue;
                            }
                        }
                    };

                    // Calcular bytes processados
                    let bytes_processed = content.text.len();

                    // Emitir TaskUpdate de sucesso com método real usado
                    self.emit(AgentProgress::TaskUpdate {
                        task_id: task_id.clone(),
                        batch_id: batch_id.clone(),
                        task_type: "WebRead".to_string(),
                        description: result.url.clone(),
                        data_info: format!("{}ms via {}", elapsed_ms, source),
                        status: "completed".to_string(),
                        elapsed_ms,
                        thread_id: None,
                        progress: 100,
                        read_method: source.to_string(),
                        bytes_processed,
                        bytes_total: bytes_processed,
                    });

                    // Log de comparação
                    log::info!(
                        "📊 {} | Jina: {}ms | Rust: {}ms | Diff: {}ms | Usado: {}",
                        result.url,
                        result.jina_time_ms,
                        result.rust_time_ms,
                        result.speed_diff_ms,
                        source
                    );
                    self.emit(AgentProgress::Info(format!(
                        "📊 Jina: {}ms vs Rust: {}ms (diff: {}ms) → usado: {}",
                        result.jina_time_ms,
                        result.rust_time_ms,
                        result.speed_diff_ms,
                        source
                    )));

                    self.context.add_knowledge(KnowledgeItem {
                        question: self.context.current_question().to_string(),
                        answer: content.text,
                        item_type: KnowledgeType::Url,
                        references: vec![Reference {
                            url: result.url.to_string(),
                            title: content.title,
                            exact_quote: None,
                            relevance_score: None,
                            answer_chunk: None,
                            answer_position: None,
                        }],
                    });
                    self.context.visited_urls.push(result.url.clone());
                    self.emit(AgentProgress::VisitedUrl(result.url.to_string()));
                    success_count += 1;
                }

                // Resumo de comparação
                let total_comparisons = self.jina_wins + self.rust_wins + self.ties;
                log::info!(
                    "🏆 Placar comparativo: Jina {} | Rust {} | Empates {} | Total: {}",
                    self.jina_wins, self.rust_wins, self.ties, total_comparisons
                );
                self.emit(AgentProgress::Success(format!(
                    "🏆 Placar: Jina {} vs Rust {} (empates: {})",
                    self.jina_wins, self.rust_wins, self.ties
                )));
            } else {
                // Modo normal: Rust primeiro, Jina fallback - COM PROGRESSO EM TEMPO REAL
                log::info!("🌐 Lendo {} URLs web em paralelo (Rust→Jina com progresso)...", web_urls.len());
                let read_start = std::time::Instant::now();

                // Criar progresso compartilhado para cada URL
                use std::sync::{Arc, atomic::{AtomicU8, Ordering}};
                let progress_map: std::collections::HashMap<String, Arc<AtomicU8>> = web_urls
                    .iter()
                    .map(|url| (url.clone(), Arc::new(AtomicU8::new(0))))
                    .collect();

                // Canal para emitir eventos de progresso
                let (progress_tx, _progress_rx) = tokio::sync::mpsc::channel::<(String, u8)>(100);

                // Clonar referências para o monitor
                let progress_map_clone = progress_map.clone();
                let url_task_ids_clone = url_task_ids.clone();
                let batch_id_clone = batch_id.clone();
                let progress_callback = self.progress_callback.clone();

                // Spawnar monitor de progresso que emite atualizações a cada 200ms
                let monitor_handle = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
                    let mut last_progress: std::collections::HashMap<String, u8> = std::collections::HashMap::new();

                    loop {
                        interval.tick().await;

                        let mut all_done = true;
                        for (url, progress) in &progress_map_clone {
                            let current = progress.load(Ordering::Relaxed);
                            let last = last_progress.get(url).copied().unwrap_or(0);

                            // Só emitir se progresso mudou e não está 100%
                            if current != last && current < 100 {
                                last_progress.insert(url.clone(), current);

                                if let Some(task_id) = url_task_ids_clone.get(url) {
                                    if let Some(ref cb) = progress_callback {
                                        cb(AgentProgress::TaskUpdate {
                                            task_id: task_id.clone(),
                                            batch_id: batch_id_clone.clone(),
                                            task_type: "WebRead".to_string(),
                                            description: url.to_string(),
                                            data_info: format!("{}%", current),
                                            status: "running".to_string(),
                                            elapsed_ms: 0,
                                            thread_id: None,
                                            progress: current,
                                            read_method: "rust+jina".to_string(),
                                            bytes_processed: 0,
                                            bytes_total: 0,
                                        });
                                    }
                                }
                            }

                            if current < 100 {
                                all_done = false;
                            }
                        }

                        if all_done {
                            break;
                        }
                    }
                });

                // Executar leituras em paralelo com progresso
                let search_client = self.search_client.clone();
                let futures: Vec<_> = web_urls
                    .iter()
                    .map(|url| {
                        let url = url.clone();
                        let progress = progress_map.get(&url).cloned().unwrap_or_else(|| Arc::new(AtomicU8::new(0)));
                        let client = search_client.clone();
                        async move {
                            let result = client.read_url_with_fallback_progress(&url, progress).await;
                            (url, result)
                        }
                    })
                    .collect();

                let results = futures::future::join_all(futures).await;

                // Parar monitor
                drop(progress_tx);
                let _ = monitor_handle.await;

                let avg_time_per_url = read_start.elapsed().as_millis() / web_urls.len().max(1) as u128;

                // Processar resultados
                for (url, (result, method, _attempts, _bytes)) in results {
                    let task_id = url_task_ids.get(&url).cloned().unwrap_or_default();

                    match result {
                        Ok(content) => {
                            let bytes_processed = content.text.len();

                            // Emitir TaskUpdate de sucesso
                            self.emit(AgentProgress::TaskUpdate {
                                task_id: task_id.clone(),
                                batch_id: batch_id.clone(),
                                task_type: "WebRead".to_string(),
                                description: url.to_string(),
                                data_info: format!("{}ms via {} | {} bytes", avg_time_per_url, method, bytes_processed),
                                status: "completed".to_string(),
                                elapsed_ms: avg_time_per_url,
                                thread_id: None,
                                progress: 100,
                                read_method: method.to_string(),
                                bytes_processed,
                                bytes_total: bytes_processed,
                            });

                            self.context.add_knowledge(KnowledgeItem {
                                question: self.context.current_question().to_string(),
                                answer: content.text,
                                item_type: KnowledgeType::Url,
                                references: vec![Reference {
                                    url: url.to_string(),
                                    title: content.title,
                                    exact_quote: None,
                                    relevance_score: None,
                                    answer_chunk: None,
                                    answer_position: None,
                                }],
                            });
                            self.context.visited_urls.push(url.clone());
                            self.emit(AgentProgress::VisitedUrl(url.to_string()));
                            success_count += 1;
                        }
                        Err(e) => {
                            // Emitir TaskUpdate de falha
                            self.emit(AgentProgress::TaskUpdate {
                                task_id: task_id.clone(),
                                batch_id: batch_id.clone(),
                                task_type: "WebRead".to_string(),
                                description: url.to_string(),
                                data_info: format!("Erro: {}", e),
                                status: "failed".to_string(),
                                elapsed_ms: avg_time_per_url,
                                thread_id: None,
                                progress: 100,
                                read_method: method.to_string(),
                                bytes_processed: 0,
                                bytes_total: 0,
                            });

                            log::warn!("❌ Falha ao ler URL {}: {}", url, e);
                            self.context.bad_urls.push(url.clone());
                            error_count += 1;
                        }
                    }
                }
            }
        }

        let read_time = read_timer.stop();
        self.timing_stats.add_read_time(read_time);

        // Emitir fim do batch
        self.emit(AgentProgress::BatchEnd {
            batch_id: batch_id.clone(),
            total_ms: read_time,
            success_count,
            fail_count: error_count,
        });

        // Incrementar contador de leituras
        self.read_count += 1;

        // Calcular tempo médio por URL (demonstra eficiência do paralelismo)
        let avg_time_per_url = if num_urls > 0 {
            read_time as f64 / num_urls as f64
        } else {
            0.0
        };

        log::info!(
            "📖 Batch {} concluído: {} URLs em {}ms (média: {:.1}ms/URL) | ✅ {} ok | ❌ {} erros",
            &batch_id[..8],
            num_urls,
            read_time,
            avg_time_per_url,
            success_count,
            error_count
        );

        // Emitir log detalhado sobre paralelismo
        self.emit(AgentProgress::Success(format!(
            "⚡ Batch {} concluído: {} URLs em {:.2}s ({:.0}ms/URL) | ✅{} ❌{}",
            &batch_id[..8],
            num_urls,
            read_time as f64 / 1000.0,
            avg_time_per_url,
            success_count,
            error_count
        )));
        self.emit_persona_stats(true);

        self.context.diary.push(DiaryEntry::Read {
            urls: urls_to_read.clone(),
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

            // Validar referências mesmo para respostas triviais
            let validated_refs = self.validate_references(references).await;

            return StepResult::Completed(AnswerResult {
                answer,
                references: validated_refs,
                trivial: true,
            });
        }

        // Obter tipos de avaliação necessários
        let pipeline = EvaluationPipeline::new(self.llm_client.clone());
        let eval_types = pipeline
            .determine_required_evaluations(&self.context.original_question, &*self.llm_client)
            .await;

        // 🔍 Emitir início de validação fast-fail
        let eval_type_names: Vec<String> = eval_types.iter().map(|t| t.as_str().to_string()).collect();
        self.emit(AgentProgress::ValidationStart {
            eval_types: eval_type_names.clone(),
        });
        self.emit(AgentProgress::Info(format!(
            "🔍 Validação Fast-Fail: {} etapas [{}]",
            eval_types.len(),
            eval_type_names.join(" → ")
        )));

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

        // 📊 Emitir resultados de cada validação
        let mut passed_count = 0;
        for eval_result in &result.results {
            let duration_ms = eval_result.duration.as_millis();

            self.emit(AgentProgress::ValidationStep {
                eval_type: eval_result.eval_type.as_str().to_string(),
                passed: eval_result.passed,
                confidence: eval_result.confidence,
                reasoning: eval_result.reasoning.chars().take(100).collect(),
                duration_ms,
            });

            if eval_result.passed {
                passed_count += 1;
                self.emit(AgentProgress::Success(format!(
                    "✅ {} passou ({:.0}% conf, {}ms)",
                    eval_result.eval_type.as_str(),
                    eval_result.confidence * 100.0,
                    duration_ms
                )));
            } else {
                self.emit(AgentProgress::Warning(format!(
                    "❌ {} FALHOU: {} ({:.0}% conf)",
                    eval_result.eval_type.as_str(),
                    eval_result.reasoning.chars().take(50).collect::<String>(),
                    eval_result.confidence * 100.0
                )));
            }
        }

        // 📊 Emitir fim de validação
        self.emit(AgentProgress::ValidationEnd {
            overall_passed: result.overall_passed,
            failed_at: result.failed_at.map(|t| t.as_str().to_string()),
            total_evals: result.results.len(),
            passed_evals: passed_count,
        });

        if result.overall_passed {
            // Incrementar contador de respostas
            self.answer_count += 1;
            // Resetar contador de falhas consecutivas
            self.consecutive_failures = 0;

            log::info!("✅ Resposta aprovada na avaliação!");

            // 🔗 Validar referências antes de finalizar
            let validated_refs = self.validate_references(references).await;

            log::info!(
                "🔗 {} referências validadas para resposta final",
                validated_refs.len()
            );

            // Emitir sucesso e atualizar persona (não ativa pois vai finalizar)
            self.emit(AgentProgress::Success(format!(
                "✅ Resposta #{} aprovada! ({} refs validadas)",
                self.answer_count,
                validated_refs.len()
            )));
            self.emit_persona_stats(false);

            StepResult::Completed(AnswerResult {
                answer,
                references: validated_refs,
                trivial: false,
            })
        } else {
            log::info!("❌ Resposta reprovada, continuando pesquisa...");

            // Incrementar contador de falhas consecutivas
            self.consecutive_failures += 1;

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
                answer: answer.clone(),
                eval_type: failed_type,
                reason: reasoning.clone(),
            });

            // 🔍 Disparar AgentAnalyzer após 2+ falhas consecutivas (máximo 3 análises por sessão)
            const MAX_ANALYSES_PER_SESSION: usize = 3;
            if self.consecutive_failures >= 2 && self.analysis_count < MAX_ANALYSES_PER_SESSION {
                self.analysis_count += 1;

                log::info!(
                    "🔬 AgentAnalyzer: Disparando análise #{} após {} falhas consecutivas",
                    self.analysis_count,
                    self.consecutive_failures
                );

                // Emitir evento de início
                self.emit(AgentProgress::AgentAnalysisStarted {
                    failures_count: self.consecutive_failures,
                    diary_entries: self.context.diary.len(),
                });

                // Preparar dados para análise em background
                let diary_clone = self.context.diary.clone();
                let original_question = self.context.original_question.clone();
                let failed_answer = answer.clone();
                let failure_reason = reasoning.clone();
                let llm_client = self.llm_client.clone();

                // Criar canal para receber resultado
                let (tx, rx) = mpsc::channel(1);
                self.analysis_rx = Some(rx);

                // Disparar análise em background (não bloqueia)
                tokio::spawn(async move {
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        agent_analyzer::analyze_steps(
                            &diary_clone,
                            &original_question,
                            &failed_answer,
                            &failure_reason,
                            llm_client,
                        ),
                    )
                    .await;

                    match result {
                        Ok(Ok(analysis)) => {
                            let _ = tx.send(analysis).await;
                        }
                        Ok(Err(e)) => {
                            log::warn!("🔬 AgentAnalyzer falhou: {}", e);
                        }
                        Err(_) => {
                            log::warn!("🔬 AgentAnalyzer timeout (30s)");
                        }
                    }
                });
            }

            self.context.total_step += 1;
            StepResult::Continue
        }
    }

    /// Executa código em sandbox seguro usando Boa Engine
    ///
    /// O sandbox permite ao agente:
    /// 1. Gerar código JavaScript via LLM para processar dados
    /// 2. Executar em ambiente isolado (sem acesso a filesystem/rede)
    /// 3. Retry inteligente se o código falhar (até 3 tentativas)
    async fn execute_coding(
        &mut self,
        problem: String,
        language: Option<String>,
        think: String,
    ) -> StepResult {
        log::info!("🖥️ Executando código em sandbox...");
        log::debug!("📝 Problema: {}", problem);
        log::debug!("🌐 Linguagem preferida: {:?}", language);
        log::debug!("💭 Raciocínio: {}", think);

        self.emit(AgentProgress::Action("coding".into()));
        self.emit(AgentProgress::Think(think.clone()));

        // Truncar problema para preview
        let problem_preview = if problem.len() > 100 {
            format!("{}...", &problem[..100])
        } else {
            problem.clone()
        };

        // Determinar linguagem preferida
        let preferred_language = match language.as_deref() {
            Some("javascript") | Some("js") => SandboxLanguage::JavaScript,
            Some("python") | Some("py") => SandboxLanguage::Python,
            _ => SandboxLanguage::Auto, // LLM escolhe
        };

        self.emit(AgentProgress::Info(format!(
            "🖥️ Gerando e executando código ({}) para: {}",
            preferred_language.display_name(),
            problem_preview
        )));

        // Emitir evento de início do sandbox
        let max_attempts = 3usize;
        let timeout_ms = 10000u64; // Maior timeout para Python
        self.emit(AgentProgress::SandboxStart {
            problem: problem_preview.clone(),
            max_attempts,
            timeout_ms,
            language: preferred_language.display_name().to_string(),
        });

        // Emitir evento de tentativa inicial
        self.emit(AgentProgress::SandboxAttempt {
            attempt: 1,
            max_attempts,
            code_preview: "Gerando código via LLM...".to_string(),
            status: "generating".to_string(),
            error: None,
        });

        // Criar sandbox unificado (suporta JS e Python)
        let sandbox = UnifiedSandbox::new(&self.context.knowledge, timeout_ms)
            .await
            .with_language(preferred_language);

        // Resolver o problema gerando e executando código
        match sandbox.solve(&*self.llm_client, &problem).await {
            Ok(result) => {
                // Truncar código para preview
                let code_preview = if result.code.len() > 200 {
                    format!("{}...", &result.code[..200])
                } else {
                    result.code.clone()
                };

                if result.success {
                    let output = result.output.clone().unwrap_or_default();
                    log::info!(
                        "✅ Código executado com sucesso em {} tentativa(s), {}ms",
                        result.attempts,
                        result.execution_time_ms
                    );
                    log::debug!("📤 Output: {}", output);

                    // Emitir evento de conclusão bem-sucedida
                    self.emit(AgentProgress::SandboxComplete {
                        success: true,
                        output: Some(if output.len() > 500 {
                            format!("{}...", &output[..500])
                        } else {
                            output.clone()
                        }),
                        error: None,
                        attempts: result.attempts,
                        execution_time_ms: result.execution_time_ms,
                        code_preview: code_preview.clone(),
                        language: result.language.display_name().to_string(),
                    });

                    self.emit(AgentProgress::Success(format!(
                        "✅ Código executado com sucesso ({} tentativas, {}ms)",
                        result.attempts, result.execution_time_ms
                    )));

                    // Adicionar resultado ao knowledge
                    self.context.knowledge.push(KnowledgeItem {
                        question: format!("[Código] {}", problem),
                        answer: output,
                        item_type: KnowledgeType::Coding,
                        references: vec![],
                    });
                } else {
                    let error = result.error.clone().unwrap_or_else(|| "Unknown error".into());
                    log::warn!(
                        "❌ Código falhou após {} tentativa(s): {}",
                        result.attempts,
                        error
                    );

                    // Emitir evento de conclusão com falha
                    self.emit(AgentProgress::SandboxComplete {
                        success: false,
                        output: None,
                        error: Some(error.clone()),
                        attempts: result.attempts,
                        execution_time_ms: result.execution_time_ms,
                        code_preview: code_preview.clone(),
                        language: result.language.display_name().to_string(),
                    });

                    self.emit(AgentProgress::Warning(format!(
                        "❌ Código {} falhou após {} tentativas: {}",
                        result.language.display_name(),
                        result.attempts, error
                    )));

                    // Adicionar erro ao knowledge para contexto futuro
                    let lang_ext = result.language.extension();
                    self.context.knowledge.push(KnowledgeItem {
                        question: format!("[Código {} Falhou] {}", result.language.display_name(), problem),
                        answer: format!("Erro: {}. Código tentado:\n```{}\n{}\n```", error, lang_ext, result.code),
                        item_type: KnowledgeType::Error,
                        references: vec![],
                    });
                }

                // Registrar no diário
                self.context.diary.push(DiaryEntry::Coding {
                    code: result.code,
                    language: result.language.display_name().to_string(),
                    think,
                });
            }
            Err(e) => {
                log::error!("💥 Sandbox error: {}", e);

                // Emitir evento de conclusão com erro fatal
                self.emit(AgentProgress::SandboxComplete {
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    attempts: 0,
                    execution_time_ms: 0,
                    code_preview: "// Erro fatal antes da execução".to_string(),
                    language: preferred_language.display_name().to_string(),
                });

                self.emit(AgentProgress::Error(format!("💥 Sandbox error: {}", e)));

                // Registrar erro no diário
                self.context.diary.push(DiaryEntry::Coding {
                    code: format!("// Error: {}", e),
                    language: preferred_language.display_name().to_string(),
                    think,
                });
            }
        }

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Executa consulta ao histórico de sessões anteriores
    async fn execute_history(
        &mut self,
        count: usize,
        filter: Option<String>,
        think: String,
    ) -> StepResult {
        self.emit(AgentProgress::Info(format!(
            "📜 Consultando histórico ({} sessões)...",
            count
        )));
        log::info!("📜 Consultando histórico de sessões anteriores");

        // Criar serviço de histórico (usa local por padrão)
        let history_service = HistoryService::default();

        // Construir query
        let query = if let Some(ref text) = filter {
            HistoryQuery::new(count).with_text_filter(text)
        } else {
            HistoryQuery::new(count)
        };

        // Buscar sessões
        match history_service.search(&query).await {
            Ok(result) => {
                let sessions_loaded = result.sessions.len();

                if sessions_loaded > 0 {
                    // Formatar contexto para adicionar ao knowledge
                    let context = result.format_for_llm();

                    self.context.knowledge.push(KnowledgeItem {
                        question: "Histórico de pesquisas anteriores".to_string(),
                        answer: context,
                        item_type: KnowledgeType::History,
                        references: vec![],
                    });

                    self.emit(AgentProgress::Success(format!(
                        "✅ {} sessões anteriores carregadas (backend: {})",
                        sessions_loaded, result.backend
                    )));
                    log::info!(
                        "📜 {} sessões carregadas em {}ms",
                        sessions_loaded,
                        result.search_time_ms
                    );
                } else {
                    self.emit(AgentProgress::Warning(
                        "⚠️ Nenhuma sessão anterior encontrada".to_string(),
                    ));
                    log::info!("📜 Nenhuma sessão anterior encontrada");
                }

                self.context.diary.push(DiaryEntry::History {
                    sessions_loaded,
                    think,
                });
            }
            Err(e) => {
                self.emit(AgentProgress::Warning(format!(
                    "⚠️ Erro ao consultar histórico: {}",
                    e
                )));
                log::warn!("📜 Erro ao consultar histórico: {}", e);

                self.context.diary.push(DiaryEntry::History {
                    sessions_loaded: 0,
                    think,
                });
            }
        }

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Executa pergunta ao usuário
    ///
    /// Compatível com OpenAI Responses API (input_required state).
    /// Se `is_blocking` for true, retorna StepResult::InputRequired
    /// e o agente pausa até receber resposta.
    async fn execute_ask_user(
        &mut self,
        question_type: QuestionType,
        question: String,
        options: Option<Vec<String>>,
        is_blocking: bool,
        think: String,
    ) -> StepResult {
        log::info!("❓ Executando pergunta ao usuário");
        log::debug!("   Tipo: {:?}", question_type);
        log::debug!("   Blocking: {}", is_blocking);
        log::debug!("   Opções: {:?}", options);

        // Criar pergunta pendente
        let pending_question = PendingQuestion {
            id: uuid::Uuid::new_v4().to_string(),
            question_type,
            question: question.clone(),
            options: options.clone(),
            is_blocking,
            context: None,
            created_at: chrono::Utc::now(),
            think: think.clone(),
        };

        let question_id = pending_question.id.clone();

        // Emitir evento para interface (TUI/Chatbot)
        self.emit(AgentProgress::AgentQuestion {
            question_id: question_id.clone(),
            question_type: question_type.as_str().to_string(),
            question: question.clone(),
            options: options.clone(),
            is_blocking,
        });

        // Registrar no diário
        self.context.diary.push(DiaryEntry::UserQuestion {
            question_id: question_id.clone(),
            question_type,
            question: question.clone(),
            was_blocking: is_blocking,
            think,
        });

        // Adicionar ao hub de interação
        if let Err(e) = self.interaction_hub.ask(pending_question).await {
            log::warn!("❌ Erro ao enviar pergunta: {}", e);
            self.emit(AgentProgress::Warning(format!(
                "❌ Erro ao enviar pergunta: {}",
                e
            )));
        }

        if is_blocking {
            // Retornar InputRequired para pausar o agente
            log::info!("⏸️ Agente pausado aguardando resposta do usuário");

            // Mudar estado para InputRequired
            self.state = AgentState::InputRequired {
                question_id: question_id.clone(),
                question: question.clone(),
                question_type,
                options,
            };

            StepResult::InputRequired {
                question_id,
                question,
                question_type,
                options: self.interaction_hub.get_blocking_question()
                    .and_then(|q| q.options.clone()),
            }
        } else {
            // Pergunta não blocking - continuar execução
            log::info!("▶️ Pergunta enviada, continuando execução");
            self.context.total_step += 1;
            StepResult::Continue
        }
    }

    /// Processa resposta do usuário recebida
    ///
    /// Chamado quando o usuário responde a uma pergunta ou envia
    /// mensagem espontânea. Adiciona ao knowledge e retoma execução.
    pub async fn process_user_response(&mut self, response: UserResponse) -> StepResult {
        log::info!("📥 Processando resposta do usuário");
        log::debug!("   Question ID: {:?}", response.question_id);
        log::debug!("   Conteúdo: {}", response.content);

        let was_spontaneous = response.question_id.is_none();

        // Emitir evento
        self.emit(AgentProgress::UserResponseReceived {
            question_id: response.question_id.clone(),
            response: response.content.clone(),
            was_spontaneous,
        });

        // Registrar no diário
        self.context.diary.push(DiaryEntry::UserResponse {
            question_id: response.question_id.clone(),
            response: response.content.clone(),
            was_spontaneous,
        });

        // Adicionar ao knowledge
        let knowledge_question = if let Some(ref qid) = response.question_id {
            format!("[Resposta do usuário para {}]", qid)
        } else {
            "[Mensagem do usuário]".to_string()
        };

        self.context.knowledge.push(KnowledgeItem {
            question: knowledge_question,
            answer: response.content.clone(),
            item_type: KnowledgeType::UserProvided,
            references: vec![],
        });

        // Marcar pergunta como respondida
        if let Some(ref qid) = response.question_id {
            self.interaction_hub.mark_answered(qid);

            // Emitir evento de retomada
            self.emit(AgentProgress::ResumedAfterInput {
                question_id: qid.clone(),
            });
        }

        // Se estava em InputRequired, voltar para Processing
        if self.state.is_input_required() {
            log::info!("▶️ Retomando execução após resposta do usuário");
            self.state = AgentState::Processing {
                step: 0,
                total_step: self.context.total_step as u32,
                current_question: self.context.current_question().to_string(),
                budget_used: self.token_tracker.budget_used_percentage(),
            };
        }

        self.context.total_step += 1;
        StepResult::Continue
    }

    /// Verifica e processa mensagens pendentes do usuário
    ///
    /// Deve ser chamado no início de cada step para processar
    /// mensagens que chegaram de forma assíncrona.
    pub fn poll_user_messages(&mut self) {
        self.interaction_hub.poll_responses();

        // Processar respostas espontâneas (adicionar ao knowledge)
        while let Some(response) = self.interaction_hub.next_response() {
            log::debug!("📥 Mensagem assíncrona recebida: {}", response.content);

            // Registrar no diário
            self.context.diary.push(DiaryEntry::UserResponse {
                question_id: response.question_id.clone(),
                response: response.content.clone(),
                was_spontaneous: response.question_id.is_none(),
            });

            // Adicionar ao knowledge
            self.context.knowledge.push(KnowledgeItem {
                question: "[Mensagem do usuário]".to_string(),
                answer: response.content,
                item_type: KnowledgeType::UserProvided,
                references: vec![],
            });
        }
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

    /// Deduplica queries usando embeddings SIMD
    /// Retorna (queries únicas, contagem de removidas, embeddings das queries únicas)
    async fn dedup_queries_with_embeddings(
        &self,
        queries: Vec<SerpQuery>,
    ) -> (Vec<SerpQuery>, usize, Vec<Vec<f32>>) {
        use crate::performance::simd::dedup_queries as simd_dedup;

        let original_count = queries.len();

        if queries.is_empty() {
            return (queries, 0, vec![]);
        }

        // Gerar embeddings para todas as novas queries
        let query_texts: Vec<String> = queries.iter().map(|q| q.q.clone()).collect();

        self.emit(AgentProgress::Info(format!(
            "🧠 Gerando embeddings para {} queries (SIMD)...",
            query_texts.len()
        )));

        let embeddings_result = self.llm_client.embed_batch(&query_texts).await;

        let new_embeddings: Vec<Vec<f32>> = match embeddings_result {
            Ok(results) => results.into_iter().map(|r| r.vector).collect(),
            Err(e) => {
                // Fallback para dedup simples se embeddings falharem
                self.emit(AgentProgress::Warning(format!(
                    "⚠️ Embeddings falhou, usando dedup textual: {}",
                    e
                )));
                return self.dedup_queries_text_fallback(queries).await;
            }
        };

        // Usar SIMD para deduplicação
        let existing_embeddings = &self.context.executed_query_embeddings;

        self.emit(AgentProgress::Info(format!(
            "⚡ Dedup SIMD: {} novas vs {} existentes (threshold: 0.86)",
            new_embeddings.len(),
            existing_embeddings.len()
        )));

        let unique_indices = simd_dedup(&new_embeddings, existing_embeddings, 0.86);

        // Filtrar queries e embeddings pelos índices únicos
        let mut unique_queries = Vec::new();
        let mut unique_embeddings = Vec::new();

        for idx in &unique_indices {
            unique_queries.push(queries[*idx].clone());
            unique_embeddings.push(new_embeddings[*idx].clone());
        }

        let removed_count = original_count - unique_queries.len();

        if removed_count > 0 {
            self.emit(AgentProgress::Success(format!(
                "🎯 SIMD Dedup: {} duplicadas removidas semanticamente",
                removed_count
            )));
        }

        (unique_queries, removed_count, unique_embeddings)
    }

    /// Fallback para dedup textual quando embeddings falham
    async fn dedup_queries_text_fallback(&self, queries: Vec<SerpQuery>) -> (Vec<SerpQuery>, usize, Vec<Vec<f32>>) {
        use std::collections::HashSet;

        let original_count = queries.len();
        let mut unique = Vec::new();
        let mut seen_normalized = HashSet::new();

        for query in queries {
            let normalized = query.q
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            let is_duplicate = seen_normalized.iter().any(|seen: &String| {
                let seen_words: HashSet<_> = seen.split_whitespace().collect();
                let new_words: HashSet<_> = normalized.split_whitespace().collect();

                if seen_words.is_empty() || new_words.is_empty() {
                    return false;
                }

                let intersection = seen_words.intersection(&new_words).count();
                let union = seen_words.union(&new_words).count();

                intersection as f32 / union as f32 >= 0.86
            });

            if !is_duplicate && !normalized.is_empty() {
                seen_normalized.insert(normalized);
                unique.push(query);
            }
        }

        let removed_count = original_count - unique.len();
        // Retorna embeddings vazios pois fallback não gera embeddings
        (unique, removed_count, vec![])
    }

    /// Deduplica queries usando SIMD (interface simplificada)
    /// Retorna (queries únicas, contagem de removidas)
    async fn dedup_queries_with_stats(&self, queries: Vec<SerpQuery>) -> (Vec<SerpQuery>, usize) {
        let (unique, removed, _embeddings) = self.dedup_queries_with_embeddings(queries).await;
        (unique, removed)
    }

    #[allow(dead_code)]
    async fn dedup_queries(&self, queries: Vec<SerpQuery>) -> Vec<SerpQuery> {
        let (unique, _) = self.dedup_queries_with_stats(queries).await;
        unique
    }

    async fn dedup_questions(&self, questions: Vec<String>) -> Vec<String> {
        use std::collections::HashSet;

        let mut unique = Vec::new();
        let mut seen_normalized = HashSet::new();

        for q in questions {
            let normalized = q
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            if !seen_normalized.contains(&normalized) && !normalized.is_empty() {
                seen_normalized.insert(normalized);
                unique.push(q);
            }
        }

        unique
    }

    fn build_query_context(&self) -> crate::personas::QueryContext {
        crate::personas::QueryContext {
            execution_id: uuid::Uuid::new_v4(),
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

    /// Constrói referências semânticas usando embeddings e cosine similarity
    ///
    /// Este método usa o `ReferenceBuilder` para:
    /// 1. Fazer chunking da resposta e do conteúdo web
    /// 2. Gerar embeddings para todos os chunks
    /// 3. Calcular similaridade cosseno (SIMD otimizado)
    /// 4. Inserir marcadores [^1], [^2] na resposta
    ///
    /// Faz fallback para `extract_references_from_knowledge` se falhar.
    async fn build_semantic_references(
        &self,
        answer: &str,
        llm_references: Vec<Reference>,
    ) -> (String, Vec<Reference>) {
        // Se o LLM já retornou referências e a resposta é curta, usar as do LLM
        if !llm_references.is_empty() && answer.len() < 500 {
            log::info!(
                "📚 Usando {} referências do LLM (resposta curta)",
                llm_references.len()
            );
            return (answer.to_string(), llm_references);
        }

        // Verificar se temos conteúdo web no knowledge
        let has_web_content = self
            .context
            .knowledge
            .iter()
            .any(|k| k.item_type == KnowledgeType::Url && !k.answer.is_empty());

        if !has_web_content {
            log::info!("📚 Sem conteúdo web, usando referências do LLM ou extraídas");
            let refs = if llm_references.is_empty() {
                self.extract_references_from_knowledge()
            } else {
                llm_references
            };
            return (answer.to_string(), refs);
        }

        // Tentar construir referências semânticas
        self.emit(AgentProgress::Info(
            "🔗 Sistema de Referências Semânticas iniciado...".into(),
        ));

        // Contar conhecimento disponível para matching
        let web_sources: Vec<_> = self
            .context
            .knowledge
            .iter()
            .filter(|k| k.item_type == KnowledgeType::Url && !k.answer.is_empty())
            .collect();

        self.emit(AgentProgress::Info(format!(
            "📊 Analisando {} fontes web ({} chars de resposta)",
            web_sources.len(),
            answer.len()
        )));

        let config = ReferenceBuilderConfig::new(
            80,   // min_chunk_length
            10,   // max_references
            0.65, // min_relevance_score (um pouco mais permissivo)
        );

        let builder = ReferenceBuilder::new(self.llm_client.clone(), config);

        self.emit(AgentProgress::Info(
            "🧠 Gerando embeddings e calculando similaridade coseno (SIMD)...".into(),
        ));

        match builder
            .build_references(answer, &self.context.knowledge)
            .await
        {
            Ok(result) => {
                if result.references.is_empty() {
                    self.emit(AgentProgress::Warning(
                        "⚠️ Nenhum match semântico encontrado (threshold: 0.65)".into(),
                    ));
                    log::warn!("📚 Nenhuma referência semântica encontrada, usando fallback");
                    let refs = if llm_references.is_empty() {
                        self.extract_references_from_knowledge()
                    } else {
                        llm_references
                    };
                    (answer.to_string(), refs)
                } else {
                    // Emitir detalhes de cada referência encontrada
                    for (i, r) in result.references.iter().enumerate() {
                        let score = r.relevance_score.unwrap_or(0.0);
                        let quote_preview = r.exact_quote.as_ref()
                            .map(|q| q.chars().take(50).collect::<String>())
                            .unwrap_or_else(|| "...".to_string());
                        self.emit(AgentProgress::Info(format!(
                            "   [^{}] {:.0}% - \"{}...\"",
                            i + 1,
                            score * 100.0,
                            quote_preview
                        )));
                    }

                    self.emit(AgentProgress::Success(format!(
                        "✅ {} referências semânticas inseridas na resposta",
                        result.references.len()
                    )));
                    log::info!(
                        "📚 Construídas {} referências semânticas",
                        result.references.len()
                    );
                    (result.answer, result.references)
                }
            }
            Err(e) => {
                log::error!("📚 Erro ao construir referências semânticas: {:?}", e);
                self.emit(AgentProgress::Warning(format!(
                    "⚠️ Fallback Jaccard: {}",
                    e
                )));
                let refs = if llm_references.is_empty() {
                    self.extract_references_from_knowledge()
                } else {
                    llm_references
                };
                (answer.to_string(), refs)
            }
        }
    }

    /// Extrai referências do conhecimento coletado (fallback legado)
    fn extract_references_from_knowledge(&self) -> Vec<Reference> {
        use std::collections::HashSet;

        let mut seen_urls = HashSet::new();
        let mut refs = Vec::new();

        // Extrair referências do conhecimento (KnowledgeItem)
        for item in &self.context.knowledge {
            if item.item_type == KnowledgeType::Url {
                for reference in &item.references {
                    if !reference.url.is_empty() && !seen_urls.contains(&reference.url) {
                        seen_urls.insert(reference.url.clone());
                        refs.push(reference.clone());
                    }
                }
            }
        }

        // Se ainda não temos referências, usar URLs visitadas
        if refs.is_empty() {
            for url in &self.context.visited_urls {
                let url_str = url.to_string();
                if !seen_urls.contains(&url_str) {
                    seen_urls.insert(url_str.clone());
                    refs.push(Reference {
                        url: url_str,
                        title: "Fonte visitada".to_string(),
                        exact_quote: None,
                        relevance_score: None,
                        answer_chunk: None,
                        answer_position: None,
                    });
                }
            }
        }

        log::info!("📚 Extraídas {} referências do conhecimento", refs.len());
        refs
    }

    /// Valida referências antes de incluir na resposta final
    ///
    /// Remove referências com:
    /// - Títulos inválidos (Cloudflare blocks, Page not found, vazios)
    /// - URLs que retornam 4xx/5xx (verificação HEAD)
    async fn validate_references(&self, references: Vec<Reference>) -> Vec<Reference> {
        use futures::future::join_all;

        let original_count = references.len();

        if references.is_empty() {
            return references;
        }

        self.emit(AgentProgress::Info(format!(
            "🔗 Validando {} referências...",
            original_count
        )));

        // Títulos que indicam problemas
        const INVALID_TITLE_PATTERNS: &[&str] = &[
            "just a moment",
            "page not found",
            "404",
            "403",
            "access denied",
            "cloudflare",
            "checking your browser",
            "please wait",
            "error",
            "not available",
            "blocked",
            "captcha",
            "verify you are human",
            "security check",
        ];

        // Filtrar por título primeiro (rápido)
        let title_filtered: Vec<Reference> = references
            .into_iter()
            .filter(|r| {
                // Título vazio ou muito curto
                let title_lower = r.title.to_lowercase().trim().to_string();
                if title_lower.is_empty() || title_lower.len() < 3 {
                    log::warn!("🔗 Removendo ref com título vazio/curto: {}", r.url);
                    return false;
                }

                // Padrões de título inválido
                for pattern in INVALID_TITLE_PATTERNS {
                    if title_lower.contains(pattern) {
                        log::warn!(
                            "🔗 Removendo ref com título inválido '{}': {}",
                            r.title,
                            r.url
                        );
                        return false;
                    }
                }

                true
            })
            .collect();

        if title_filtered.is_empty() {
            self.emit(AgentProgress::Warning(
                "⚠️ Todas as referências tinham títulos inválidos".into(),
            ));
            return vec![];
        }

        // Validar URLs com HEAD request (em paralelo)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        let validation_futures: Vec<_> = title_filtered
            .iter()
            .map(|r| {
                let client = client.clone();
                let url = r.url.clone();
                async move {
                    match client.head(&url).send().await {
                        Ok(response) => {
                            let status = response.status();
                            if status.is_success() || status.is_redirection() {
                                log::debug!("✅ URL válida ({}): {}", status, url);
                                true
                            } else {
                                log::warn!("❌ URL inválida ({}): {}", status, url);
                                false
                            }
                        }
                        Err(e) => {
                            // Tentar GET se HEAD falhar (alguns servidores não suportam HEAD)
                            match client.get(&url).send().await {
                                Ok(response) => {
                                    let status = response.status();
                                    if status.is_success() || status.is_redirection() {
                                        log::debug!("✅ URL válida via GET ({}): {}", status, url);
                                        true
                                    } else {
                                        log::warn!("❌ URL inválida ({}): {}", status, url);
                                        false
                                    }
                                }
                                Err(_) => {
                                    log::warn!("❌ URL inacessível: {} ({})", url, e);
                                    false
                                }
                            }
                        }
                    }
                }
            })
            .collect();

        let validation_results = join_all(validation_futures).await;

        // Filtrar referências válidas
        let validated: Vec<Reference> = title_filtered
            .into_iter()
            .zip(validation_results.into_iter())
            .filter_map(|(r, is_valid)| if is_valid { Some(r) } else { None })
            .collect();

        let removed_count = original_count - validated.len();
        if removed_count > 0 {
            self.emit(AgentProgress::Warning(format!(
                "🔗 Removidas {} referências inválidas (restam {})",
                removed_count,
                validated.len()
            )));
        } else {
            self.emit(AgentProgress::Success(format!(
                "✅ Todas {} referências validadas",
                validated.len()
            )));
        }

        log::info!(
            "🔗 Validação: {} de {} referências aprovadas",
            validated.len(),
            original_count
        );

        validated
    }
}
