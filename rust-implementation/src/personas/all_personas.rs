// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPLEMENTAÇÃO DAS 7 PERSONAS COGNITIVAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use chrono::Datelike;
use rand::seq::SliceRandom;

use super::{
    extract_main_topic, negate_assumption, translate_to_french, translate_to_german,
    translate_to_italian, translate_to_japanese, CognitivePersona, QueryContext,
};
use crate::types::{SerpQuery, TopicCategory};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 1. EXPERT SKEPTIC
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Expert Skeptic - Busca problemas, limitações, contra-evidências
///
/// Esta persona foca em encontrar o lado negativo das coisas:
/// - Problemas conhecidos
/// - Limitações técnicas
/// - Casos de falha
/// - Contra-argumentos
pub struct ExpertSkeptic;

impl CognitivePersona for ExpertSkeptic {
    fn name(&self) -> &'static str {
        "Expert Skeptic"
    }

    fn focus(&self) -> &'static str {
        "edge cases, limitations, counter-evidence, potential failures"
    }

    fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
        let skeptic_terms = ["problems", "issues", "failures", "limitations", "drawbacks"];
        let topic = extract_main_topic(original);

        let term = skeptic_terms
            .choose(&mut rand::thread_rng())
            .unwrap_or(&"problems");

        SerpQuery {
            q: format!("{} {} real experiences", topic, term),
            tbs: None,
            location: None,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 2. DETAIL ANALYST
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Detail Analyst - Especificações técnicas precisas
///
/// Esta persona foca em detalhes técnicos:
/// - Especificações exatas
/// - Parâmetros mensuráveis
/// - Dados de referência
/// - Comparações quantitativas
pub struct DetailAnalyst;

impl CognitivePersona for DetailAnalyst {
    fn name(&self) -> &'static str {
        "Detail Analyst"
    }

    fn focus(&self) -> &'static str {
        "precise specifications, technical details, exact parameters"
    }

    fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);

        SerpQuery {
            q: format!("{} specifications technical details comparison", topic),
            tbs: None,
            location: None,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 3. HISTORICAL RESEARCHER
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Historical Researcher - Evolução e contexto histórico
///
/// Esta persona foca na dimensão temporal:
/// - Como algo evoluiu ao longo do tempo
/// - Versões anteriores
/// - Contexto histórico
/// - Mudanças e tendências
pub struct HistoricalResearcher;

impl CognitivePersona for HistoricalResearcher {
    fn name(&self) -> &'static str {
        "Historical Researcher"
    }

    fn focus(&self) -> &'static str {
        "evolution over time, previous iterations, historical context"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);
        let year = ctx.current_date.year();

        SerpQuery {
            q: format!("{} history evolution {} changes", topic, year - 5),
            tbs: Some("qdr:y".into()), // Último ano
            location: None,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 4. COMPARATIVE THINKER
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Comparative Thinker - Alternativas e trade-offs
///
/// Esta persona foca em comparações:
/// - Alternativas disponíveis
/// - Trade-offs entre opções
/// - Prós e contras
/// - Análise competitiva
pub struct ComparativeThinker;

impl CognitivePersona for ComparativeThinker {
    fn name(&self) -> &'static str {
        "Comparative Thinker"
    }

    fn focus(&self) -> &'static str {
        "alternatives, competitors, contrasts, trade-offs"
    }

    fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);

        SerpQuery {
            q: format!("{} vs alternatives comparison pros cons", topic),
            tbs: None,
            location: None,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 5. TEMPORAL CONTEXT
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Temporal Context - Informações recentes com data atual
///
/// Esta persona foca em atualidade:
/// - Informações mais recentes
/// - Estado atual do tópico
/// - Últimas atualizações
/// - Tendências atuais
pub struct TemporalContext;

impl CognitivePersona for TemporalContext {
    fn name(&self) -> &'static str {
        "Temporal Context"
    }

    fn focus(&self) -> &'static str {
        "time-sensitive queries, recency, current state"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let topic = extract_main_topic(original);
        let year = ctx.current_date.year();
        let month = ctx.current_date.month();

        SerpQuery {
            q: format!("{} {} {}", topic, year, month),
            tbs: Some("qdr:m".into()), // Último mês
            location: None,
        }
    }

    fn weight(&self) -> f32 {
        1.2 // Peso maior para informações recentes
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 6. GLOBALIZER
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Globalizer - Fontes no idioma mais autoritativo
///
/// Esta persona busca informações na língua de origem:
/// - Carros alemães → busca em alemão
/// - Anime japonês → busca em japonês
/// - Culinária italiana → busca em italiano
/// - Tecnologia → busca em inglês (Silicon Valley)
pub struct Globalizer;

impl CognitivePersona for Globalizer {
    fn name(&self) -> &'static str {
        "Globalizer"
    }

    fn focus(&self) -> &'static str {
        "authoritative language/region for the subject matter"
    }

    fn expand_query(&self, original: &str, ctx: &QueryContext) -> SerpQuery {
        let (query, location) = match &ctx.detected_topic {
            TopicCategory::Automotive(brand) => match brand.as_str() {
                "BMW" | "Mercedes" | "Audi" | "Volkswagen" | "Porsche" => {
                    (translate_to_german(original), Some("Germany"))
                }
                "Toyota" | "Honda" | "Nissan" | "Mazda" | "Subaru" => {
                    (translate_to_japanese(original), Some("Japan"))
                }
                _ => (original.to_string(), None),
            },
            TopicCategory::Technology => (original.to_string(), Some("San Francisco")),
            TopicCategory::Cuisine(cuisine) => match cuisine.as_str() {
                "Italian" | "Pizza" | "Pasta" => (translate_to_italian(original), Some("Italy")),
                "French" | "Croissant" | "Wine" => (translate_to_french(original), Some("France")),
                "Japanese" | "Sushi" | "Ramen" => (translate_to_japanese(original), Some("Japan")),
                _ => (original.to_string(), None),
            },
            TopicCategory::Finance => (original.to_string(), Some("New York")),
            _ => (original.to_string(), None),
        };

        SerpQuery {
            q: query,
            tbs: None,
            location: location.map(String::from),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 7. REALITY SKEPTICALIST
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Reality Skepticalist - Contradições e evidências contrárias
///
/// Esta persona questiona tudo:
/// - Busca evidências contrárias
/// - Encontra mitos e suas refutações
/// - Descobre "verdades inconvenientes"
/// - Tenta provar o oposto
pub struct RealitySkepticalist;

impl CognitivePersona for RealitySkepticalist {
    fn name(&self) -> &'static str {
        "Reality Skepticalist"
    }

    fn focus(&self) -> &'static str {
        "contradicting evidence, disprove assumptions, contrary perspectives"
    }

    fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
        let negated = negate_assumption(original);

        SerpQuery {
            q: format!("{} wrong myth debunked evidence against", negated),
            tbs: None,
            location: None,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TIPOS EXPORTADOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn default_context() -> QueryContext {
        QueryContext {
            original_query: "test query".into(),
            user_intent: String::new(),
            soundbites: vec![],
            current_date: Utc::now().date_naive(),
            detected_language: crate::types::Language::English,
            detected_topic: TopicCategory::General,
        }
    }

    #[test]
    fn test_expert_skeptic() {
        let persona = ExpertSkeptic;
        let ctx = default_context();
        let query = persona.expand_query("rust programming", &ctx);

        assert!(query.q.contains("rust programming"));
        // Deve conter um termo cético
        assert!(
            query.q.contains("problems")
                || query.q.contains("issues")
                || query.q.contains("failures")
                || query.q.contains("limitations")
                || query.q.contains("drawbacks")
        );
    }

    #[test]
    fn test_detail_analyst() {
        let persona = DetailAnalyst;
        let ctx = default_context();
        let query = persona.expand_query("rust programming", &ctx);

        assert!(query.q.contains("specifications"));
        assert!(query.q.contains("technical"));
    }

    #[test]
    fn test_historical_researcher() {
        let persona = HistoricalResearcher;
        let ctx = default_context();
        let query = persona.expand_query("rust programming", &ctx);

        assert!(query.q.contains("history"));
        assert!(query.q.contains("evolution"));
        assert_eq!(query.tbs, Some("qdr:y".into()));
    }

    #[test]
    fn test_comparative_thinker() {
        let persona = ComparativeThinker;
        let ctx = default_context();
        let query = persona.expand_query("rust programming", &ctx);

        assert!(query.q.contains("vs"));
        assert!(query.q.contains("alternatives"));
    }

    #[test]
    fn test_temporal_context() {
        let persona = TemporalContext;
        let ctx = default_context();
        let query = persona.expand_query("rust programming", &ctx);

        let year = ctx.current_date.year();
        assert!(query.q.contains(&year.to_string()));
        assert_eq!(query.tbs, Some("qdr:m".into()));
        assert_eq!(persona.weight(), 1.2);
    }

    #[test]
    fn test_globalizer_german_cars() {
        let persona = Globalizer;
        let mut ctx = default_context();
        ctx.detected_topic = TopicCategory::Automotive("BMW".into());

        let query = persona.expand_query("BMW problems", &ctx);

        assert_eq!(query.location, Some("Germany".into()));
    }

    #[test]
    fn test_reality_skepticalist() {
        let persona = RealitySkepticalist;
        let ctx = default_context();
        let query = persona.expand_query("best programming language", &ctx);

        assert!(
            query.q.contains("worst") || query.q.contains("debunked") || query.q.contains("wrong")
        );
    }
}
