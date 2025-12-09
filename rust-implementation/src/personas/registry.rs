// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// REGISTRO DINÂMICO DE PERSONAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Sistema para registrar, listar e gerenciar personas em runtime.
// Permite adicionar novas personas sem recompilar o código.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;
use std::path::Path;
use std::fs;

use serde::{Deserialize, Serialize};

use super::{
    PersonaBox, CognitivePersona, QueryContext, WeightedQuery,
    ExpertSkeptic, DetailAnalyst, HistoricalResearcher,
    ComparativeThinker, TemporalContext, Globalizer, RealitySkepticalist,
};

/// Schema de configuração de uma persona (para JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaSchema {
    /// Nome único da persona
    pub name: String,
    /// Descrição do foco/especialidade
    pub focus: String,
    /// Peso no ranking (0.0 - 2.0)
    pub weight: f32,
    /// Se está habilitada
    pub enabled: bool,
    /// Sufixo a adicionar nas queries (para personas baseadas em padrão)
    #[serde(default)]
    pub query_suffix: Option<String>,
    /// Filtro de tempo (tbs) padrão
    #[serde(default)]
    pub default_tbs: Option<String>,
    /// Localização padrão
    #[serde(default)]
    pub default_location: Option<String>,
}

impl Default for PersonaSchema {
    fn default() -> Self {
        Self {
            name: "Unknown".into(),
            focus: "general".into(),
            weight: 1.0,
            enabled: true,
            query_suffix: None,
            default_tbs: None,
            default_location: None,
        }
    }
}

/// Configuração completa do registry (para carregar de JSON)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryConfig {
    /// Versão do schema de configuração
    #[serde(default = "default_version")]
    pub version: String,
    /// Lista de personas configuradas
    #[serde(default)]
    pub personas: Vec<PersonaSchema>,
}

fn default_version() -> String {
    "1.0".into()
}

/// Erro ao operar com o registry
#[derive(Debug, Clone)]
pub enum RegistryError {
    /// Persona já existe com esse nome
    AlreadyExists(String),
    /// Persona não encontrada
    NotFound(String),
    /// Erro ao carregar configuração
    ConfigError(String),
    /// Validação falhou
    ValidationError(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists(name) => write!(f, "Persona '{}' already exists", name),
            Self::NotFound(name) => write!(f, "Persona '{}' not found", name),
            Self::ConfigError(msg) => write!(f, "Config error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Registry central de personas
///
/// Permite registrar, remover e listar personas em runtime.
/// Suporta carregar configuração de arquivo JSON.
///
/// # Exemplo
///
/// ```rust,ignore
/// let mut registry = PersonaRegistry::new();
/// registry.register_default_personas();
///
/// // Listar todas
/// for name in registry.list_available() {
///     println!("Persona: {}", name);
/// }
///
/// // Obter uma específica
/// if let Some(persona) = registry.get("Expert Skeptic") {
///     let query = persona.expand_query("rust programming", &ctx);
/// }
/// ```
pub struct PersonaRegistry {
    /// Personas registradas (nome -> implementação)
    personas: HashMap<String, PersonaBox>,
    /// Schemas de configuração (nome -> schema)
    schemas: HashMap<String, PersonaSchema>,
}

impl PersonaRegistry {
    /// Cria um registry vazio
    pub fn new() -> Self {
        Self {
            personas: HashMap::new(),
            schemas: HashMap::new(),
        }
    }

    /// Cria um registry com as 7 personas padrão já registradas
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_default_personas();
        registry
    }

    /// Registra as 7 personas cognitivas padrão
    pub fn register_default_personas(&mut self) {
        self.register_builtin("Expert Skeptic", PersonaBox::new(ExpertSkeptic), PersonaSchema {
            name: "Expert Skeptic".into(),
            focus: "problems, limitations, counter-evidence".into(),
            weight: 1.0,
            enabled: true,
            query_suffix: Some("problems issues failures".into()),
            ..Default::default()
        });

        self.register_builtin("Detail Analyst", PersonaBox::new(DetailAnalyst), PersonaSchema {
            name: "Detail Analyst".into(),
            focus: "specifications, technical details, exact parameters".into(),
            weight: 1.0,
            enabled: true,
            query_suffix: Some("specifications technical details".into()),
            ..Default::default()
        });

        self.register_builtin("Historical Researcher", PersonaBox::new(HistoricalResearcher), PersonaSchema {
            name: "Historical Researcher".into(),
            focus: "evolution over time, historical context".into(),
            weight: 1.0,
            enabled: true,
            query_suffix: Some("history evolution changes".into()),
            default_tbs: Some("qdr:y".into()),
            ..Default::default()
        });

        self.register_builtin("Comparative Thinker", PersonaBox::new(ComparativeThinker), PersonaSchema {
            name: "Comparative Thinker".into(),
            focus: "alternatives, competitors, trade-offs".into(),
            weight: 1.0,
            enabled: true,
            query_suffix: Some("vs alternatives comparison".into()),
            ..Default::default()
        });

        self.register_builtin("Temporal Context", PersonaBox::new(TemporalContext), PersonaSchema {
            name: "Temporal Context".into(),
            focus: "time-sensitive queries, recency".into(),
            weight: 1.2, // Peso maior para informações recentes
            enabled: true,
            default_tbs: Some("qdr:m".into()),
            ..Default::default()
        });

        self.register_builtin("Globalizer", PersonaBox::new(Globalizer), PersonaSchema {
            name: "Globalizer".into(),
            focus: "authoritative language/region for subject".into(),
            weight: 1.0,
            enabled: true,
            ..Default::default()
        });

        self.register_builtin("Reality Skepticalist", PersonaBox::new(RealitySkepticalist), PersonaSchema {
            name: "Reality Skepticalist".into(),
            focus: "contradicting evidence, disprove assumptions".into(),
            weight: 1.0,
            enabled: true,
            query_suffix: Some("wrong myth debunked".into()),
            ..Default::default()
        });
    }

    /// Registra uma persona builtin (interna)
    fn register_builtin(&mut self, name: &str, persona: PersonaBox, schema: PersonaSchema) {
        self.personas.insert(name.to_string(), persona);
        self.schemas.insert(name.to_string(), schema);
    }

    /// Registra uma nova persona
    ///
    /// # Erros
    /// - `AlreadyExists` se já existe persona com esse nome
    pub fn register<P: CognitivePersona + 'static>(
        &mut self,
        persona: P,
    ) -> Result<(), RegistryError> {
        let name = persona.name().to_string();
        
        if self.personas.contains_key(&name) {
            return Err(RegistryError::AlreadyExists(name));
        }

        let schema = PersonaSchema {
            name: name.clone(),
            focus: persona.focus().to_string(),
            weight: persona.weight(),
            enabled: true,
            ..Default::default()
        };

        self.personas.insert(name.clone(), PersonaBox::new(persona));
        self.schemas.insert(name, schema);
        
        Ok(())
    }

    /// Remove uma persona do registry
    ///
    /// # Erros
    /// - `NotFound` se a persona não existe
    pub fn unregister(&mut self, name: &str) -> Result<(), RegistryError> {
        if !self.personas.contains_key(name) {
            return Err(RegistryError::NotFound(name.to_string()));
        }

        self.personas.remove(name);
        self.schemas.remove(name);
        
        Ok(())
    }

    /// Lista nomes de todas as personas disponíveis
    pub fn list_available(&self) -> Vec<&str> {
        self.personas.keys().map(|s| s.as_str()).collect()
    }

    /// Lista nomes de personas habilitadas
    pub fn list_enabled(&self) -> Vec<&str> {
        self.schemas
            .iter()
            .filter(|(_, schema)| schema.enabled)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Obtém uma persona pelo nome
    pub fn get(&self, name: &str) -> Option<&PersonaBox> {
        self.personas.get(name)
    }

    /// Obtém o schema de uma persona
    pub fn get_schema(&self, name: &str) -> Option<&PersonaSchema> {
        self.schemas.get(name)
    }

    /// Habilita uma persona
    pub fn enable(&mut self, name: &str) -> Result<(), RegistryError> {
        self.schemas
            .get_mut(name)
            .map(|s| s.enabled = true)
            .ok_or_else(|| RegistryError::NotFound(name.to_string()))
    }

    /// Desabilita uma persona
    pub fn disable(&mut self, name: &str) -> Result<(), RegistryError> {
        self.schemas
            .get_mut(name)
            .map(|s| s.enabled = false)
            .ok_or_else(|| RegistryError::NotFound(name.to_string()))
    }

    /// Retorna número total de personas registradas
    pub fn count(&self) -> usize {
        self.personas.len()
    }

    /// Retorna número de personas habilitadas
    pub fn count_enabled(&self) -> usize {
        self.schemas.values().filter(|s| s.enabled).count()
    }

    /// Verifica se uma persona existe
    pub fn contains(&self, name: &str) -> bool {
        self.personas.contains_key(name)
    }

    /// Carrega configuração de um arquivo JSON
    ///
    /// O arquivo deve seguir o formato `RegistryConfig`.
    /// Personas no JSON atualizam os schemas das personas existentes.
    pub fn load_config(&mut self, path: impl AsRef<Path>) -> Result<(), RegistryError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| RegistryError::ConfigError(format!("Failed to read file: {}", e)))?;

        let config: RegistryConfig = serde_json::from_str(&content)
            .map_err(|e| RegistryError::ConfigError(format!("Failed to parse JSON: {}", e)))?;

        // Atualiza schemas das personas existentes
        for persona_config in config.personas {
            if let Some(schema) = self.schemas.get_mut(&persona_config.name) {
                schema.enabled = persona_config.enabled;
                schema.weight = persona_config.weight;
                if persona_config.query_suffix.is_some() {
                    schema.query_suffix = persona_config.query_suffix;
                }
                if persona_config.default_tbs.is_some() {
                    schema.default_tbs = persona_config.default_tbs;
                }
                if persona_config.default_location.is_some() {
                    schema.default_location = persona_config.default_location;
                }
            }
        }

        Ok(())
    }

    /// Salva configuração atual em um arquivo JSON
    pub fn save_config(&self, path: impl AsRef<Path>) -> Result<(), RegistryError> {
        let config = RegistryConfig {
            version: "1.0".into(),
            personas: self.schemas.values().cloned().collect(),
        };

        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| RegistryError::ConfigError(format!("Failed to serialize: {}", e)))?;

        fs::write(path.as_ref(), content)
            .map_err(|e| RegistryError::ConfigError(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    /// Expande uma query usando todas as personas habilitadas
    pub fn expand_query_all(&self, original: &str, context: &QueryContext) -> Vec<WeightedQuery> {
        self.personas
            .iter()
            .filter(|(name, _)| {
                self.schemas
                    .get(*name)
                    .map(|s| s.enabled)
                    .unwrap_or(false)
            })
            .filter(|(_, persona)| persona.is_applicable(context))
            .map(|(_, persona)| {
                let query = persona.expand_query(original, context);
                WeightedQuery {
                    query,
                    weight: persona.weight(),
                    source_persona: persona.name(),
                }
            })
            .collect()
    }

    /// Retorna iterador sobre todas as personas
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PersonaBox)> {
        self.personas.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Retorna iterador sobre personas habilitadas
    pub fn iter_enabled(&self) -> impl Iterator<Item = (&str, &PersonaBox)> {
        self.personas
            .iter()
            .filter(|(name, _)| {
                self.schemas
                    .get(*name)
                    .map(|s| s.enabled)
                    .unwrap_or(false)
            })
            .map(|(k, v)| (k.as_str(), v))
    }
}

impl Default for PersonaRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SerpQuery;

    #[test]
    fn test_new_registry_empty() {
        let registry = PersonaRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_with_defaults() {
        let registry = PersonaRegistry::with_defaults();
        assert_eq!(registry.count(), 7);
        assert!(registry.contains("Expert Skeptic"));
        assert!(registry.contains("Globalizer"));
    }

    #[test]
    fn test_list_available() {
        let registry = PersonaRegistry::with_defaults();
        let names = registry.list_available();
        assert_eq!(names.len(), 7);
        assert!(names.contains(&"Expert Skeptic"));
    }

    #[test]
    fn test_get_persona() {
        let registry = PersonaRegistry::with_defaults();
        
        let persona = registry.get("Expert Skeptic");
        assert!(persona.is_some());
        assert_eq!(persona.unwrap().name(), "Expert Skeptic");

        let not_found = registry.get("NonExistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_schema() {
        let registry = PersonaRegistry::with_defaults();
        
        let schema = registry.get_schema("Temporal Context");
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().weight, 1.2);
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = PersonaRegistry::with_defaults();
        
        assert_eq!(registry.count_enabled(), 7);
        
        registry.disable("Expert Skeptic").unwrap();
        assert_eq!(registry.count_enabled(), 6);
        
        registry.enable("Expert Skeptic").unwrap();
        assert_eq!(registry.count_enabled(), 7);
    }

    #[test]
    fn test_unregister() {
        let mut registry = PersonaRegistry::with_defaults();
        assert_eq!(registry.count(), 7);
        
        registry.unregister("Expert Skeptic").unwrap();
        assert_eq!(registry.count(), 6);
        assert!(!registry.contains("Expert Skeptic"));
    }

    #[test]
    fn test_unregister_not_found() {
        let mut registry = PersonaRegistry::with_defaults();
        let result = registry.unregister("NonExistent");
        assert!(matches!(result, Err(RegistryError::NotFound(_))));
    }

    #[test]
    fn test_register_custom_persona() {
        struct CustomPersona;
        
        impl CognitivePersona for CustomPersona {
            fn name(&self) -> &'static str { "Custom Test" }
            fn focus(&self) -> &'static str { "testing custom registration" }
            fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
                SerpQuery {
                    q: format!("{} custom", original),
                    tbs: None,
                    location: None,
                }
            }
        }

        let mut registry = PersonaRegistry::new();
        registry.register(CustomPersona).unwrap();
        
        assert_eq!(registry.count(), 1);
        assert!(registry.contains("Custom Test"));
    }

    #[test]
    fn test_register_duplicate() {
        let mut registry = PersonaRegistry::with_defaults();
        
        struct DuplicatePersona;
        impl CognitivePersona for DuplicatePersona {
            fn name(&self) -> &'static str { "Expert Skeptic" } // Já existe!
            fn focus(&self) -> &'static str { "duplicate" }
            fn expand_query(&self, original: &str, _ctx: &QueryContext) -> SerpQuery {
                SerpQuery { q: original.into(), tbs: None, location: None }
            }
        }

        let result = registry.register(DuplicatePersona);
        assert!(matches!(result, Err(RegistryError::AlreadyExists(_))));
    }

    #[test]
    fn test_expand_query_all() {
        let registry = PersonaRegistry::with_defaults();
        let ctx = QueryContext::default();
        
        let queries = registry.expand_query_all("rust programming", &ctx);
        
        assert_eq!(queries.len(), 7);
        for wq in &queries {
            assert!(!wq.query.q.is_empty());
            assert!(wq.weight > 0.0);
        }
    }

    #[test]
    fn test_expand_query_with_disabled() {
        let mut registry = PersonaRegistry::with_defaults();
        registry.disable("Expert Skeptic").unwrap();
        registry.disable("Globalizer").unwrap();
        
        let ctx = QueryContext::default();
        let queries = registry.expand_query_all("rust programming", &ctx);
        
        assert_eq!(queries.len(), 5); // 7 - 2 desabilitadas
    }

    #[test]
    fn test_iter() {
        let registry = PersonaRegistry::with_defaults();
        
        let count = registry.iter().count();
        assert_eq!(count, 7);
    }

    #[test]
    fn test_iter_enabled() {
        let mut registry = PersonaRegistry::with_defaults();
        registry.disable("Expert Skeptic").unwrap();
        
        let count = registry.iter_enabled().count();
        assert_eq!(count, 6);
    }

    #[test]
    fn test_schema_serialization() {
        let schema = PersonaSchema {
            name: "Test".into(),
            focus: "testing".into(),
            weight: 1.5,
            enabled: true,
            query_suffix: Some("test suffix".into()),
            default_tbs: None,
            default_location: None,
        };

        let json = serde_json::to_string(&schema).unwrap();
        let deserialized: PersonaSchema = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.name, "Test");
        assert_eq!(deserialized.weight, 1.5);
    }

    #[test]
    fn test_registry_config_serialization() {
        let config = RegistryConfig {
            version: "1.0".into(),
            personas: vec![
                PersonaSchema {
                    name: "Test1".into(),
                    focus: "focus1".into(),
                    weight: 1.0,
                    enabled: true,
                    ..Default::default()
                },
                PersonaSchema {
                    name: "Test2".into(),
                    focus: "focus2".into(),
                    weight: 1.5,
                    enabled: false,
                    ..Default::default()
                },
            ],
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: RegistryConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.version, "1.0");
        assert_eq!(deserialized.personas.len(), 2);
    }
}

