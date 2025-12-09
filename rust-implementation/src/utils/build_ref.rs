// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BUILD-REF - Sistema de Referências Semânticas
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Baseado em src/tools/build-ref.ts do TypeScript.
// Constrói referências precisas entre a resposta e as fontes web usando:
// - Chunking de texto (segment.rs)
// - Embeddings semânticos (llm.rs)
// - Similaridade cosseno SIMD (simd.rs)
// - Inserção de marcadores [^1], [^2] na resposta
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashSet;
use std::sync::Arc;

use crate::llm::LlmClient;
use crate::performance::cosine_similarity;
use crate::types::{KnowledgeItem, KnowledgeType, Reference};
use crate::utils::segment::{chunk_text, ChunkOptions};

/// Erro do ReferenceBuilder
#[derive(Debug)]
pub enum ReferenceError {
    /// Erro de embedding
    EmbeddingFailed(String),
    /// Sem conteúdo web para referenciar
    NoWebContent,
    /// Erro interno
    Internal(String),
}

impl std::fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmbeddingFailed(msg) => write!(f, "Embedding failed: {}", msg),
            Self::NoWebContent => write!(f, "No web content available for references"),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ReferenceError {}

/// Configuração do ReferenceBuilder
#[derive(Debug, Clone)]
pub struct ReferenceBuilderConfig {
    /// Tamanho mínimo do chunk em caracteres (default: 80)
    pub min_chunk_length: usize,
    /// Número máximo de referências a incluir (default: 10)
    pub max_references: usize,
    /// Score mínimo de relevância (0.0 - 1.0) (default: 0.7)
    pub min_relevance_score: f32,
    /// Filtrar apenas por esses hostnames (vazio = todos)
    pub only_hostnames: Vec<String>,
}

impl Default for ReferenceBuilderConfig {
    fn default() -> Self {
        Self {
            min_chunk_length: 80,
            max_references: 10,
            min_relevance_score: 0.7,
            only_hostnames: Vec::new(),
        }
    }
}

impl ReferenceBuilderConfig {
    /// Cria config com valores customizados
    pub fn new(
        min_chunk_length: usize,
        max_references: usize,
        min_relevance_score: f32,
    ) -> Self {
        Self {
            min_chunk_length,
            max_references,
            min_relevance_score,
            only_hostnames: Vec::new(),
        }
    }

    /// Define filtro de hostnames
    pub fn with_hostnames(mut self, hostnames: Vec<String>) -> Self {
        self.only_hostnames = hostnames;
        self
    }
}

/// Chunk de conteúdo web com metadados
#[derive(Debug, Clone)]
struct WebChunk {
    /// URL da fonte
    url: String,
    /// Título da página
    title: String,
    /// Texto do chunk
    text: String,
    /// Índice original no array de chunks
    index: usize,
}

/// Match entre chunk da resposta e chunk web
#[derive(Debug, Clone)]
struct ChunkMatch {
    /// Chunk web que deu match
    web_chunk: WebChunk,
    /// Índice do chunk da resposta
    answer_chunk_index: usize,
    /// Posição (start, end) no texto da resposta
    answer_position: (usize, usize),
    /// Texto do chunk da resposta
    answer_chunk: String,
    /// Score de similaridade (0.0 - 1.0)
    relevance_score: f32,
}

/// Resultado da construção de referências
#[derive(Debug, Clone)]
pub struct ReferenceResult {
    /// Resposta com marcadores [^1], [^2] inseridos
    pub answer: String,
    /// Lista de referências ordenadas
    pub references: Vec<Reference>,
}

/// Builder de referências semânticas
pub struct ReferenceBuilder {
    config: ReferenceBuilderConfig,
    llm_client: Arc<dyn LlmClient>,
}

impl ReferenceBuilder {
    /// Cria um novo ReferenceBuilder
    pub fn new(llm_client: Arc<dyn LlmClient>, config: ReferenceBuilderConfig) -> Self {
        Self { config, llm_client }
    }

    /// Cria com configuração padrão
    pub fn with_defaults(llm_client: Arc<dyn LlmClient>) -> Self {
        Self::new(llm_client, ReferenceBuilderConfig::default())
    }

    /// Constrói referências semânticas entre resposta e conteúdo web
    ///
    /// # Fluxo
    /// 1. Chunk da resposta
    /// 2. Chunk do conteúdo web
    /// 3. Embeddings batch de todos os chunks
    /// 4. Cosine similarity para encontrar matches
    /// 5. Filtrar por score e dedup
    /// 6. Inserir marcadores [^1], [^2] na resposta
    pub async fn build_references(
        &self,
        answer: &str,
        knowledge: &[KnowledgeItem],
    ) -> Result<ReferenceResult, ReferenceError> {
        log::info!("[build_references] Starting with min_chunk={}, max_refs={}, min_score={}",
            self.config.min_chunk_length,
            self.config.max_references,
            self.config.min_relevance_score
        );

        // Step 1: Chunk da resposta
        let chunk_options = ChunkOptions::newline()
            .with_min_length(self.config.min_chunk_length);
        let answer_result = chunk_text(answer, &chunk_options);

        log::info!("[build_references] Answer chunked into {} valid chunks",
            answer_result.chunks.len()
        );

        if answer_result.is_empty() {
            log::warn!("[build_references] No valid answer chunks, returning without references");
            return Ok(ReferenceResult {
                answer: answer.to_string(),
                references: Vec::new(),
            });
        }

        // Step 2: Coletar e fazer chunk do conteúdo web
        let web_chunks = self.collect_web_chunks(knowledge);

        if web_chunks.is_empty() {
            log::warn!("[build_references] No web content chunks available");
            return Ok(ReferenceResult {
                answer: answer.to_string(),
                references: Vec::new(),
            });
        }

        log::info!("[build_references] Collected {} web chunks from {} knowledge items",
            web_chunks.len(),
            knowledge.iter().filter(|k| k.item_type == KnowledgeType::Url).count()
        );

        // Step 3: Gerar embeddings para todos os chunks em batch
        let mut all_chunks: Vec<String> = Vec::new();

        // Adicionar chunks da resposta
        for chunk in &answer_result.chunks {
            all_chunks.push(chunk.clone());
        }

        // Adicionar chunks web (apenas os válidos)
        for wc in &web_chunks {
            all_chunks.push(wc.text.clone());
        }

        log::info!("[build_references] Getting embeddings for {} total chunks ({} answer + {} web)",
            all_chunks.len(),
            answer_result.chunks.len(),
            web_chunks.len()
        );

        // Obter embeddings
        let embeddings = match self.llm_client.embed_batch(&all_chunks).await {
            Ok(results) => results.into_iter().map(|r| r.vector).collect::<Vec<_>>(),
            Err(e) => {
                log::error!("[build_references] Embedding failed: {:?}, falling back to Jaccard", e);
                return self.build_references_jaccard_fallback(answer, &answer_result, &web_chunks);
            }
        };

        // Separar embeddings
        let answer_embeddings: Vec<&Vec<f32>> = embeddings[..answer_result.chunks.len()].iter().collect();
        let web_embeddings: Vec<&Vec<f32>> = embeddings[answer_result.chunks.len()..].iter().collect();

        // Step 4: Calcular similaridade cosseno entre todos os pares
        let matches = self.compute_matches(
            &answer_result.chunks,
            &answer_result.positions,
            &answer_embeddings,
            &web_chunks,
            &web_embeddings,
        );

        // Step 5: Filtrar matches por score e dedup
        let filtered_matches = self.filter_matches(matches);

        log::info!("[build_references] Selected {} references after filtering",
            filtered_matches.len()
        );

        // Step 6: Construir resultado final com marcadores
        self.build_final_result(answer, filtered_matches)
    }

    /// Coleta e faz chunk do conteúdo web do knowledge
    fn collect_web_chunks(&self, knowledge: &[KnowledgeItem]) -> Vec<WebChunk> {
        let mut web_chunks = Vec::new();
        let mut chunk_index = 0;

        for item in knowledge {
            if item.item_type != KnowledgeType::Url {
                continue;
            }

            // Pegar URL e título das referências
            let (url, title) = if let Some(ref_item) = item.references.first() {
                (ref_item.url.clone(), ref_item.title.clone())
            } else {
                continue;
            };

            // Filtrar por hostname se configurado
            if !self.config.only_hostnames.is_empty() {
                let hostname = extract_hostname(&url);
                if !self.config.only_hostnames.iter().any(|h| h == &hostname) {
                    continue;
                }
            }

            // Fazer chunk do conteúdo
            let chunk_options = ChunkOptions::newline()
                .with_min_length(self.config.min_chunk_length);
            let result = chunk_text(&item.answer, &chunk_options);

            for chunk in result.chunks {
                web_chunks.push(WebChunk {
                    url: url.clone(),
                    title: title.clone(),
                    text: chunk,
                    index: chunk_index,
                });
                chunk_index += 1;
            }
        }

        web_chunks
    }

    /// Calcula matches usando cosine similarity SIMD
    fn compute_matches(
        &self,
        answer_chunks: &[String],
        answer_positions: &[(usize, usize)],
        answer_embeddings: &[&Vec<f32>],
        web_chunks: &[WebChunk],
        web_embeddings: &[&Vec<f32>],
    ) -> Vec<ChunkMatch> {
        let mut all_matches = Vec::new();

        for (i, (answer_chunk, answer_embedding)) in
            answer_chunks.iter().zip(answer_embeddings.iter()).enumerate()
        {
            let answer_position = answer_positions[i];
            let mut matches_for_chunk = Vec::new();

            // Calcular similaridade com cada chunk web
            for (j, (web_chunk, web_embedding)) in
                web_chunks.iter().zip(web_embeddings.iter()).enumerate()
            {
                let score = cosine_similarity(answer_embedding, web_embedding);

                matches_for_chunk.push(ChunkMatch {
                    web_chunk: web_chunk.clone(),
                    answer_chunk_index: i,
                    answer_position,
                    answer_chunk: answer_chunk.clone(),
                    relevance_score: score,
                });

                // Log para debug do top match
                if j == 0 || score > matches_for_chunk.iter().map(|m| m.relevance_score).fold(0.0f32, f32::max) {
                    log::debug!("[compute_matches] Answer chunk {} vs Web chunk {}: score={:.4}",
                        i, j, score
                    );
                }
            }

            // Ordenar por score decrescente
            matches_for_chunk.sort_by(|a, b| {
                b.relevance_score.partial_cmp(&a.relevance_score).unwrap()
            });

            // Adicionar todos os matches
            all_matches.extend(matches_for_chunk);
        }

        // Ordenar todos por relevância
        all_matches.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap()
        });

        // Log estatísticas
        if !all_matches.is_empty() {
            let scores: Vec<f32> = all_matches.iter().map(|m| m.relevance_score).collect();
            let min = scores.iter().cloned().fold(f32::MAX, f32::min);
            let max = scores.iter().cloned().fold(f32::MIN, f32::max);
            let mean = scores.iter().sum::<f32>() / scores.len() as f32;

            log::info!("[compute_matches] Relevance stats: min={:.4}, max={:.4}, mean={:.4}, count={}",
                min, max, mean, scores.len()
            );
        }

        all_matches
    }

    /// Filtra matches por score e remove duplicatas
    fn filter_matches(&self, matches: Vec<ChunkMatch>) -> Vec<ChunkMatch> {
        let mut used_web_chunks = HashSet::new();
        let mut used_answer_chunks = HashSet::new();
        let mut filtered = Vec::new();

        for m in matches {
            // Filtrar por score mínimo
            if m.relevance_score < self.config.min_relevance_score {
                continue;
            }

            // Dedup: cada chunk web e cada chunk answer só pode aparecer uma vez
            let web_key = format!("{}:{}", m.web_chunk.url, m.web_chunk.index);
            if used_web_chunks.contains(&web_key) ||
               used_answer_chunks.contains(&m.answer_chunk_index)
            {
                continue;
            }

            used_web_chunks.insert(web_key);
            used_answer_chunks.insert(m.answer_chunk_index);
            filtered.push(m);

            // Parar se atingiu o máximo
            if filtered.len() >= self.config.max_references {
                break;
            }
        }

        filtered
    }

    /// Constrói resultado final com marcadores inseridos
    fn build_final_result(
        &self,
        original_answer: &str,
        matches: Vec<ChunkMatch>,
    ) -> Result<ReferenceResult, ReferenceError> {
        // Construir referências
        let references: Vec<Reference> = matches.iter().map(|m| {
            Reference {
                url: m.web_chunk.url.clone(),
                title: m.web_chunk.title.clone(),
                exact_quote: Some(m.web_chunk.text.clone()),
                relevance_score: Some(m.relevance_score),
                answer_chunk: Some(m.answer_chunk.clone()),
                answer_position: Some(m.answer_position),
            }
        }).collect();

        // Ordenar matches por posição na resposta
        let mut matches_by_position = matches.clone();
        matches_by_position.sort_by_key(|m| m.answer_position.0);

        // Inserir marcadores [^1], [^2] na resposta
        let mut modified_answer = original_answer.to_string();
        let mut offset = 0;

        for (i, m) in matches_by_position.iter().enumerate() {
            let marker = format!("[^{}]", i + 1);
            let mut insert_position = m.answer_position.1 + offset;

            // Ajustar posição para evitar inserir depois de newlines
            let text_before = &modified_answer[..insert_position.min(modified_answer.len())];
            if let Some(newline_match) = text_before.rfind('\n') {
                let chars_after_newline = text_before.len() - newline_match - 1;
                if chars_after_newline < 3 {
                    // Se muito perto de newline, inserir antes
                    insert_position = newline_match + offset - (text_before.len() - newline_match - 1);
                }
            }

            // Verificar se não estamos no meio de uma tabela
            let text_around = &modified_answer[insert_position.saturating_sub(5)..insert_position.min(modified_answer.len())];
            if text_around.contains('|') {
                // Mover para antes do pipe
                if let Some(pipe_pos) = text_around.rfind('|') {
                    insert_position = insert_position.saturating_sub(5) + pipe_pos;
                }
            }

            // Inserir marcador
            if insert_position <= modified_answer.len() {
                modified_answer.insert_str(insert_position, &marker);
                offset += marker.len();
            }
        }

        log::info!("[build_final_result] Generated {} references with markers", references.len());

        Ok(ReferenceResult {
            answer: modified_answer,
            references,
        })
    }

    /// Fallback usando Jaccard similarity quando embeddings falham
    fn build_references_jaccard_fallback(
        &self,
        answer: &str,
        answer_result: &crate::utils::segment::ChunkResult,
        web_chunks: &[WebChunk],
    ) -> Result<ReferenceResult, ReferenceError> {
        log::warn!("[build_references] Using Jaccard fallback");

        let mut all_matches = Vec::new();

        for (i, answer_chunk) in answer_result.chunks.iter().enumerate() {
            let answer_position = answer_result.positions[i];

            for web_chunk in web_chunks {
                let score = jaccard_similarity(answer_chunk, &web_chunk.text);

                all_matches.push(ChunkMatch {
                    web_chunk: web_chunk.clone(),
                    answer_chunk_index: i,
                    answer_position,
                    answer_chunk: answer_chunk.clone(),
                    relevance_score: score,
                });
            }
        }

        // Ordenar e filtrar
        all_matches.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap()
        });

        let filtered = self.filter_matches(all_matches);
        self.build_final_result(answer, filtered)
    }
}

/// Extrai hostname de uma URL
fn extract_hostname(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Calcula Jaccard similarity entre dois textos (fallback)
fn jaccard_similarity(a: &str, b: &str) -> f32 {
    let words_a: HashSet<&str> = a.split_whitespace().collect();
    let words_b: HashSet<&str> = b.split_whitespace().collect();

    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f32 / union as f32
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hostname() {
        assert_eq!(extract_hostname("https://example.com/path"), "example.com");
        assert_eq!(extract_hostname("http://test.org/a/b"), "test.org");
        assert_eq!(extract_hostname("https://sub.domain.com/"), "sub.domain.com");
    }

    #[test]
    fn test_jaccard_similarity() {
        // Textos idênticos
        let sim1 = jaccard_similarity("hello world test", "hello world test");
        assert!((sim1 - 1.0).abs() < 0.01);

        // Textos completamente diferentes
        let sim2 = jaccard_similarity("apple banana", "car house");
        assert!(sim2 < 0.01);

        // Textos parcialmente similares
        let sim3 = jaccard_similarity("hello world", "hello there");
        assert!(sim3 > 0.0 && sim3 < 1.0);
    }

    #[test]
    fn test_config_default() {
        let config = ReferenceBuilderConfig::default();
        assert_eq!(config.min_chunk_length, 80);
        assert_eq!(config.max_references, 10);
        assert!((config.min_relevance_score - 0.7).abs() < 0.01);
        assert!(config.only_hostnames.is_empty());
    }

    #[test]
    fn test_config_builder() {
        let config = ReferenceBuilderConfig::new(100, 5, 0.8)
            .with_hostnames(vec!["example.com".into()]);

        assert_eq!(config.min_chunk_length, 100);
        assert_eq!(config.max_references, 5);
        assert!((config.min_relevance_score - 0.8).abs() < 0.01);
        assert_eq!(config.only_hostnames, vec!["example.com"]);
    }

    #[test]
    fn test_reference_error_display() {
        let err1 = ReferenceError::EmbeddingFailed("test error".into());
        assert!(err1.to_string().contains("Embedding failed"));

        let err2 = ReferenceError::NoWebContent;
        assert!(err2.to_string().contains("No web content"));

        let err3 = ReferenceError::Internal("internal".into());
        assert!(err3.to_string().contains("Internal"));
    }
}
