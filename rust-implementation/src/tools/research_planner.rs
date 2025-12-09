//! # Research Planner
//!
//! Este módulo implementa a divisão de problemas complexos em subproblemas
//! ortogonais para pesquisa paralela por múltiplos agentes.
//!
//! ## Baseado em
//! `src/tools/research-planner.ts` do projeto TypeScript original.
//!
//! ## Princípios de Decomposição
//!
//! ### Ortogonalidade
//! - Cada subproblema cobre um aspecto diferente
//! - Mínimo 20% de overlap entre subproblemas
//! - Remoção de qualquer subproblema deve criar gap significativo
//!
//! ### Profundidade
//! - Cada subproblema requer 15-25h de pesquisa focada
//! - Vai além de informação superficial
//! - Inclui perguntas "o que" e "por que/como"
//!
//! ### Cobertura
//! - União dos subproblemas cobre 90%+ do tópico principal
//! - Validação de completude antes de finalizar

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Datelike, Utc};

use crate::llm::LlmClient;
use crate::utils::TokenTracker;

/// Erro que pode ocorrer durante o planejamento de pesquisa.
#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    /// Erro na comunicação com o LLM.
    #[error("LLM error: {0}")]
    LlmError(String),

    /// Pergunta muito simples para decomposição.
    #[error("Question too simple for decomposition")]
    QuestionTooSimple,

    /// Tamanho de equipe inválido.
    #[error("Invalid team size: {0}")]
    InvalidTeamSize(usize),

    /// Erro de parse na resposta do LLM.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Configuração do planejador de pesquisa.
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Tamanho mínimo da equipe.
    pub min_team_size: usize,

    /// Tamanho máximo da equipe.
    pub max_team_size: usize,

    /// Temperatura para geração do LLM.
    pub temperature: f32,

    /// Comprimento mínimo da pergunta para decomposição.
    pub min_question_length: usize,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            min_team_size: 2,
            max_team_size: 10,
            temperature: 0.7,
            min_question_length: 20,
        }
    }
}

/// Plano de pesquisa com subproblemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchPlan {
    /// Raciocínio do planejador sobre a decomposição.
    pub think: String,

    /// Lista de subproblemas ortogonais.
    pub subproblems: Vec<String>,

    /// Matriz de overlap estimada entre subproblemas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlap_matrix: Option<Vec<Vec<f32>>>,

    /// Score de cobertura estimada (0.0 - 1.0).
    pub coverage_score: f32,
}

/// Planejador de pesquisa para decomposição de problemas.
///
/// Atua como um "Principal Research Lead" que divide problemas complexos
/// em subproblemas ortogonais para serem trabalhados por uma equipe
/// de pesquisadores juniores.
///
/// # Exemplo
/// ```rust,ignore
/// use deep_research::tools::ResearchPlanner;
///
/// let planner = ResearchPlanner::new(llm_client);
/// let subproblems = planner.plan_research(
///     "What is the future of AI in healthcare?",
///     3,  // team_size
///     "Some initial soundbites about the topic...",
///     &mut tracker,
/// ).await?;
/// ```
pub struct ResearchPlanner {
    /// Cliente LLM para geração de texto.
    llm_client: Arc<dyn LlmClient>,

    /// Configuração do planejador.
    config: PlannerConfig,
}

impl ResearchPlanner {
    /// Cria um novo planejador com configuração padrão.
    ///
    /// # Argumentos
    /// * `llm_client` - Cliente LLM para geração de texto.
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            llm_client,
            config: PlannerConfig::default(),
        }
    }

    /// Cria um novo planejador com configuração customizada.
    ///
    /// # Argumentos
    /// * `llm_client` - Cliente LLM para geração de texto.
    /// * `config` - Configuração do planejador.
    pub fn with_config(llm_client: Arc<dyn LlmClient>, config: PlannerConfig) -> Self {
        Self { llm_client, config }
    }

    /// Planeja a pesquisa dividindo em subproblemas ortogonais.
    ///
    /// # Argumentos
    /// * `question` - Pergunta principal a ser pesquisada.
    /// * `team_size` - Número de agentes/subproblemas desejados.
    /// * `sound_bites` - Contexto inicial sobre o tópico.
    /// * `tracker` - Tracker de tokens para monitoramento.
    ///
    /// # Retorna
    /// Vec de strings com os subproblemas.
    pub async fn plan_research(
        &self,
        question: &str,
        team_size: usize,
        sound_bites: &str,
        tracker: &mut TokenTracker,
    ) -> Result<Vec<String>, PlannerError> {
        // Validações
        if team_size < self.config.min_team_size {
            return Err(PlannerError::InvalidTeamSize(team_size));
        }
        if team_size > self.config.max_team_size {
            return Err(PlannerError::InvalidTeamSize(team_size));
        }
        if question.len() < self.config.min_question_length {
            return Err(PlannerError::QuestionTooSimple);
        }

        // Gera o plano usando LLM (com fallback para heurísticas)
        let plan = self.generate_plan(question, team_size, sound_bites).await?;

        // Valida o plano
        self.validate_plan(&plan)?;

        // Atualiza tracker de tokens (estimativa)
        let input_tokens = (question.len() + sound_bites.len()) / 4;
        let output_tokens: usize = plan.subproblems.iter().map(|s| s.len() / 4).sum();
        tracker.add_tokens("research_planner", input_tokens as u64, output_tokens as u64);

        Ok(plan.subproblems)
    }

    /// Gera o plano completo com metadados.
    ///
    /// # Argumentos
    /// * `question` - Pergunta principal.
    /// * `team_size` - Número de subproblemas.
    /// * `sound_bites` - Contexto inicial.
    ///
    /// # Retorna
    /// ResearchPlan com subproblemas e metadados.
    pub async fn plan_research_full(
        &self,
        question: &str,
        team_size: usize,
        sound_bites: &str,
        tracker: &mut TokenTracker,
    ) -> Result<ResearchPlan, PlannerError> {
        // Validações
        if team_size < self.config.min_team_size {
            return Err(PlannerError::InvalidTeamSize(team_size));
        }
        if team_size > self.config.max_team_size {
            return Err(PlannerError::InvalidTeamSize(team_size));
        }

        let plan = self.generate_plan(question, team_size, sound_bites).await?;

        // Atualiza tracker
        let input_tokens = (question.len() + sound_bites.len()) / 4;
        let output_tokens: usize = plan.subproblems.iter().map(|s| s.len() / 4).sum();
        tracker.add_tokens("research_planner", input_tokens as u64, output_tokens as u64);

        Ok(plan)
    }

    /// Gera plano usando LLM (com fallback para heurísticas se falhar).
    async fn generate_plan(
        &self,
        question: &str,
        team_size: usize,
        sound_bites: &str,
    ) -> Result<ResearchPlan, PlannerError> {
        // Tenta usar LLM primeiro
        match self.generate_plan_with_llm(question, team_size, sound_bites).await {
            Ok(plan) => {
                log::info!("✅ ResearchPlanner: Plano gerado com LLM");
                return Ok(plan);
            }
            Err(e) => {
                log::warn!("⚠️ ResearchPlanner: Falha ao usar LLM ({}), usando fallback heurístico", e);
            }
        }

        // Fallback: usa heurísticas
        self.generate_plan_heuristic(question, team_size, sound_bites)
    }

    /// Gera plano usando LLM.
    async fn generate_plan_with_llm(
        &self,
        question: &str,
        team_size: usize,
        sound_bites: &str,
    ) -> Result<ResearchPlan, PlannerError> {
        let system_prompt = self.build_system_prompt(team_size);
        let user_prompt = self.build_user_prompt(question, sound_bites);

        let prompt = crate::agent::AgentPrompt {
            system: system_prompt,
            user: user_prompt,
            diary: vec![],
        };

        let response = self
            .llm_client
            .generate_answer(&prompt, self.config.temperature)
            .await
            .map_err(|e| PlannerError::LlmError(e.to_string()))?;

        // Parse da resposta JSON
        let plan: ResearchPlan = serde_json::from_str(&response.answer)
            .map_err(|e| PlannerError::ParseError(format!("Failed to parse LLM response: {}", e)))?;

        Ok(plan)
    }

    /// Gera plano usando heurísticas (fallback sem LLM).
    fn generate_plan_heuristic(
        &self,
        question: &str,
        team_size: usize,
        sound_bites: &str,
    ) -> Result<ResearchPlan, PlannerError> {
        // Analisa a pergunta para identificar dimensões
        let dimensions = self.identify_dimensions(question, sound_bites);

        // Gera subproblemas baseados nas dimensões
        let subproblems = self.generate_subproblems(question, &dimensions, team_size);

        // Estima matriz de overlap
        let overlap_matrix = self.estimate_overlap_matrix(&subproblems);

        // Calcula score de cobertura
        let coverage_score = self.estimate_coverage(&subproblems, question);

        Ok(ResearchPlan {
            think: format!(
                "Identified {} key dimensions in the research question. \
                Decomposed into {} orthogonal subproblems with estimated {:.0}% coverage.",
                dimensions.len(),
                subproblems.len(),
                coverage_score * 100.0
            ),
            subproblems,
            overlap_matrix: Some(overlap_matrix),
            coverage_score,
        })
    }

    /// Constrói o prompt do sistema para o LLM.
    fn build_system_prompt(&self, team_size: usize) -> String {
        let current_time: DateTime<Utc> = Utc::now();
        let date = current_time.date_naive();
        let current_year = date.year();
        let current_month = date.month();

        format!(
            r#"You are a Principal Research Lead managing a team of {} junior researchers. Your role is to break down a complex research topic into focused, manageable subproblems and assign them to your team members.

User give you a research topic and some soundbites about the topic, and you follow this systematic approach:
<approach>
First, analyze the main research topic and identify:
- Core research questions that need to be answered
- Key domains/disciplines involved
- Critical dependencies between different aspects
- Potential knowledge gaps or challenges

Then decompose the topic into {} distinct, focused subproblems using these ORTHOGONALITY & DEPTH PRINCIPLES:
</approach>

<requirements>
Orthogonality Requirements:
- Each subproblem must address a fundamentally different aspect/dimension of the main topic
- Use different decomposition axes (e.g., high-level, temporal, methodological, stakeholder-based, technical layers, side-effects, etc.)
- Minimize subproblem overlap - if two subproblems share >20% of their scope, redesign them
- Apply the "substitution test": removing any single subproblem should create a significant gap in understanding

Depth Requirements:
- Each subproblem should require 15-25 hours of focused research to properly address
- Must go beyond surface-level information to explore underlying mechanisms, theories, or implications
- Should generate insights that require synthesis of multiple sources and original analysis
- Include both "what" and "why/how" questions to ensure analytical depth

Validation Checks: Before finalizing assignments, verify:
Orthogonality Matrix: Create a 2D matrix showing overlap between each pair of subproblems - aim for <20% overlap
Depth Assessment: Each subproblem should have 4-6 layers of inquiry (surface → mechanisms → implications → future directions)
Coverage Completeness: The union of all subproblems should address 90%+ of the main topic's scope
</requirements>

The current time is {}. Current year: {}, current month: {}.

Structure your response as valid JSON matching this exact schema:
{{
  "think": "Your reasoning about the decomposition",
  "subproblems": ["subproblem 1", "subproblem 2", ...],
  "overlap_matrix": [[0.0, 0.15, ...], [0.15, 0.0, ...], ...],
  "coverage_score": 0.95
}}

Do not include any text like (this subproblem is about ...) in the subproblems, use second person to describe the subproblems. Do not use the word "subproblem" or refer to other subproblems in the problem statement.
Now proceed with decomposing and assigning the research topic."#,
            team_size,
            team_size,
            current_time.to_rfc3339(),
            current_year,
            current_month
        )
    }

    /// Constrói o prompt do usuário.
    fn build_user_prompt(&self, question: &str, sound_bites: &str) -> String {
        format!(
            r#"{}

<soundbites>
{}
</soundbites>

<think>"#,
            question, sound_bites
        )
    }

    /// Identifica dimensões de análise na pergunta.
    fn identify_dimensions(&self, question: &str, sound_bites: &str) -> Vec<String> {
        let mut dimensions = Vec::new();
        let text = format!("{} {}", question, sound_bites).to_lowercase();

        // Dimensões temporais
        if text.contains("future") || text.contains("futuro") || text.contains("2024") || text.contains("2025") {
            dimensions.push("temporal_future".to_string());
        }
        if text.contains("history") || text.contains("história") || text.contains("past") {
            dimensions.push("temporal_past".to_string());
        }

        // Dimensões de stakeholder
        if text.contains("user") || text.contains("customer") || text.contains("consumer") {
            dimensions.push("stakeholder_consumer".to_string());
        }
        if text.contains("business") || text.contains("company") || text.contains("enterprise") {
            dimensions.push("stakeholder_business".to_string());
        }
        if text.contains("government") || text.contains("regulation") || text.contains("policy") {
            dimensions.push("stakeholder_government".to_string());
        }

        // Dimensões técnicas
        if text.contains("technical") || text.contains("technology") || text.contains("implementation") {
            dimensions.push("technical_implementation".to_string());
        }
        if text.contains("security") || text.contains("privacy") || text.contains("risk") {
            dimensions.push("technical_security".to_string());
        }

        // Dimensões de impacto
        if text.contains("impact") || text.contains("effect") || text.contains("consequence") {
            dimensions.push("impact_analysis".to_string());
        }
        if text.contains("benefit") || text.contains("advantage") || text.contains("opportunity") {
            dimensions.push("impact_positive".to_string());
        }
        if text.contains("challenge") || text.contains("problem") || text.contains("limitation") {
            dimensions.push("impact_challenges".to_string());
        }

        // Dimensão econômica
        if text.contains("cost") || text.contains("price") || text.contains("economic") || text.contains("market") {
            dimensions.push("economic_analysis".to_string());
        }

        // Se não identificou dimensões suficientes, usa padrões
        if dimensions.len() < 2 {
            dimensions = vec![
                "overview_definition".to_string(),
                "current_state".to_string(),
                "key_players".to_string(),
                "challenges_opportunities".to_string(),
                "future_outlook".to_string(),
            ];
        }

        dimensions
    }

    /// Gera subproblemas baseados nas dimensões identificadas.
    fn generate_subproblems(
        &self,
        question: &str,
        dimensions: &[String],
        team_size: usize,
    ) -> Vec<String> {
        let base_topic = self.extract_topic(question);

        // Seleciona dimensões para o tamanho da equipe
        let selected_dimensions: Vec<&String> = dimensions
            .iter()
            .take(team_size)
            .collect();

        // Se não há dimensões suficientes, cria subproblemas genéricos
        if selected_dimensions.len() < team_size {
            return self.generate_generic_subproblems(&base_topic, team_size);
        }

        // Gera subproblemas baseados nas dimensões
        selected_dimensions
            .iter()
            .map(|dim| self.dimension_to_subproblem(&base_topic, dim))
            .collect()
    }

    /// Extrai o tópico principal da pergunta.
    fn extract_topic(&self, question: &str) -> String {
        // Remove palavras de pergunta comuns
        let cleaned = question
            .to_lowercase()
            .replace("what is", "")
            .replace("what are", "")
            .replace("how does", "")
            .replace("how do", "")
            .replace("why is", "")
            .replace("why are", "")
            .replace("can you explain", "")
            .replace("tell me about", "")
            .replace("?", "")
            .trim()
            .to_string();

        if cleaned.len() > 10 {
            cleaned
        } else {
            question.to_string()
        }
    }

    /// Converte uma dimensão em um subproblema.
    fn dimension_to_subproblem(&self, topic: &str, dimension: &str) -> String {
        match dimension.as_ref() {
            "temporal_future" => format!(
                "Investigate the future trajectory and emerging trends of {}. \
                What developments are expected in the next 3-5 years? \
                What factors will drive these changes?",
                topic
            ),
            "temporal_past" => format!(
                "Research the historical evolution of {}. \
                How did it develop over time? \
                What were the key milestones and turning points?",
                topic
            ),
            "stakeholder_consumer" => format!(
                "Analyze how {} impacts end users and consumers. \
                What are their experiences, needs, and pain points? \
                How can user experience be improved?",
                topic
            ),
            "stakeholder_business" => format!(
                "Examine the business implications of {}. \
                What are the opportunities and challenges for organizations? \
                What business models are emerging?",
                topic
            ),
            "stakeholder_government" => format!(
                "Investigate the regulatory landscape surrounding {}. \
                What policies and regulations exist? \
                How might future regulation evolve?",
                topic
            ),
            "technical_implementation" => format!(
                "Deep dive into the technical architecture and implementation of {}. \
                What are the core technologies involved? \
                What are the best practices and common patterns?",
                topic
            ),
            "technical_security" => format!(
                "Analyze the security and privacy aspects of {}. \
                What are the risks and vulnerabilities? \
                What mitigation strategies exist?",
                topic
            ),
            "impact_analysis" => format!(
                "Evaluate the broader impact and consequences of {}. \
                What are the second-order effects? \
                How does it affect different sectors?",
                topic
            ),
            "impact_positive" => format!(
                "Identify the benefits and opportunities presented by {}. \
                What positive outcomes have been observed? \
                What potential remains untapped?",
                topic
            ),
            "impact_challenges" => format!(
                "Examine the challenges and limitations of {}. \
                What obstacles exist? \
                How are they being addressed?",
                topic
            ),
            "economic_analysis" => format!(
                "Analyze the economic aspects of {}. \
                What are the costs and revenue potential? \
                What market dynamics are at play?",
                topic
            ),
            "overview_definition" => format!(
                "Provide a comprehensive overview and definition of {}. \
                What are its core components? \
                How is it commonly understood?",
                topic
            ),
            "current_state" => format!(
                "Research the current state of {}. \
                What is the present landscape? \
                Who are the key players?",
                topic
            ),
            "key_players" => format!(
                "Identify and analyze the key players in {}. \
                Who are the leaders? \
                What differentiates them?",
                topic
            ),
            "challenges_opportunities" => format!(
                "Explore both the challenges and opportunities in {}. \
                What barriers exist? \
                What potential is yet to be realized?",
                topic
            ),
            "future_outlook" => format!(
                "Project the future outlook for {}. \
                What trends are emerging? \
                What scenarios are possible?",
                topic
            ),
            _ => format!(
                "Investigate the {} dimension of {}. \
                What key insights can be uncovered? \
                What implications do they have?",
                dimension, topic
            ),
        }
    }

    /// Gera subproblemas genéricos quando dimensões específicas não são identificadas.
    fn generate_generic_subproblems(&self, topic: &str, team_size: usize) -> Vec<String> {
        let templates = vec![
            format!("What is the current state and landscape of {}?", topic),
            format!("Who are the key players and stakeholders in {}?", topic),
            format!("What are the main challenges and limitations of {}?", topic),
            format!("What opportunities and benefits does {} present?", topic),
            format!("What is the future outlook and emerging trends for {}?", topic),
            format!("How does {} impact different sectors and industries?", topic),
            format!("What are the technical foundations and implementation details of {}?", topic),
            format!("What regulatory and policy considerations affect {}?", topic),
            format!("What are the economic and market dynamics of {}?", topic),
            format!("What are the ethical and social implications of {}?", topic),
        ];

        templates.into_iter().take(team_size).collect()
    }

    /// Estima matriz de overlap entre subproblemas.
    fn estimate_overlap_matrix(&self, subproblems: &[String]) -> Vec<Vec<f32>> {
        let n = subproblems.len();
        let mut matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    matrix[i][j] = 1.0;
                } else {
                    // Calcula overlap baseado em palavras comuns
                    let words_i: std::collections::HashSet<&str> =
                        subproblems[i].split_whitespace().collect();
                    let words_j: std::collections::HashSet<&str> =
                        subproblems[j].split_whitespace().collect();

                    let intersection = words_i.intersection(&words_j).count();
                    let union = words_i.union(&words_j).count();

                    matrix[i][j] = intersection as f32 / union as f32;
                }
            }
        }

        matrix
    }

    /// Estima cobertura dos subproblemas em relação à pergunta original.
    fn estimate_coverage(&self, subproblems: &[String], question: &str) -> f32 {
        let question_words: std::collections::HashSet<&str> =
            question.split_whitespace().collect();

        let mut covered_words = std::collections::HashSet::new();

        for subproblem in subproblems {
            for word in subproblem.split_whitespace() {
                if question_words.contains(word) {
                    covered_words.insert(word);
                }
            }
        }

        // Adiciona um boost baseado no número de subproblemas (mais = mais cobertura)
        let word_coverage = covered_words.len() as f32 / question_words.len().max(1) as f32;
        let subproblem_boost = (subproblems.len() as f32 * 0.1).min(0.3);

        (word_coverage + subproblem_boost).min(1.0)
    }

    /// Valida se o plano atende aos critérios de qualidade.
    fn validate_plan(&self, plan: &ResearchPlan) -> Result<(), PlannerError> {
        // Verifica se há subproblemas suficientes
        if plan.subproblems.is_empty() {
            return Err(PlannerError::ParseError("No subproblems generated".into()));
        }

        // Verifica overlap máximo
        if let Some(ref matrix) = plan.overlap_matrix {
            for i in 0..matrix.len() {
                for j in (i + 1)..matrix.len() {
                    if matrix[i][j] > 0.5 {
                        log::warn!(
                            "High overlap ({:.1}%) between subproblems {} and {}",
                            matrix[i][j] * 100.0,
                            i,
                            j
                        );
                    }
                }
            }
        }

        // Verifica cobertura mínima
        if plan.coverage_score < 0.7 {
            log::warn!(
                "Low coverage score ({:.1}%), consider adding more subproblems",
                plan.coverage_score * 100.0
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use async_trait::async_trait;

    // Mock LLM client
    struct MockLlmClient;

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn decide_action(
            &self,
            _prompt: &crate::agent::AgentPrompt,
            _permissions: &crate::agent::ActionPermissions,
        ) -> Result<crate::agent::AgentAction, crate::llm::LlmError> {
            Ok(crate::agent::AgentAction::Answer {
                answer: "Mock".into(),
                references: vec![],
                think: "Mock".into(),
            })
        }

        async fn generate_answer(
            &self,
            _prompt: &crate::agent::AgentPrompt,
            _temperature: f32,
        ) -> Result<crate::llm::LlmResponse, crate::llm::LlmError> {
            Ok(crate::llm::LlmResponse {
                answer: "Mock".into(),
                references: vec![],
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
            })
        }

        async fn embed(&self, _text: &str) -> Result<crate::llm::EmbeddingResult, crate::llm::LlmError> {
            Ok(crate::llm::EmbeddingResult {
                vector: vec![0.0; 1536],
                tokens_used: 10,
            })
        }

        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<crate::llm::EmbeddingResult>, crate::llm::LlmError> {
            Ok(texts.iter().map(|_| crate::llm::EmbeddingResult {
                vector: vec![0.0; 1536],
                tokens_used: 10,
            }).collect())
        }

        async fn evaluate(
            &self,
            _question: &str,
            _answer: &str,
            _criteria: &str,
        ) -> Result<crate::llm::EvaluationResponse, crate::llm::LlmError> {
            Ok(crate::llm::EvaluationResponse {
                passed: true,
                reasoning: "Mock".into(),
                confidence: 0.95,
            })
        }

        async fn determine_eval_types(
            &self,
            _question: &str,
        ) -> Result<Vec<crate::evaluation::EvaluationType>, crate::llm::LlmError> {
            Ok(vec![crate::evaluation::EvaluationType::Definitive])
        }

        async fn generate_code(
            &self,
            _problem: &str,
            _available_vars: &str,
            _previous_attempts: &[(String, Option<String>)],
        ) -> Result<crate::llm::CodeGenResponse, crate::llm::LlmError> {
            Ok(crate::llm::CodeGenResponse {
                code: "return 42;".into(),
                think: "Mock code generation".into(),
            })
        }

        async fn generate_python_code(
            &self,
            _problem: &str,
            _available_vars: &str,
            _previous_attempts: &[(String, Option<String>)],
        ) -> Result<crate::llm::CodeGenResponse, crate::llm::LlmError> {
            Ok(crate::llm::CodeGenResponse {
                code: "print(42)".into(),
                think: "Mock Python code generation".into(),
            })
        }

        async fn choose_coding_language(
            &self,
            _problem: &str,
        ) -> Result<crate::agent::SandboxLanguage, crate::llm::LlmError> {
            Ok(crate::agent::SandboxLanguage::JavaScript)
        }
    }

    #[tokio::test]
    async fn test_planner_basic() {
        let client = Arc::new(MockLlmClient);
        let planner = ResearchPlanner::new(client);
        let mut tracker = TokenTracker::new(Some(100000));

        let result = planner
            .plan_research(
                "What is the future of artificial intelligence in healthcare?",
                3,
                "AI is transforming medical diagnosis and treatment.",
                &mut tracker,
            )
            .await;

        assert!(result.is_ok());
        let subproblems = result.unwrap();
        assert_eq!(subproblems.len(), 3);
    }

    #[tokio::test]
    async fn test_planner_invalid_team_size() {
        let client = Arc::new(MockLlmClient);
        let planner = ResearchPlanner::new(client);
        let mut tracker = TokenTracker::new(Some(100000));

        let result = planner
            .plan_research(
                "What is the future of AI?",
                1, // Too small
                "",
                &mut tracker,
            )
            .await;

        assert!(matches!(result, Err(PlannerError::InvalidTeamSize(_))));
    }

    #[tokio::test]
    async fn test_planner_question_too_simple() {
        let client = Arc::new(MockLlmClient);
        let planner = ResearchPlanner::new(client);
        let mut tracker = TokenTracker::new(Some(100000));

        let result = planner
            .plan_research(
                "What is AI?", // Too short
                3,
                "",
                &mut tracker,
            )
            .await;

        assert!(matches!(result, Err(PlannerError::QuestionTooSimple)));
    }

    #[test]
    fn test_dimension_identification() {
        let client = Arc::new(MockLlmClient);
        let planner = ResearchPlanner::new(client);

        let dims = planner.identify_dimensions(
            "What is the future impact of AI on business security?",
            "Technology is evolving rapidly.",
        );

        assert!(dims.contains(&"temporal_future".to_string()));
        assert!(dims.contains(&"stakeholder_business".to_string()));
        assert!(dims.contains(&"technical_security".to_string()));
    }

    #[test]
    fn test_topic_extraction() {
        let client = Arc::new(MockLlmClient);
        let planner = ResearchPlanner::new(client);

        let topic = planner.extract_topic("What is the future of artificial intelligence?");
        assert!(topic.contains("artificial intelligence"));
    }
}
