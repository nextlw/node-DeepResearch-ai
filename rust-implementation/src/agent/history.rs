// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HISTÓRICO DE SESSÕES - Multi-Backend (Local/Postgres/Qdrant)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tui::ResearchSession;

/// Resumo de uma sessão anterior (versão leve para contexto do LLM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// ID da sessão
    pub id: String,
    /// Pergunta pesquisada
    pub question: String,
    /// Resposta resumida (primeiros 500 chars)
    pub answer_preview: String,
    /// Quantidade de referências
    pub references_count: usize,
    /// URLs visitadas
    pub urls_visited: usize,
    /// Data/hora (ISO 8601)
    pub timestamp: String,
    /// Se teve sucesso
    pub success: bool,
    /// Score de relevância (para busca semântica)
    #[serde(default)]
    pub relevance_score: Option<f32>,
}

impl SessionSummary {
    /// Cria um resumo a partir de uma sessão completa
    pub fn from_session(session: &ResearchSession) -> Self {
        let answer_preview = session
            .answer
            .as_ref()
            .map(|a| {
                if a.len() > 500 {
                    format!("{}...", &a[..500])
                } else {
                    a.clone()
                }
            })
            .unwrap_or_default();

        Self {
            id: session.id.clone(),
            question: session.question.clone(),
            answer_preview,
            references_count: session.references.len(),
            urls_visited: session.visited_urls.len(),
            timestamp: session.started_at.clone(),
            success: session.success,
            relevance_score: None,
        }
    }

    /// Formata para contexto do LLM
    pub fn format_for_llm(&self) -> String {
        format!(
            "[{}] Q: {}\nA: {}\n({} refs, {} URLs, {})",
            &self.id[..8],
            self.question,
            self.answer_preview,
            self.references_count,
            self.urls_visited,
            if self.success { "✓" } else { "✗" }
        )
    }
}

/// Resultado de busca no histórico
#[derive(Debug, Clone)]
pub struct HistorySearchResult {
    /// Sessões encontradas
    pub sessions: Vec<SessionSummary>,
    /// Total de sessões no backend
    pub total_sessions: usize,
    /// Backend usado
    pub backend: &'static str,
    /// Tempo de busca em ms
    pub search_time_ms: u64,
}

impl HistorySearchResult {
    /// Formata todas as sessões para contexto do LLM
    pub fn format_for_llm(&self) -> String {
        if self.sessions.is_empty() {
            return "Nenhuma sessão anterior encontrada.".to_string();
        }

        let mut output = format!(
            "=== HISTÓRICO DE PESQUISAS ({} de {} sessões) ===\n\n",
            self.sessions.len(),
            self.total_sessions
        );

        for (i, session) in self.sessions.iter().enumerate() {
            output.push_str(&format!("--- Sessão {} ---\n", i + 1));
            output.push_str(&session.format_for_llm());
            output.push_str("\n\n");
        }

        output
    }
}

/// Opções de busca no histórico
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    /// Quantidade máxima de sessões a retornar
    pub limit: usize,
    /// Filtro por texto (busca em question e answer)
    pub text_filter: Option<String>,
    /// Busca semântica (embedding da query)
    pub semantic_query: Option<Vec<f32>>,
    /// Threshold de similaridade para busca semântica
    pub similarity_threshold: f32,
    /// Apenas sessões com sucesso
    pub only_successful: bool,
    /// Ordenar por data (mais recentes primeiro)
    pub order_by_date: bool,
}

impl HistoryQuery {
    /// Cria uma nova query de histórico com limite de resultados
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            text_filter: None,
            semantic_query: None,
            similarity_threshold: 0.7,
            only_successful: false,
            order_by_date: true,
        }
    }

    /// Adiciona filtro por texto (busca em question e answer)
    pub fn with_text_filter(mut self, filter: &str) -> Self {
        self.text_filter = Some(filter.to_string());
        self
    }

    /// Adiciona busca semântica por embedding
    pub fn with_semantic_query(mut self, embedding: Vec<f32>) -> Self {
        self.semantic_query = Some(embedding);
        self
    }

    /// Filtra apenas sessões bem-sucedidas
    pub fn only_successful(mut self) -> Self {
        self.only_successful = true;
        self
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TRAIT: HistoryBackend - Abstração para diferentes backends
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Trait para backends de armazenamento de histórico de sessões.
///
/// Implementações disponíveis:
/// - `LocalBackend`: Arquivos JSON locais (desenvolvimento)
/// - `PostgresBackend`: PostgreSQL (produção no Railway)
/// - `QdrantBackend`: Qdrant para busca semântica vetorial
#[async_trait]
pub trait HistoryBackend: Send + Sync {
    /// Nome do backend
    fn name(&self) -> &'static str;

    /// Verifica se o backend está disponível
    async fn is_available(&self) -> bool;

    /// Busca sessões no histórico
    async fn search(&self, query: &HistoryQuery) -> anyhow::Result<HistorySearchResult>;

    /// Salva uma sessão no histórico
    async fn save(&self, session: &ResearchSession) -> anyhow::Result<()>;

    /// Obtém uma sessão específica por ID
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<ResearchSession>>;

    /// Conta total de sessões
    async fn count(&self) -> anyhow::Result<usize>;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BACKEND: Local (arquivos JSON) - Para desenvolvimento
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Backend local que armazena sessões em arquivos JSON.
///
/// Ideal para desenvolvimento e testes. Em produção, use PostgresBackend.
pub struct LocalBackend {
    sessions_dir: PathBuf,
}

impl LocalBackend {
    /// Cria um novo backend local com diretório customizado
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    /// Cria backend local usando o diretório padrão `./sessions`
    pub fn default_path() -> Self {
        let sessions_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("sessions");
        Self::new(sessions_dir)
    }

    fn load_all_sessions(&self) -> Vec<ResearchSession> {
        let mut sessions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.sessions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(session) = serde_json::from_str::<ResearchSession>(&content) {
                            sessions.push(session);
                        }
                    }
                }
            }
        }

        // Ordenar por data (mais recentes primeiro)
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        sessions
    }
}

#[async_trait]
impl HistoryBackend for LocalBackend {
    fn name(&self) -> &'static str {
        "local"
    }

    async fn is_available(&self) -> bool {
        self.sessions_dir.exists()
    }

    async fn search(&self, query: &HistoryQuery) -> anyhow::Result<HistorySearchResult> {
        let start = std::time::Instant::now();
        let all_sessions = self.load_all_sessions();
        let total = all_sessions.len();

        let filtered: Vec<_> = all_sessions
            .into_iter()
            .filter(|s| {
                // Filtro de sucesso
                if query.only_successful && !s.success {
                    return false;
                }

                // Filtro de texto
                if let Some(ref filter) = query.text_filter {
                    let filter_lower = filter.to_lowercase();
                    let question_match = s.question.to_lowercase().contains(&filter_lower);
                    let answer_match = s
                        .answer
                        .as_ref()
                        .map(|a| a.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false);
                    if !question_match && !answer_match {
                        return false;
                    }
                }

                true
            })
            .take(query.limit)
            .map(|s| SessionSummary::from_session(&s))
            .collect();

        Ok(HistorySearchResult {
            sessions: filtered,
            total_sessions: total,
            backend: "local",
            search_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn save(&self, session: &ResearchSession) -> anyhow::Result<()> {
        // Criar diretório se não existir
        std::fs::create_dir_all(&self.sessions_dir)?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.json", timestamp, &session.id[..8]);
        let path = self.sessions_dir.join(filename);

        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(path, json)?;

        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<ResearchSession>> {
        let sessions = self.load_all_sessions();
        Ok(sessions.into_iter().find(|s| s.id == id || s.id.starts_with(id)))
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.load_all_sessions().len())
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BACKEND: PostgreSQL - Para produção no Railway
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(feature = "postgres")]
pub struct PostgresBackend {
    pool: sqlx::PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresBackend {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn from_env() -> anyhow::Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("POSTGRES_URL"))
            .map_err(|_| anyhow::anyhow!("DATABASE_URL ou POSTGRES_URL não definido"))?;
        Self::new(&url).await
    }

    /// Executa migrations para criar tabelas necessárias
    pub async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS research_sessions (
                id UUID PRIMARY KEY,
                question TEXT NOT NULL,
                answer TEXT,
                references JSONB DEFAULT '[]',
                visited_urls JSONB DEFAULT '[]',
                logs JSONB DEFAULT '[]',
                personas JSONB DEFAULT '{}',
                timing JSONB DEFAULT '{}',
                stats JSONB DEFAULT '{}',
                success BOOLEAN DEFAULT false,
                error TEXT,
                parallel_batches JSONB DEFAULT '[]',
                all_tasks JSONB DEFAULT '[]',
                completed_steps JSONB DEFAULT '[]',
                started_at TIMESTAMPTZ NOT NULL,
                finished_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                -- Full-text search
                search_vector TSVECTOR GENERATED ALWAYS AS (
                    setweight(to_tsvector('portuguese', coalesce(question, '')), 'A') ||
                    setweight(to_tsvector('portuguese', coalesce(answer, '')), 'B')
                ) STORED
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON research_sessions(started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_sessions_success ON research_sessions(success);
            CREATE INDEX IF NOT EXISTS idx_sessions_search ON research_sessions USING GIN(search_vector);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl HistoryBackend for PostgresBackend {
    fn name(&self) -> &'static str {
        "postgres"
    }

    async fn is_available(&self) -> bool {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }

    async fn search(&self, query: &HistoryQuery) -> anyhow::Result<HistorySearchResult> {
        let start = std::time::Instant::now();

        // Contar total
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM research_sessions")
            .fetch_one(&self.pool)
            .await?;

        // Construir query dinâmica
        let mut sql = String::from(
            "SELECT id, question, answer, references, visited_urls, started_at, success
             FROM research_sessions WHERE 1=1"
        );

        if query.only_successful {
            sql.push_str(" AND success = true");
        }

        if query.text_filter.is_some() {
            sql.push_str(" AND search_vector @@ plainto_tsquery('portuguese', $1)");
        }

        sql.push_str(" ORDER BY started_at DESC");
        sql.push_str(&format!(" LIMIT {}", query.limit));

        let rows: Vec<(String, String, Option<String>, serde_json::Value, serde_json::Value, chrono::DateTime<chrono::Utc>, bool)> =
            if let Some(ref filter) = query.text_filter {
                sqlx::query_as(&sql)
                    .bind(filter)
                    .fetch_all(&self.pool)
                    .await?
            } else {
                sqlx::query_as(&sql)
                    .fetch_all(&self.pool)
                    .await?
            };

        let sessions: Vec<SessionSummary> = rows
            .into_iter()
            .map(|(id, question, answer, refs, urls, timestamp, success)| {
                let refs_arr: Vec<String> = serde_json::from_value(refs).unwrap_or_default();
                let urls_arr: Vec<String> = serde_json::from_value(urls).unwrap_or_default();

                SessionSummary {
                    id,
                    question,
                    answer_preview: answer.map(|a| {
                        if a.len() > 500 { format!("{}...", &a[..500]) } else { a }
                    }).unwrap_or_default(),
                    references_count: refs_arr.len(),
                    urls_visited: urls_arr.len(),
                    timestamp: timestamp.to_rfc3339(),
                    success,
                    relevance_score: None,
                }
            })
            .collect();

        Ok(HistorySearchResult {
            sessions,
            total_sessions: total.0 as usize,
            backend: "postgres",
            search_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn save(&self, session: &ResearchSession) -> anyhow::Result<()> {
        let id = uuid::Uuid::parse_str(&session.id)?;
        let started_at = chrono::DateTime::parse_from_rfc3339(&session.started_at)?;
        let finished_at = session.finished_at.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());

        sqlx::query(
            r#"
            INSERT INTO research_sessions (
                id, question, answer, references, visited_urls, logs,
                personas, timing, stats, success, error,
                parallel_batches, all_tasks, completed_steps,
                started_at, finished_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            ON CONFLICT (id) DO UPDATE SET
                answer = EXCLUDED.answer,
                references = EXCLUDED.references,
                visited_urls = EXCLUDED.visited_urls,
                logs = EXCLUDED.logs,
                success = EXCLUDED.success,
                error = EXCLUDED.error,
                finished_at = EXCLUDED.finished_at
            "#
        )
        .bind(id)
        .bind(&session.question)
        .bind(&session.answer)
        .bind(serde_json::to_value(&session.references)?)
        .bind(serde_json::to_value(&session.visited_urls)?)
        .bind(serde_json::to_value(&session.logs)?)
        .bind(serde_json::to_value(&session.personas)?)
        .bind(serde_json::to_value(&session.timing)?)
        .bind(serde_json::to_value(&session.stats)?)
        .bind(session.success)
        .bind(&session.error)
        .bind(serde_json::to_value(&session.parallel_batches)?)
        .bind(serde_json::to_value(&session.all_tasks)?)
        .bind(serde_json::to_value(&session.completed_steps)?)
        .bind(started_at)
        .bind(finished_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<ResearchSession>> {
        let uuid = uuid::Uuid::parse_str(id)?;

        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT row_to_json(research_sessions) FROM research_sessions WHERE id = $1"
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((json,)) => Ok(Some(serde_json::from_value(json)?)),
            None => Ok(None),
        }
    }

    async fn count(&self) -> anyhow::Result<usize> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM research_sessions")
            .fetch_one(&self.pool)
            .await?;
        Ok(count as usize)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BACKEND: Qdrant - Para busca semântica em produção
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(feature = "qdrant")]
pub struct QdrantBackend {
    client: qdrant_client::Qdrant,
    collection_name: String,
    /// Backend de fallback para dados completos (Qdrant só guarda embeddings)
    fallback: Box<dyn HistoryBackend>,
}

#[cfg(feature = "qdrant")]
impl QdrantBackend {
    pub async fn new(
        url: &str,
        api_key: Option<&str>,
        collection_name: &str,
        fallback: Box<dyn HistoryBackend>,
    ) -> anyhow::Result<Self> {
        let mut config = qdrant_client::QdrantClientConfig::from_url(url);
        if let Some(key) = api_key {
            config = config.api_key(key);
        }

        let client = qdrant_client::Qdrant::new(Some(config))?;

        Ok(Self {
            client,
            collection_name: collection_name.to_string(),
            fallback,
        })
    }

    pub async fn from_env(fallback: Box<dyn HistoryBackend>) -> anyhow::Result<Self> {
        let url = std::env::var("QDRANT_URL")
            .unwrap_or_else(|_| "http://localhost:6333".to_string());
        let api_key = std::env::var("QDRANT_API_KEY").ok();
        let collection = std::env::var("QDRANT_COLLECTION")
            .unwrap_or_else(|_| "research_sessions".to_string());

        Self::new(&url, api_key.as_deref(), &collection, fallback).await
    }

    /// Cria a collection se não existir
    pub async fn ensure_collection(&self, vector_size: u64) -> anyhow::Result<()> {
        use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder};

        let exists = self.client.collection_exists(&self.collection_name).await?;

        if !exists {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection_name)
                        .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
                )
                .await?;
        }

        Ok(())
    }
}

#[cfg(feature = "qdrant")]
#[async_trait]
impl HistoryBackend for QdrantBackend {
    fn name(&self) -> &'static str {
        "qdrant"
    }

    async fn is_available(&self) -> bool {
        self.client.health_check().await.is_ok()
    }

    async fn search(&self, query: &HistoryQuery) -> anyhow::Result<HistorySearchResult> {
        use qdrant_client::qdrant::{SearchPointsBuilder, Filter, Condition, FieldCondition, Match};

        let start = std::time::Instant::now();

        // Se tem query semântica, usar Qdrant
        if let Some(ref embedding) = query.semantic_query {
            let mut search = SearchPointsBuilder::new(
                &self.collection_name,
                embedding.clone(),
                query.limit as u64,
            )
            .score_threshold(query.similarity_threshold)
            .with_payload(true);

            // Filtro de sucesso
            if query.only_successful {
                search = search.filter(Filter::must([
                    Condition::Field(FieldCondition {
                        key: "success".to_string(),
                        r#match: Some(Match::boolean(true)),
                        ..Default::default()
                    })
                ]));
            }

            let results = self.client.search_points(search).await?;

            let sessions: Vec<SessionSummary> = results
                .result
                .into_iter()
                .filter_map(|point| {
                    let payload = point.payload;
                    Some(SessionSummary {
                        id: payload.get("id")?.as_str()?.to_string(),
                        question: payload.get("question")?.as_str()?.to_string(),
                        answer_preview: payload.get("answer_preview")?.as_str()?.to_string(),
                        references_count: payload.get("references_count")?.as_integer()? as usize,
                        urls_visited: payload.get("urls_visited")?.as_integer()? as usize,
                        timestamp: payload.get("timestamp")?.as_str()?.to_string(),
                        success: payload.get("success")?.as_bool()?,
                        relevance_score: Some(point.score),
                    })
                })
                .collect();

            let total = self.count().await.unwrap_or(0);

            return Ok(HistorySearchResult {
                sessions,
                total_sessions: total,
                backend: "qdrant",
                search_time_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Sem query semântica, usar fallback
        self.fallback.search(query).await
    }

    async fn save(&self, session: &ResearchSession) -> anyhow::Result<()> {
        // Salvar no fallback primeiro (dados completos)
        self.fallback.save(session).await?;

        // TODO: Gerar embedding e salvar no Qdrant
        // Isso requer acesso ao LLM client para gerar embeddings
        // Por enquanto, apenas salva no fallback

        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<ResearchSession>> {
        // Dados completos estão no fallback
        self.fallback.get_by_id(id).await
    }

    async fn count(&self) -> anyhow::Result<usize> {
        let info = self.client.collection_info(&self.collection_name).await?;
        Ok(info.result.map(|r| r.points_count.unwrap_or(0) as usize).unwrap_or(0))
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HISTORY SERVICE - Gerenciador unificado
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Serviço de histórico com fallback automático entre backends.
///
/// Tenta usar backends na ordem de prioridade:
/// 1. PostgreSQL (se feature `postgres` habilitada e disponível)
/// 2. Local (arquivos JSON, sempre disponível)
pub struct HistoryService {
    backends: Vec<Box<dyn HistoryBackend>>,
}

impl HistoryService {
    /// Cria um novo serviço de histórico vazio
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    /// Adiciona um backend (primeiro adicionado = maior prioridade)
    pub fn add_backend(mut self, backend: Box<dyn HistoryBackend>) -> Self {
        self.backends.push(backend);
        self
    }

    /// Cria serviço apenas com backend local
    pub fn local_only() -> Self {
        Self::new().add_backend(Box::new(LocalBackend::default_path()))
    }

    /// Cria serviço com auto-detecção de backends disponíveis
    pub async fn auto_detect() -> Self {
        let mut service = Self::new();

        // Tentar Postgres primeiro (produção)
        #[cfg(feature = "postgres")]
        {
            if let Ok(pg) = PostgresBackend::from_env().await {
                if pg.is_available().await {
                    log::info!("📦 HistoryService: PostgreSQL disponível");
                    service.backends.push(Box::new(pg));
                }
            }
        }

        // Sempre ter local como fallback
        let local = LocalBackend::default_path();
        if local.is_available().await {
            log::info!("📁 HistoryService: Local backend disponível");
            service.backends.push(Box::new(local));
        }

        service
    }

    /// Retorna o backend primário disponível
    fn primary_backend(&self) -> Option<&dyn HistoryBackend> {
        self.backends.first().map(|b| b.as_ref())
    }

    /// Busca no histórico usando o backend primário
    pub async fn search(&self, query: &HistoryQuery) -> anyhow::Result<HistorySearchResult> {
        match self.primary_backend() {
            Some(backend) => backend.search(query).await,
            None => Ok(HistorySearchResult {
                sessions: vec![],
                total_sessions: 0,
                backend: "none",
                search_time_ms: 0,
            }),
        }
    }

    /// Busca as últimas N sessões
    pub async fn get_recent(&self, count: usize) -> anyhow::Result<HistorySearchResult> {
        self.search(&HistoryQuery::new(count)).await
    }

    /// Busca sessões por texto
    pub async fn search_text(
        &self,
        text: &str,
        limit: usize,
    ) -> anyhow::Result<HistorySearchResult> {
        self.search(&HistoryQuery::new(limit).with_text_filter(text))
            .await
    }

    /// Busca semântica (requer embedding)
    pub async fn search_semantic(
        &self,
        embedding: Vec<f32>,
        limit: usize,
    ) -> anyhow::Result<HistorySearchResult> {
        self.search(&HistoryQuery::new(limit).with_semantic_query(embedding))
            .await
    }

    /// Salva sessão em todos os backends
    pub async fn save(&self, session: &ResearchSession) -> anyhow::Result<()> {
        for backend in &self.backends {
            if let Err(e) = backend.save(session).await {
                log::warn!("Falha ao salvar em {}: {}", backend.name(), e);
            }
        }
        Ok(())
    }

    /// Obtém sessão por ID
    pub async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<ResearchSession>> {
        for backend in &self.backends {
            if let Ok(Some(session)) = backend.get_by_id(id).await {
                return Ok(Some(session));
            }
        }
        Ok(None)
    }

    /// Lista backends disponíveis
    pub fn available_backends(&self) -> Vec<&'static str> {
        self.backends.iter().map(|b| b.name()).collect()
    }
}

impl Default for HistoryService {
    fn default() -> Self {
        Self::local_only()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_summary_format() {
        let summary = SessionSummary {
            id: "abc12345-6789-0000-0000-000000000000".to_string(),
            question: "Como funciona X?".to_string(),
            answer_preview: "X funciona assim...".to_string(),
            references_count: 5,
            urls_visited: 10,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            success: true,
            relevance_score: Some(0.95),
        };

        let formatted = summary.format_for_llm();
        assert!(formatted.contains("abc12345"));
        assert!(formatted.contains("Como funciona X?"));
        assert!(formatted.contains("5 refs"));
        assert!(formatted.contains("10 URLs"));
    }

    #[test]
    fn test_history_query_builder() {
        let query = HistoryQuery::new(10)
            .with_text_filter("rust")
            .only_successful();

        assert_eq!(query.limit, 10);
        assert_eq!(query.text_filter, Some("rust".to_string()));
        assert!(query.only_successful);
    }

    #[tokio::test]
    async fn test_local_backend_availability() {
        let backend = LocalBackend::default_path();
        // O teste não garante que existe, apenas que não falha
        let _ = backend.is_available().await;
    }
}
