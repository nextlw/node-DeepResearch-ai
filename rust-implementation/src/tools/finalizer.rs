//! # Response Finalizer
//!
//! Este módulo implementa o polimento final de respostas, atuando como um
//! "editor sênior" que preserva a essência do conteúdo enquanto melhora
//! a estrutura e clareza.
//!
//! ## Baseado em
//! `src/tools/finalizer.ts` do projeto TypeScript original.
//!
//! ## Filosofia
//! - Preservar a "vibe" original da resposta
//! - Estrutura: fatos → argumentos → conclusão
//! - Linguagem natural, sem bullet points excessivos
//! - Se resultado < 85% do original, retorna original

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::llm::LlmClient;
use crate::types::{KnowledgeItem, Language};
use crate::utils::TokenTracker;

/// Erro que pode ocorrer durante a finalização de respostas.
#[derive(Debug, thiserror::Error)]
pub enum FinalizerError {
    /// Erro na comunicação com o LLM.
    #[error("LLM error: {0}")]
    LlmError(String),
    
    /// Resposta gerada é muito curta comparada ao original.
    #[error("Response too short: {ratio}% of original")]
    ResponseTooShort {
        /// Proporção da resposta em relação ao original.
        ratio: f32,
    },
    
    /// Erro de parse na resposta do LLM.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Configuração do finalizador de respostas.
#[derive(Debug, Clone)]
pub struct FinalizerConfig {
    /// Proporção mínima do resultado em relação ao original (0.0 - 1.0).
    /// Se o resultado for menor que isso, retorna o original.
    pub min_ratio: f32,
    
    /// Temperatura para geração do LLM.
    pub temperature: f32,
    
    /// Número máximo de tokens para a resposta.
    pub max_tokens: u32,
}

impl Default for FinalizerConfig {
    fn default() -> Self {
        Self {
            min_ratio: 0.85,
            temperature: 0.3,
            max_tokens: 4096,
        }
    }
}

/// Resposta estruturada do finalizador.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizedResponse {
    /// Texto polido da resposta.
    pub content: String,
    
    /// Raciocínio do editor sobre as mudanças.
    pub think: String,
    
    /// Se o original foi preservado (não atendeu critérios).
    pub preserved_original: bool,
}

/// Finalizador de respostas que atua como "editor sênior".
///
/// # Exemplo
/// ```rust,ignore
/// use deep_research::tools::ResponseFinalizer;
///
/// let finalizer = ResponseFinalizer::new(llm_client);
/// let polished = finalizer.finalize_answer(
///     "resposta bruta aqui...",
///     &knowledge_items,
///     &Language::Portuguese,
///     &mut tracker,
/// ).await?;
/// ```
pub struct ResponseFinalizer {
    /// Cliente LLM para geração de texto.
    llm_client: Arc<dyn LlmClient>,
    
    /// Configuração do finalizador.
    config: FinalizerConfig,
}

impl ResponseFinalizer {
    /// Cria um novo finalizador com configuração padrão.
    ///
    /// # Argumentos
    /// * `llm_client` - Cliente LLM para geração de texto.
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            llm_client,
            config: FinalizerConfig::default(),
        }
    }
    
    /// Cria um novo finalizador com configuração customizada.
    ///
    /// # Argumentos
    /// * `llm_client` - Cliente LLM para geração de texto.
    /// * `config` - Configuração do finalizador.
    pub fn with_config(llm_client: Arc<dyn LlmClient>, config: FinalizerConfig) -> Self {
        Self { llm_client, config }
    }
    
    /// Finaliza (polir) uma resposta como um editor sênior.
    ///
    /// # Argumentos
    /// * `md_content` - Conteúdo markdown da resposta bruta.
    /// * `knowledge_items` - Itens de conhecimento acumulados durante a pesquisa.
    /// * `language` - Idioma da resposta.
    /// * `tracker` - Tracker de tokens para monitoramento.
    ///
    /// # Retorna
    /// String com a resposta polida, ou o original se não atender critérios.
    pub async fn finalize_answer(
        &self,
        md_content: &str,
        knowledge_items: &[KnowledgeItem],
        language: &Language,
        tracker: &mut TokenTracker,
    ) -> Result<String, FinalizerError> {
        // Se o conteúdo é muito curto, não vale a pena polir
        if md_content.len() < 100 {
            return Ok(md_content.to_string());
        }
        
        let system_prompt = self.build_system_prompt(language);
        let user_prompt = self.build_user_prompt(md_content, knowledge_items);
        
        // Simula chamada ao LLM (em produção, usaria self.llm_client)
        // Por enquanto, retorna o original com pequenas melhorias
        let result = self.polish_content(md_content);
        
        // Validação: se resultado é muito menor que original, usa original
        let ratio = result.len() as f32 / md_content.len() as f32;
        if ratio < self.config.min_ratio {
            log::warn!(
                "Finalized response too short ({:.1}% of original), preserving original",
                ratio * 100.0
            );
            return Ok(md_content.to_string());
        }
        
        // Atualiza tracker de tokens (estimativa)
        tracker.add_tokens("finalizer", (md_content.len() / 4) as u64, (result.len() / 4) as u64);
        
        Ok(result)
    }
    
    /// Constrói o prompt do sistema para o editor.
    fn build_system_prompt(&self, language: &Language) -> String {
        let lang_instruction = match language {
            Language::Portuguese => "Responda em português brasileiro.",
            Language::Spanish => "Responde en español.",
            Language::German => "Antworten Sie auf Deutsch.",
            Language::French => "Répondez en français.",
            Language::Italian => "Rispondi in italiano.",
            Language::Japanese => "日本語で回答してください。",
            Language::Chinese => "请用中文回答。",
            Language::Korean => "한국어로 답변해 주세요.",
            _ => "Respond in English.",
        };
        
        format!(r#"You are a senior editor with multiple best-selling books. Your job is to polish the given response while preserving its original essence and voice.

## Guidelines

1. **Preserve the Vibe**: Keep the original tone and personality
2. **Structure**: Facts → Arguments → Conclusion
3. **Natural Language**: Avoid excessive bullet points, use flowing prose
4. **Clarity**: Make complex ideas accessible without dumbing them down
5. **Citations**: Preserve all references and citations exactly

## What NOT to do
- Don't add new information not in the original
- Don't remove important details
- Don't change the meaning or conclusions
- Don't add unnecessary fluff or filler

{}

Output the polished version directly, no explanations needed."#, lang_instruction)
    }
    
    /// Constrói o prompt do usuário com o conteúdo a ser polido.
    fn build_user_prompt(&self, md_content: &str, knowledge_items: &[KnowledgeItem]) -> String {
        let knowledge_context = if knowledge_items.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = knowledge_items
                .iter()
                .take(5) // Limita a 5 itens para não sobrecarregar
                .map(|k| format!("- {}: {}", k.question, k.answer))
                .collect();
            format!("\n\n<context>\n{}\n</context>", items.join("\n"))
        };
        
        format!(r#"Please polish the following response:

<response>
{}
</response>
{}"#, md_content, knowledge_context)
    }
    
    /// Polimento básico do conteúdo (fallback quando LLM não disponível).
    fn polish_content(&self, content: &str) -> String {
        let mut result = content.to_string();
        
        // Remove linhas em branco duplicadas
        while result.contains("\n\n\n") {
            result = result.replace("\n\n\n", "\n\n");
        }
        
        // Remove espaços em branco no final das linhas
        result = result
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        
        // Garante que termina com uma linha em branco
        if !result.ends_with('\n') {
            result.push('\n');
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    // Mock LLM client para testes
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
                answer: "Mock answer".into(),
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
    async fn test_finalizer_short_content() {
        let client = Arc::new(MockLlmClient);
        let finalizer = ResponseFinalizer::new(client);
        let mut tracker = TokenTracker::new(Some(100000));
        
        let result = finalizer
            .finalize_answer("Short", &[], &Language::English, &mut tracker)
            .await
            .unwrap();
        
        assert_eq!(result, "Short");
    }
    
    #[tokio::test]
    async fn test_finalizer_removes_extra_newlines() {
        let client = Arc::new(MockLlmClient);
        let finalizer = ResponseFinalizer::new(client);
        let mut tracker = TokenTracker::new(Some(100000));
        
        let content = "This is a test\n\n\n\nwith many\n\n\nnewlines that should be cleaned up properly in the final output.";
        let result = finalizer
            .finalize_answer(content, &[], &Language::English, &mut tracker)
            .await
            .unwrap();
        
        assert!(!result.contains("\n\n\n"));
    }
    
    #[test]
    fn test_config_defaults() {
        let config = FinalizerConfig::default();
        assert_eq!(config.min_ratio, 0.85);
        assert_eq!(config.temperature, 0.3);
    }
}
