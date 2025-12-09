//! # Evidências de Avaliação
//!
//! Estruturas para coletar e reportar evidências de operações de avaliação,
//! incluindo detalhes de cada avaliação executada e suas métricas.

use crate::evaluation::EvaluationType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::EvidenceReport;

/// Evidência de uma avaliação individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationEvidence {
    /// ID único da avaliação
    pub eval_id: Uuid,
    /// Tipo de avaliação
    pub eval_type: EvaluationType,
    /// Se o prompt foi gerado com sucesso
    pub prompt_generated: bool,
    /// Tamanho do prompt gerado (chars)
    pub prompt_length: usize,
    /// Se o LLM foi chamado
    pub llm_called: bool,
    /// Latência da chamada ao LLM
    pub llm_latency: Duration,
    /// Tokens usados pelo LLM
    pub llm_tokens_used: u32,
    /// Se a resposta passou nesta avaliação
    pub result_passed: bool,
    /// Nível de confiança do resultado
    pub result_confidence: f32,
    /// Tamanho do raciocínio/reasoning
    pub reasoning_length: usize,
    /// Número de sugestões geradas
    pub suggestions_count: usize,
    /// Erro ocorrido (se houver)
    pub error: Option<String>,
}

impl EvaluationEvidence {
    /// Cria uma nova evidência de avaliação
    pub fn new(eval_type: EvaluationType) -> Self {
        Self {
            eval_id: Uuid::new_v4(),
            eval_type,
            prompt_generated: false,
            prompt_length: 0,
            llm_called: false,
            llm_latency: Duration::ZERO,
            llm_tokens_used: 0,
            result_passed: false,
            result_confidence: 0.0,
            reasoning_length: 0,
            suggestions_count: 0,
            error: None,
        }
    }
    
    /// Registra que o prompt foi gerado
    pub fn prompt_generated(&mut self, length: usize) {
        self.prompt_generated = true;
        self.prompt_length = length;
    }
    
    /// Registra chamada ao LLM
    pub fn llm_called(&mut self, latency: Duration, tokens: u32) {
        self.llm_called = true;
        self.llm_latency = latency;
        self.llm_tokens_used = tokens;
    }
    
    /// Registra o resultado da avaliação
    pub fn set_result(&mut self, passed: bool, confidence: f32, reasoning_length: usize, suggestions: usize) {
        self.result_passed = passed;
        self.result_confidence = confidence;
        self.reasoning_length = reasoning_length;
        self.suggestions_count = suggestions;
    }
    
    /// Registra erro
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }
    
    /// Verifica se a avaliação foi bem sucedida (executou sem erros)
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
    
    /// Gera resumo textual
    pub fn summary(&self) -> String {
        let status = if self.result_passed { "PASS" } else { "FAIL" };
        format!(
            "{}: {} (confidence={:.2}, tokens={}, latency={:?})",
            self.eval_type.as_str(),
            status,
            self.result_confidence,
            self.llm_tokens_used,
            self.llm_latency
        )
    }
}

/// Relatório completo de evidências de avaliação para uma execução
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationEvidenceReport {
    /// ID da execução
    pub execution_id: Uuid,
    /// Timestamp de criação
    pub timestamp: DateTime<Utc>,
    /// Pergunta que foi avaliada
    pub question: String,
    /// Tamanho da resposta avaliada
    pub answer_length: usize,
    /// Tipos de avaliação que foram determinados como necessários
    pub evaluations_required: Vec<EvaluationType>,
    /// Evidências de cada avaliação executada
    pub evaluations_executed: Vec<EvaluationEvidence>,
    /// Veredicto final (todas passaram?)
    pub final_verdict: bool,
    /// Tempo total de avaliação
    pub total_evaluation_time: Duration,
    /// Total de tokens LLM usados
    pub total_llm_tokens: u32,
    /// Razão para early-fail (se aplicável)
    pub early_fail_reason: Option<String>,
}

impl Default for EvaluationEvidenceReport {
    fn default() -> Self {
        Self::new(Uuid::new_v4(), "", 0)
    }
}

impl EvaluationEvidenceReport {
    /// Cria um novo relatório de evidências de avaliação
    pub fn new(execution_id: Uuid, question: impl Into<String>, answer_length: usize) -> Self {
        Self {
            execution_id,
            timestamp: Utc::now(),
            question: question.into(),
            answer_length,
            evaluations_required: vec![],
            evaluations_executed: vec![],
            final_verdict: false,
            total_evaluation_time: Duration::ZERO,
            total_llm_tokens: 0,
            early_fail_reason: None,
        }
    }
    
    /// Define os tipos de avaliação requeridos
    pub fn set_required_evaluations(&mut self, types: Vec<EvaluationType>) {
        self.evaluations_required = types;
    }
    
    /// Adiciona uma evidência de avaliação
    pub fn add_evaluation(&mut self, evidence: EvaluationEvidence) {
        self.total_llm_tokens += evidence.llm_tokens_used;
        self.total_evaluation_time += evidence.llm_latency;
        
        // Se falhou, registra early-fail
        if !evidence.result_passed && self.early_fail_reason.is_none() {
            self.early_fail_reason = Some(format!(
                "Failed at {} evaluation",
                evidence.eval_type.as_str()
            ));
        }
        
        self.evaluations_executed.push(evidence);
    }
    
    /// Finaliza o relatório
    pub fn finalize(&mut self) {
        // Veredicto final: todas as avaliações passaram
        self.final_verdict = self.evaluations_executed.iter()
            .all(|e| e.result_passed);
    }
    
    /// Retorna taxa de sucesso das avaliações
    pub fn success_rate(&self) -> f32 {
        let total = self.evaluations_executed.len();
        if total == 0 {
            return 0.0;
        }
        
        let passed = self.evaluations_executed.iter()
            .filter(|e| e.result_passed)
            .count();
        
        passed as f32 / total as f32
    }
    
    /// Retorna média de confiança
    pub fn avg_confidence(&self) -> f32 {
        let total = self.evaluations_executed.len();
        if total == 0 {
            return 0.0;
        }
        
        let sum: f32 = self.evaluations_executed.iter()
            .map(|e| e.result_confidence)
            .sum();
        
        sum / total as f32
    }
    
    /// Retorna avaliações que falharam
    pub fn failed_evaluations(&self) -> Vec<&EvaluationEvidence> {
        self.evaluations_executed.iter()
            .filter(|e| !e.result_passed)
            .collect()
    }
    
    /// Retorna todas as sugestões
    pub fn all_suggestions(&self) -> usize {
        self.evaluations_executed.iter()
            .map(|e| e.suggestions_count)
            .sum()
    }
    
    /// Gera resumo textual
    pub fn summary_text(&self) -> String {
        let verdict = if self.final_verdict { "APPROVED" } else { "REJECTED" };
        
        let mut lines = vec![
            format!("EvaluationEvidenceReport [{}]", self.execution_id),
            format!("Question: {}...", &self.question.chars().take(50).collect::<String>()),
            format!("Answer Length: {} chars", self.answer_length),
            format!("Required: {:?}", self.evaluations_required.iter().map(|t| t.as_str()).collect::<Vec<_>>()),
            format!("Executed: {} evaluations", self.evaluations_executed.len()),
            format!("Verdict: {}", verdict),
            format!("Total Tokens: {}", self.total_llm_tokens),
            format!("Total Time: {:?}", self.total_evaluation_time),
        ];
        
        if let Some(reason) = &self.early_fail_reason {
            lines.push(format!("Early Fail: {}", reason));
        }
        
        lines.push("\nEvaluations:".to_string());
        for evidence in &self.evaluations_executed {
            lines.push(format!("  - {}", evidence.summary()));
        }
        
        lines.join("\n")
    }
}

impl EvidenceReport for EvaluationEvidenceReport {
    fn execution_id(&self) -> Uuid {
        self.execution_id
    }
    
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    
    fn summary(&self) -> String {
        self.summary_text()
    }
    
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Builder para criar EvaluationEvidenceReport
#[derive(Debug, Default)]
pub struct EvaluationEvidenceBuilder {
    execution_id: Option<Uuid>,
    question: String,
    answer_length: usize,
    required: Vec<EvaluationType>,
    evidences: Vec<EvaluationEvidence>,
}

impl EvaluationEvidenceBuilder {
    /// Cria um novo builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Define o ID da execução
    pub fn execution_id(mut self, id: Uuid) -> Self {
        self.execution_id = Some(id);
        self
    }
    
    /// Define a pergunta
    pub fn question(mut self, q: impl Into<String>) -> Self {
        self.question = q.into();
        self
    }
    
    /// Define o tamanho da resposta
    pub fn answer_length(mut self, len: usize) -> Self {
        self.answer_length = len;
        self
    }
    
    /// Define avaliações requeridas
    pub fn required(mut self, types: Vec<EvaluationType>) -> Self {
        self.required = types;
        self
    }
    
    /// Adiciona uma evidência
    pub fn add_evidence(mut self, evidence: EvaluationEvidence) -> Self {
        self.evidences.push(evidence);
        self
    }
    
    /// Constrói o relatório
    pub fn build(self) -> EvaluationEvidenceReport {
        let mut report = EvaluationEvidenceReport::new(
            self.execution_id.unwrap_or_else(Uuid::new_v4),
            self.question,
            self.answer_length,
        );
        
        report.set_required_evaluations(self.required);
        
        for evidence in self.evidences {
            report.add_evaluation(evidence);
        }
        
        report.finalize();
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Testes para EvaluationEvidence
    // ========================================================================

    #[test]
    fn test_evaluation_evidence_new() {
        let evidence = EvaluationEvidence::new(EvaluationType::Definitive);
        
        assert_eq!(evidence.eval_type, EvaluationType::Definitive);
        assert!(!evidence.prompt_generated);
        assert!(!evidence.llm_called);
        assert!(!evidence.result_passed);
    }

    #[test]
    fn test_evaluation_evidence_prompt_generated() {
        let mut evidence = EvaluationEvidence::new(EvaluationType::Freshness);
        evidence.prompt_generated(1500);
        
        assert!(evidence.prompt_generated);
        assert_eq!(evidence.prompt_length, 1500);
    }

    #[test]
    fn test_evaluation_evidence_llm_called() {
        let mut evidence = EvaluationEvidence::new(EvaluationType::Plurality);
        evidence.llm_called(Duration::from_millis(500), 300);
        
        assert!(evidence.llm_called);
        assert_eq!(evidence.llm_latency, Duration::from_millis(500));
        assert_eq!(evidence.llm_tokens_used, 300);
    }

    #[test]
    fn test_evaluation_evidence_set_result() {
        let mut evidence = EvaluationEvidence::new(EvaluationType::Completeness);
        evidence.set_result(true, 0.95, 150, 0);
        
        assert!(evidence.result_passed);
        assert_eq!(evidence.result_confidence, 0.95);
        assert_eq!(evidence.reasoning_length, 150);
        assert_eq!(evidence.suggestions_count, 0);
    }

    #[test]
    fn test_evaluation_evidence_set_error() {
        let mut evidence = EvaluationEvidence::new(EvaluationType::Strict);
        evidence.set_error("LLM timeout");
        
        assert!(!evidence.is_success());
        assert_eq!(evidence.error, Some("LLM timeout".to_string()));
    }

    #[test]
    fn test_evaluation_evidence_summary() {
        let mut evidence = EvaluationEvidence::new(EvaluationType::Definitive);
        evidence.set_result(true, 0.9, 100, 0);
        evidence.llm_called(Duration::from_millis(200), 150);
        
        let summary = evidence.summary();
        
        assert!(summary.contains("definitive"));
        assert!(summary.contains("PASS"));
        assert!(summary.contains("0.90"));
        assert!(summary.contains("150"));
    }

    // ========================================================================
    // Testes para EvaluationEvidenceReport
    // ========================================================================

    #[test]
    fn test_evaluation_evidence_report_new() {
        let id = Uuid::new_v4();
        let report = EvaluationEvidenceReport::new(id, "What is Rust?", 500);
        
        assert_eq!(report.execution_id, id);
        assert_eq!(report.question, "What is Rust?");
        assert_eq!(report.answer_length, 500);
        assert!(!report.final_verdict);
    }

    #[test]
    fn test_evaluation_evidence_report_add_evaluation() {
        let mut report = EvaluationEvidenceReport::default();
        
        let mut evidence = EvaluationEvidence::new(EvaluationType::Definitive);
        evidence.llm_called(Duration::from_millis(100), 200);
        evidence.set_result(true, 0.9, 50, 0);
        
        report.add_evaluation(evidence);
        
        assert_eq!(report.evaluations_executed.len(), 1);
        assert_eq!(report.total_llm_tokens, 200);
    }

    #[test]
    fn test_evaluation_evidence_report_early_fail() {
        let mut report = EvaluationEvidenceReport::default();
        
        let mut e1 = EvaluationEvidence::new(EvaluationType::Definitive);
        e1.set_result(true, 0.9, 50, 0);
        report.add_evaluation(e1);
        
        let mut e2 = EvaluationEvidence::new(EvaluationType::Freshness);
        e2.set_result(false, 0.3, 100, 2);
        report.add_evaluation(e2);
        
        assert!(report.early_fail_reason.is_some());
        assert!(report.early_fail_reason.as_ref().unwrap().contains("freshness"));
    }

    #[test]
    fn test_evaluation_evidence_report_finalize() {
        let mut report = EvaluationEvidenceReport::default();
        
        let mut e1 = EvaluationEvidence::new(EvaluationType::Definitive);
        e1.set_result(true, 0.9, 50, 0);
        report.add_evaluation(e1);
        
        let mut e2 = EvaluationEvidence::new(EvaluationType::Freshness);
        e2.set_result(true, 0.8, 60, 0);
        report.add_evaluation(e2);
        
        report.finalize();
        
        assert!(report.final_verdict); // Todas passaram
    }

    #[test]
    fn test_evaluation_evidence_report_finalize_fail() {
        let mut report = EvaluationEvidenceReport::default();
        
        let mut e1 = EvaluationEvidence::new(EvaluationType::Definitive);
        e1.set_result(false, 0.3, 50, 1);
        report.add_evaluation(e1);
        
        report.finalize();
        
        assert!(!report.final_verdict);
    }

    #[test]
    fn test_evaluation_evidence_report_success_rate() {
        let mut report = EvaluationEvidenceReport::default();
        
        for i in 0..10 {
            let mut e = EvaluationEvidence::new(EvaluationType::Definitive);
            e.set_result(i < 7, 0.5, 10, 0); // 7 passed
            report.add_evaluation(e);
        }
        
        assert!((report.success_rate() - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_evaluation_evidence_report_avg_confidence() {
        let mut report = EvaluationEvidenceReport::default();
        
        let mut e1 = EvaluationEvidence::new(EvaluationType::Definitive);
        e1.set_result(true, 0.8, 10, 0);
        report.add_evaluation(e1);
        
        let mut e2 = EvaluationEvidence::new(EvaluationType::Freshness);
        e2.set_result(true, 1.0, 10, 0);
        report.add_evaluation(e2);
        
        // (0.8 + 1.0) / 2 = 0.9
        assert!((report.avg_confidence() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_evaluation_evidence_report_failed_evaluations() {
        let mut report = EvaluationEvidenceReport::default();
        
        let mut e1 = EvaluationEvidence::new(EvaluationType::Definitive);
        e1.set_result(true, 0.9, 10, 0);
        report.add_evaluation(e1);
        
        let mut e2 = EvaluationEvidence::new(EvaluationType::Freshness);
        e2.set_result(false, 0.3, 10, 2);
        report.add_evaluation(e2);
        
        let failed = report.failed_evaluations();
        
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].eval_type, EvaluationType::Freshness);
    }

    #[test]
    fn test_evaluation_evidence_report_summary() {
        let mut report = EvaluationEvidenceReport::new(
            Uuid::nil(),
            "What is Rust programming language?",
            1000,
        );
        
        report.set_required_evaluations(vec![EvaluationType::Definitive]);
        
        let mut e = EvaluationEvidence::new(EvaluationType::Definitive);
        e.set_result(true, 0.9, 50, 0);
        e.llm_called(Duration::from_millis(100), 200);
        report.add_evaluation(e);
        
        report.finalize();
        
        let summary = report.summary_text();
        
        assert!(summary.contains("EvaluationEvidenceReport"));
        assert!(summary.contains("What is Rust"));
        assert!(summary.contains("APPROVED"));
        assert!(summary.contains("definitive"));
    }

    // ========================================================================
    // Testes para EvaluationEvidenceBuilder
    // ========================================================================

    #[test]
    fn test_evaluation_evidence_builder() {
        let id = Uuid::new_v4();
        
        let mut e = EvaluationEvidence::new(EvaluationType::Definitive);
        e.set_result(true, 0.9, 50, 0);
        
        let report = EvaluationEvidenceBuilder::new()
            .execution_id(id)
            .question("Test question")
            .answer_length(500)
            .required(vec![EvaluationType::Definitive])
            .add_evidence(e)
            .build();
        
        assert_eq!(report.execution_id, id);
        assert_eq!(report.question, "Test question");
        assert_eq!(report.answer_length, 500);
        assert_eq!(report.evaluations_required.len(), 1);
        assert_eq!(report.evaluations_executed.len(), 1);
        assert!(report.final_verdict); // finalize() foi chamado
    }

    // ========================================================================
    // Testes para trait EvidenceReport
    // ========================================================================

    #[test]
    fn test_evidence_report_trait_impl() {
        let id = Uuid::new_v4();
        let report = EvaluationEvidenceReport::new(id, "Q", 100);
        
        assert_eq!(report.execution_id(), id);
        assert!(!report.summary().is_empty());
        assert!(!report.to_json().is_null());
    }
}

