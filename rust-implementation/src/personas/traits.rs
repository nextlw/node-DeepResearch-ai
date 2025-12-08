// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TRAIT DE PERSONA COGNITIVA
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::QueryContext;
use crate::types::SerpQuery;

/// Trait que define o comportamento de uma persona cognitiva
///
/// Cada persona representa uma perspectiva diferente para expandir queries:
/// - Expert Skeptic: Busca problemas e limitações
/// - Detail Analyst: Foca em especificações técnicas
/// - Historical Researcher: Analisa evolução temporal
/// - Comparative Thinker: Compara alternativas
/// - Temporal Context: Adiciona contexto temporal atual
/// - Globalizer: Busca fontes no idioma autoritativo
/// - Reality Skepticalist: Busca contradições
///
/// # Requisitos de Thread Safety
///
/// O trait requer `Send + Sync` porque as personas são usadas em
/// paralelo com Rayon. Isso garante que múltiplas threads podem
/// executar `expand_query` simultaneamente.
///
/// # Exemplo de Implementação
///
/// ```rust
/// pub struct MyPersona;
///
/// impl CognitivePersona for MyPersona {
///     fn name(&self) -> &'static str { "My Persona" }
///     fn focus(&self) -> &'static str { "custom focus area" }
///
///     fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
///         SerpQuery {
///             q: format!("{} custom expansion", original),
///             tbs: None,
///             location: None,
///         }
///     }
/// }
/// ```
pub trait CognitivePersona: Send + Sync {
    /// Nome da persona para logging e debugging
    ///
    /// Retorna uma string estática identificando a persona.
    fn name(&self) -> &'static str;

    /// Descrição do foco desta persona
    ///
    /// Explica que tipo de informação esta persona busca.
    fn focus(&self) -> &'static str;

    /// Gera uma query expandida a partir da query original
    ///
    /// # Argumentos
    ///
    /// * `original` - Query original do usuário
    /// * `context` - Contexto compartilhado com informações adicionais
    ///
    /// # Retorno
    ///
    /// Uma `SerpQuery` com a query expandida, opcionalmente com
    /// filtros de tempo (`tbs`) e localização (`location`).
    fn expand_query(&self, original: &str, context: &QueryContext) -> SerpQuery;

    /// Peso desta persona no ranking final (0.0 - 2.0)
    ///
    /// Personas com peso maior têm suas queries priorizadas.
    /// O default é 1.0 (peso neutro).
    fn weight(&self) -> f32 {
        1.0
    }

    /// Verifica se esta persona é aplicável ao contexto
    ///
    /// Algumas personas podem ser desabilitadas para certos tipos
    /// de queries ou tópicos.
    fn is_applicable(&self, _context: &QueryContext) -> bool {
        true
    }

    /// Descrição curta para uso em prompts
    fn prompt_description(&self) -> String {
        format!("{}: {}", self.name(), self.focus())
    }
}

/// Wrapper para permitir `Box<dyn CognitivePersona>` ser usado com Rayon
///
/// Este wrapper é necessário porque `Box<dyn Trait>` não implementa
/// automaticamente `Send + Sync` mesmo quando o trait os requer.
pub struct PersonaBox(pub Box<dyn CognitivePersona>);

impl PersonaBox {
    /// Cria um novo PersonaBox com uma persona específica
    pub fn new<P: CognitivePersona + 'static>(persona: P) -> Self {
        Self(Box::new(persona))
    }
}

// Safety: CognitivePersona requer Send + Sync, então PersonaBox também é thread-safe
unsafe impl Send for PersonaBox {}
unsafe impl Sync for PersonaBox {}

impl std::ops::Deref for PersonaBox {
    type Target = dyn CognitivePersona;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPersona;

    impl CognitivePersona for TestPersona {
        fn name(&self) -> &'static str {
            "Test"
        }
        fn focus(&self) -> &'static str {
            "testing"
        }

        fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
            SerpQuery {
                q: format!("{} test", original),
                tbs: None,
                location: None,
            }
        }

        fn weight(&self) -> f32 {
            1.5
        }
    }

    #[test]
    fn test_persona_implementation() {
        let persona = TestPersona;
        assert_eq!(persona.name(), "Test");
        assert_eq!(persona.focus(), "testing");
        assert_eq!(persona.weight(), 1.5);
    }

    #[test]
    fn test_expand_query() {
        let persona = TestPersona;
        let ctx = QueryContext::default();
        let query = persona.expand_query("rust programming", &ctx);
        assert_eq!(query.q, "rust programming test");
    }

    #[test]
    fn test_prompt_description() {
        let persona = TestPersona;
        assert_eq!(persona.prompt_description(), "Test: testing");
    }

    #[test]
    fn test_is_applicable_default() {
        let persona = TestPersona;
        let ctx = QueryContext::default();
        assert!(persona.is_applicable(&ctx));
    }

    #[test]
    fn test_persona_box() {
        let boxed = PersonaBox::new(TestPersona);
        assert_eq!(boxed.name(), "Test");
    }
}
