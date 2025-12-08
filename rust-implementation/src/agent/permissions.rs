// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PERMISSÕES DE AÇÕES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::AgentContext;

/// Limites de ações por step
/// Máximo de reflexões permitidas por step
pub const MAX_REFLECT_PER_STEP: usize = 2;
/// Máximo de URLs antes de desabilitar busca
pub const MAX_URLS_BEFORE_DISABLE_SEARCH: usize = 50;
/// Máximo de URLs para mostrar nos resultados
pub const MAX_URLS_TO_SHOW: usize = 20;

/// Estado das permissões - imutável, criado a cada iteração
///
/// Este struct determina quais ações o agente pode tomar no passo atual.
/// As permissões são calculadas dinamicamente baseadas no contexto.
#[derive(Debug, Clone, Copy)]
pub struct ActionPermissions {
    /// Pode executar busca na web
    pub search: bool,
    /// Pode ler URLs
    pub read: bool,
    /// Pode gerar perguntas de reflexão
    pub reflect: bool,
    /// Pode fornecer resposta
    pub answer: bool,
    /// Pode executar código
    pub coding: bool,
}

impl ActionPermissions {
    /// Cria permissões baseadas no contexto atual
    ///
    /// # Regras:
    /// - `search`: Desabilitada se já tem 50+ URLs
    /// - `read`: Desabilitada se não há URLs disponíveis
    /// - `reflect`: Desabilitada se já tem muitas perguntas de gap
    /// - `answer`: Habilitada após min_steps (carregado do .env) ou se allow_direct_answer
    /// - `coding`: Sempre habilitada (por padrão)
    pub fn from_context(ctx: &AgentContext) -> Self {
        // Carregar configuração do agente do .env
        let agent_config = crate::config::load_agent_config();

        Self {
            search: ctx.collected_urls.len() < MAX_URLS_BEFORE_DISABLE_SEARCH,
            read: ctx.available_urls() > 0,
            reflect: ctx.gap_questions.len() <= MAX_REFLECT_PER_STEP,
            // ANSWER só é permitido após min_steps OU se allow_direct_answer está habilitado
            answer: ctx.total_step >= agent_config.min_steps_before_answer
                || (ctx.allow_direct_answer && agent_config.allow_direct_answer),
            coding: true, // Coding geralmente está habilitado
        }
    }

    /// Cria permissões baseadas no contexto e configuração específica
    pub fn from_context_with_config(ctx: &AgentContext, config: &crate::config::AgentConfig) -> Self {
        Self {
            search: ctx.collected_urls.len() < MAX_URLS_BEFORE_DISABLE_SEARCH,
            read: ctx.available_urls() > 0,
            reflect: ctx.gap_questions.len() <= MAX_REFLECT_PER_STEP,
            answer: ctx.total_step >= config.min_steps_before_answer
                || (ctx.allow_direct_answer && config.allow_direct_answer),
            coding: true,
        }
    }

    /// Cria permissões com tudo habilitado
    pub fn all_enabled() -> Self {
        Self {
            search: true,
            read: true,
            reflect: true,
            answer: true,
            coding: true,
        }
    }

    /// Cria permissões com tudo desabilitado
    pub fn all_disabled() -> Self {
        Self {
            search: false,
            read: false,
            reflect: false,
            answer: false,
            coding: false,
        }
    }

    /// Cria permissões para Beast Mode (apenas answer)
    pub fn beast_mode() -> Self {
        Self {
            search: false,
            read: false,
            reflect: false,
            answer: true,
            coding: false,
        }
    }

    /// Lista de ações permitidas (para logging/debug)
    pub fn allowed_actions(&self) -> Vec<&'static str> {
        let mut actions = Vec::with_capacity(5);
        if self.search {
            actions.push("search");
        }
        if self.read {
            actions.push("read");
        }
        if self.reflect {
            actions.push("reflect");
        }
        if self.answer {
            actions.push("answer");
        }
        if self.coding {
            actions.push("coding");
        }
        actions
    }

    /// Conta quantas ações estão permitidas
    pub fn count_allowed(&self) -> usize {
        [
            self.search,
            self.read,
            self.reflect,
            self.answer,
            self.coding,
        ]
        .iter()
        .filter(|&&x| x)
        .count()
    }

    /// Verifica se pelo menos uma ação está permitida
    pub fn has_any_allowed(&self) -> bool {
        self.search || self.read || self.reflect || self.answer || self.coding
    }

    /// Verifica se uma ação específica está permitida
    pub fn is_allowed(&self, action_name: &str) -> bool {
        match action_name {
            "search" => self.search,
            "read" => self.read,
            "reflect" => self.reflect,
            "answer" => self.answer,
            "coding" => self.coding,
            _ => false,
        }
    }

    /// Cria uma cópia com search desabilitado
    pub fn without_search(mut self) -> Self {
        self.search = false;
        self
    }

    /// Cria uma cópia com read desabilitado
    pub fn without_read(mut self) -> Self {
        self.read = false;
        self
    }

    /// Cria uma cópia com reflect desabilitado
    pub fn without_reflect(mut self) -> Self {
        self.reflect = false;
        self
    }

    /// Cria uma cópia com answer desabilitado
    pub fn without_answer(mut self) -> Self {
        self.answer = false;
        self
    }
}

impl Default for ActionPermissions {
    fn default() -> Self {
        Self::all_enabled()
    }
}

impl std::fmt::Display for ActionPermissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Permissions: [{}]", self.allowed_actions().join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_enabled() {
        let perms = ActionPermissions::all_enabled();
        assert!(perms.search);
        assert!(perms.read);
        assert!(perms.reflect);
        assert!(perms.answer);
        assert!(perms.coding);
        assert_eq!(perms.count_allowed(), 5);
    }

    #[test]
    fn test_all_disabled() {
        let perms = ActionPermissions::all_disabled();
        assert!(!perms.search);
        assert!(!perms.read);
        assert!(!perms.reflect);
        assert!(!perms.answer);
        assert!(!perms.coding);
        assert_eq!(perms.count_allowed(), 0);
    }

    #[test]
    fn test_beast_mode() {
        let perms = ActionPermissions::beast_mode();
        assert!(!perms.search);
        assert!(!perms.read);
        assert!(!perms.reflect);
        assert!(perms.answer);
        assert!(!perms.coding);
        assert_eq!(perms.count_allowed(), 1);
    }

    #[test]
    fn test_allowed_actions() {
        let perms = ActionPermissions {
            search: true,
            read: false,
            reflect: true,
            answer: false,
            coding: true,
        };
        let actions = perms.allowed_actions();
        assert_eq!(actions, vec!["search", "reflect", "coding"]);
    }

    #[test]
    fn test_without_methods() {
        let perms = ActionPermissions::all_enabled()
            .without_search()
            .without_read();

        assert!(!perms.search);
        assert!(!perms.read);
        assert!(perms.reflect);
        assert!(perms.answer);
        assert!(perms.coding);
    }

    #[test]
    fn test_is_allowed() {
        let perms = ActionPermissions {
            search: true,
            read: false,
            reflect: true,
            answer: true,
            coding: false,
        };

        assert!(perms.is_allowed("search"));
        assert!(!perms.is_allowed("read"));
        assert!(perms.is_allowed("reflect"));
        assert!(perms.is_allowed("answer"));
        assert!(!perms.is_allowed("coding"));
        assert!(!perms.is_allowed("unknown"));
    }
}
