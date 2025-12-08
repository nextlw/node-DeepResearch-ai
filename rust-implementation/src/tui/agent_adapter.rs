//! Adaptador para conectar eventos do agente à TUI

use std::sync::mpsc::Sender;

use super::app::{AppEvent, LogEntry, LogLevel};
use crate::agent::AgentAction;

/// Adaptador que converte eventos do agente para eventos da TUI
#[derive(Clone)]
pub struct AgentTuiAdapter {
    tx: Sender<AppEvent>,
}

impl AgentTuiAdapter {
    /// Cria novo adaptador
    pub fn new(tx: Sender<AppEvent>) -> Self {
        Self { tx }
    }

    /// Envia log de informação
    pub fn info(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::new(LogLevel::Info, msg)));
    }

    /// Envia log de sucesso
    pub fn success(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::new(LogLevel::Success, msg)));
    }

    /// Envia log de aviso
    pub fn warning(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::new(LogLevel::Warning, msg)));
    }

    /// Envia log de erro
    pub fn error(&self, msg: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Log(LogEntry::new(LogLevel::Error, msg)));
    }

    /// Atualiza step atual
    pub fn set_step(&self, step: usize) {
        let _ = self.tx.send(AppEvent::SetStep(step));
    }

    /// Atualiza ação atual (baseado no AgentAction)
    pub fn set_action(&self, action: &AgentAction) {
        let action_name = match action {
            AgentAction::Search { .. } => "🔍 Buscando",
            AgentAction::Read { .. } => "📖 Lendo URLs",
            AgentAction::Answer { .. } => "✍️ Gerando resposta",
            AgentAction::Reflect { .. } => "🤔 Refletindo",
        };
        let _ = self.tx.send(AppEvent::SetAction(action_name.to_string()));
    }

    /// Atualiza ação com texto customizado
    pub fn set_action_text(&self, text: impl Into<String>) {
        let _ = self.tx.send(AppEvent::SetAction(text.into()));
    }

    /// Atualiza o raciocínio atual
    pub fn set_think(&self, action: &AgentAction) {
        let think = action.think();
        let _ = self.tx.send(AppEvent::SetThink(think));
    }

    /// Atualiza contagem de URLs
    pub fn set_url_stats(&self, total: usize, visited: usize) {
        let _ = self.tx.send(AppEvent::SetUrlCount(total));
        let _ = self.tx.send(AppEvent::SetVisitedCount(visited));
    }

    /// Atualiza tokens utilizados
    pub fn set_tokens(&self, tokens: u64) {
        let _ = self.tx.send(AppEvent::SetTokens(tokens));
    }

    /// Log quando uma busca é feita
    pub fn on_search(&self, query: &str, result_count: usize) {
        self.info(format!("Busca: \"{}\" → {} URLs", query, result_count));
    }

    /// Log quando URLs são lidas
    pub fn on_read(&self, urls: &[String]) {
        for url in urls.iter().take(3) {
            // Trunca URL para exibição
            let display = if url.len() > 50 {
                format!("{}...", &url[..47])
            } else {
                url.clone()
            };
            self.info(format!("Lendo: {}", display));
        }
        if urls.len() > 3 {
            self.info(format!("... e mais {} URLs", urls.len() - 3));
        }
    }

    /// Log quando leitura de URL falha
    pub fn on_read_error(&self, url: &str, error: &str) {
        self.warning(format!("Erro ao ler {}: {}", url, error));
    }

    /// Log quando resposta é gerada
    pub fn on_answer(&self, answer: &str, ref_count: usize) {
        self.success(format!(
            "Resposta gerada ({} chars, {} refs)",
            answer.len(),
            ref_count
        ));
    }

    /// Marca pesquisa como completa
    pub fn complete(&self, answer: String, references: Vec<String>) {
        let _ = self.tx.send(AppEvent::SetAnswer(answer));
        let _ = self.tx.send(AppEvent::SetReferences(references));
        let _ = self.tx.send(AppEvent::Complete);
    }

    /// Marca como erro
    pub fn fail(&self, error: impl Into<String>) {
        let _ = self.tx.send(AppEvent::Error(error.into()));
    }
}
