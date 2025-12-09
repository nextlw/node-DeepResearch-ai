//! # Response Reducer
//!
//! Este módulo implementa a mesclagem de respostas de múltiplos agentes,
//! combinando insights únicos enquanto elimina duplicatas.
//!
//! ## Baseado em
//! `src/tools/reducer.ts` do projeto TypeScript original.
//!
//! ## Lógica de Merge
//! 1. Identificar clusters de conteúdo similar
//! 2. Selecionar melhor versão de cada cluster
//! 3. Eliminar duplicatas puras
//! 4. Preservar detalhes complementares
//! 5. Se ratio < 60%, retorna join simples

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::llm::LlmClient;
use crate::utils::TokenTracker;
use crate::performance::simd::cosine_similarity;

/// Erro que pode ocorrer durante a redução de respostas.
#[derive(Debug, thiserror::Error)]
pub enum ReducerError {
    /// Erro na comunicação com o LLM.
    #[error("LLM error: {0}")]
    LlmError(String),
    
    /// Nenhuma resposta válida fornecida.
    #[error("No valid answers to reduce")]
    NoValidAnswers,
    
    /// Erro de parse na resposta do LLM.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Configuração do redutor de respostas.
#[derive(Debug, Clone)]
pub struct ReducerConfig {
    /// Proporção mínima do resultado em relação ao join (0.0 - 1.0).
    /// Se o resultado for menor que isso, retorna join simples.
    pub min_ratio: f32,
    
    /// Similaridade mínima para considerar duplicata (0.0 - 1.0).
    pub similarity_threshold: f32,
    
    /// Temperatura para geração do LLM.
    pub temperature: f32,
}

impl Default for ReducerConfig {
    fn default() -> Self {
        Self {
            min_ratio: 0.60,
            similarity_threshold: 0.85,
            temperature: 0.3,
        }
    }
}

/// Cluster de respostas similares.
#[derive(Debug, Clone)]
struct ResponseCluster {
    /// Índices das respostas neste cluster.
    indices: Vec<usize>,
    /// Resposta representativa do cluster.
    representative: String,
    /// Score médio de qualidade.
    quality_score: f32,
}

/// Redutor de respostas multi-agente.
///
/// Combina múltiplas respostas de diferentes agentes em uma única
/// resposta coerente, eliminando duplicatas e preservando insights únicos.
///
/// # Exemplo
/// ```rust,ignore
/// use deep_research::tools::ResponseReducer;
///
/// let reducer = ResponseReducer::new(llm_client);
/// let combined = reducer.reduce_answers(
///     &["resposta 1...", "resposta 2...", "resposta 3..."],
///     &mut tracker,
/// ).await?;
/// ```
pub struct ResponseReducer {
    /// Cliente LLM para geração de texto.
    llm_client: Arc<dyn LlmClient>,
    
    /// Configuração do redutor.
    config: ReducerConfig,
}

impl ResponseReducer {
    /// Cria um novo redutor com configuração padrão.
    ///
    /// # Argumentos
    /// * `llm_client` - Cliente LLM para geração de texto e embeddings.
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            llm_client,
            config: ReducerConfig::default(),
        }
    }
    
    /// Cria um novo redutor com configuração customizada.
    ///
    /// # Argumentos
    /// * `llm_client` - Cliente LLM para geração de texto e embeddings.
    /// * `config` - Configuração do redutor.
    pub fn with_config(llm_client: Arc<dyn LlmClient>, config: ReducerConfig) -> Self {
        Self { llm_client, config }
    }
    
    /// Reduz múltiplas respostas em uma única resposta combinada.
    ///
    /// # Argumentos
    /// * `answers` - Slice de respostas a serem combinadas.
    /// * `tracker` - Tracker de tokens para monitoramento.
    ///
    /// # Retorna
    /// String com a resposta combinada.
    pub async fn reduce_answers(
        &self,
        answers: &[String],
        tracker: &mut TokenTracker,
    ) -> Result<String, ReducerError> {
        // Filtrar respostas vazias
        let valid_answers: Vec<&String> = answers
            .iter()
            .filter(|a| !a.trim().is_empty())
            .collect();
        
        if valid_answers.is_empty() {
            return Err(ReducerError::NoValidAnswers);
        }
        
        // Se só há uma resposta, retorna ela
        if valid_answers.len() == 1 {
            return Ok(valid_answers[0].clone());
        }
        
        // Identificar clusters de conteúdo similar
        let clusters = self.identify_clusters(&valid_answers).await?;
        
        // Se não conseguiu identificar clusters distintos, faz join simples
        if clusters.is_empty() {
            return Ok(self.simple_join(&valid_answers));
        }
        
        // Combinar clusters em resposta final
        let result = self.combine_clusters(&clusters, &valid_answers);
        
        // Validação: se resultado é muito menor que join simples, usa join
        let simple_join = self.simple_join(&valid_answers);
        let ratio = result.len() as f32 / simple_join.len() as f32;
        
        if ratio < self.config.min_ratio {
            log::warn!(
                "Reduced response too short ({:.1}% of simple join), using simple join",
                ratio * 100.0
            );
            return Ok(simple_join);
        }
        
        // Atualiza tracker de tokens (estimativa)
        let input_tokens: u64 = valid_answers.iter().map(|a| (a.len() / 4) as u64).sum();
        tracker.add_tokens("reducer", input_tokens, (result.len() / 4) as u64);
        
        Ok(result)
    }
    
    /// Identifica clusters de respostas similares.
    async fn identify_clusters(
        &self,
        answers: &[&String],
    ) -> Result<Vec<ResponseCluster>, ReducerError> {
        // Gerar embeddings para cada resposta (truncado para economia)
        let truncated: Vec<String> = answers
            .iter()
            .map(|a| a.chars().take(2000).collect())
            .collect();
        
        // Por enquanto, usa heurística simples baseada em tamanho e palavras-chave
        // Em produção, usaria embeddings reais do LLM
        let mut clusters: Vec<ResponseCluster> = Vec::new();
        let mut assigned: Vec<bool> = vec![false; answers.len()];
        
        for i in 0..answers.len() {
            if assigned[i] {
                continue;
            }
            
            let mut cluster = ResponseCluster {
                indices: vec![i],
                representative: answers[i].clone(),
                quality_score: self.estimate_quality(answers[i]),
            };
            
            for j in (i + 1)..answers.len() {
                if assigned[j] {
                    continue;
                }
                
                // Similaridade simples baseada em palavras comuns
                let sim = self.simple_similarity(answers[i], answers[j]);
                
                if sim >= self.config.similarity_threshold {
                    cluster.indices.push(j);
                    assigned[j] = true;
                    
                    // Atualiza representante se nova resposta é melhor
                    let quality = self.estimate_quality(answers[j]);
                    if quality > cluster.quality_score {
                        cluster.representative = answers[j].clone();
                        cluster.quality_score = quality;
                    }
                }
            }
            
            assigned[i] = true;
            clusters.push(cluster);
        }
        
        Ok(clusters)
    }
    
    /// Combina clusters em uma resposta final.
    fn combine_clusters(&self, clusters: &[ResponseCluster], _answers: &[&String]) -> String {
        // Ordena clusters por qualidade
        let mut sorted_clusters = clusters.to_vec();
        sorted_clusters.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap());
        
        // Combina representantes dos clusters
        let mut parts: Vec<String> = Vec::new();
        
        for (i, cluster) in sorted_clusters.iter().enumerate() {
            if i == 0 {
                // Primeiro cluster é o principal
                parts.push(cluster.representative.clone());
            } else {
                // Outros clusters adicionam informações complementares
                let complement = self.extract_unique_info(&cluster.representative, &parts);
                if !complement.is_empty() {
                    parts.push(complement);
                }
            }
        }
        
        parts.join("\n\n")
    }
    
    /// Extrai informações únicas que não estão nas partes existentes.
    fn extract_unique_info(&self, new_content: &str, existing: &[String]) -> String {
        // Divide em parágrafos
        let paragraphs: Vec<&str> = new_content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();
        
        let existing_text = existing.join(" ");
        
        // Mantém parágrafos que contêm informações não presentes
        let unique: Vec<&str> = paragraphs
            .into_iter()
            .filter(|p| {
                // Verifica se pelo menos 50% das palavras são únicas
                let words: Vec<&str> = p.split_whitespace().collect();
                let unique_count = words
                    .iter()
                    .filter(|w| !existing_text.contains(*w))
                    .count();
                
                unique_count as f32 / words.len() as f32 > 0.5
            })
            .collect();
        
        unique.join("\n\n")
    }
    
    /// Join simples de respostas separadas por quebras de linha.
    fn simple_join(&self, answers: &[&String]) -> String {
        answers
            .iter()
            .map(|a| a.trim())
            .filter(|a| !a.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }
    
    /// Estima qualidade de uma resposta baseado em heurísticas.
    fn estimate_quality(&self, answer: &str) -> f32 {
        let mut score: f32 = 0.0;
        
        // Tamanho (respostas mais completas são geralmente melhores)
        let len = answer.len();
        if len > 500 {
            score += 0.2;
        }
        if len > 1000 {
            score += 0.1;
        }
        
        // Estrutura (parágrafos)
        let paragraphs = answer.split("\n\n").count();
        if paragraphs >= 3 {
            score += 0.2;
        }
        
        // Referências/citações
        if answer.contains("[^") || answer.contains("](http") {
            score += 0.3;
        }
        
        // Headers markdown
        if answer.contains("## ") || answer.contains("### ") {
            score += 0.1;
        }
        
        // Código
        if answer.contains("```") {
            score += 0.1;
        }
        
        score.min(1.0)
    }
    
    /// Calcula similaridade simples entre duas strings.
    fn simple_similarity(&self, a: &str, b: &str) -> f32 {
        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
        
        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }
        
        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();
        
        intersection as f32 / union as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
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
    }
    
    #[tokio::test]
    async fn test_reducer_single_answer() {
        let client = Arc::new(MockLlmClient);
        let reducer = ResponseReducer::new(client);
        let mut tracker = TokenTracker::new(Some(100000));
        
        let answers = vec!["This is the only answer".to_string()];
        let result = reducer.reduce_answers(&answers, &mut tracker).await.unwrap();
        
        assert_eq!(result, "This is the only answer");
    }
    
    #[tokio::test]
    async fn test_reducer_empty_answers() {
        let client = Arc::new(MockLlmClient);
        let reducer = ResponseReducer::new(client);
        let mut tracker = TokenTracker::new(Some(100000));
        
        let answers: Vec<String> = vec!["".to_string(), "   ".to_string()];
        let result = reducer.reduce_answers(&answers, &mut tracker).await;
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_quality_estimation() {
        let client = Arc::new(MockLlmClient);
        let reducer = ResponseReducer::new(client);
        
        let simple = "Short answer.";
        let complex = "## Header\n\nThis is a longer answer with multiple paragraphs.\n\n[^1] Reference here.\n\n```code```";
        
        let score_simple = reducer.estimate_quality(simple);
        let score_complex = reducer.estimate_quality(complex);
        
        assert!(score_complex > score_simple);
    }
    
    #[test]
    fn test_simple_similarity() {
        let client = Arc::new(MockLlmClient);
        let reducer = ResponseReducer::new(client);
        
        let a = "The quick brown fox jumps over the lazy dog";
        let b = "The quick brown fox jumps over the lazy cat";
        let c = "Completely different text with no common words here";
        
        let sim_ab = reducer.simple_similarity(a, b);
        let sim_ac = reducer.simple_similarity(a, c);
        
        assert!(sim_ab > sim_ac);
    }
}
