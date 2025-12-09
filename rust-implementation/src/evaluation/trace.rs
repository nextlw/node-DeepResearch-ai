// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// RASTREAMENTO DE AVALIAÇÃO (EVALUATION TRACE)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Sistema de rastreamento para operações de avaliação.
// Permite saber como cada avaliação funcionou e quanto tempo durou.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

use super::EvaluationType;

/// Trace de uma única avaliação
///
/// Captura todos os detalhes de uma avaliação individual:
/// - Tipo de avaliação executada
/// - Pergunta e hash da resposta (não loga resposta inteira)
/// - Tempo de execução e tokens usados
/// - Resultado (passou/falhou) e confiança
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationTrace {
    /// ID único deste trace
    pub trace_id: Uuid,
    /// ID da execução pai (para agrupar traces)
    pub execution_id: Uuid,
    /// Tipo de avaliação
    pub eval_type: EvaluationType,
    /// Pergunta avaliada
    pub question: String,
    /// Hash da resposta (não loga resposta inteira por privacidade)
    pub answer_hash: String,
    /// Tamanho da resposta em caracteres
    pub answer_length: usize,
    /// Timestamp de início
    pub start_time: DateTime<Utc>,
    /// Timestamp de fim
    pub end_time: Option<DateTime<Utc>>,
    /// Tokens de entrada usados (estimativa)
    pub input_tokens: u32,
    /// Tokens de saída usados (estimativa)
    pub output_tokens: u32,
    /// Se a avaliação passou
    pub passed: bool,
    /// Confiança da avaliação (0.0 - 1.0)
    pub confidence: f32,
    /// Tamanho do reasoning em caracteres
    pub reasoning_length: usize,
    /// Número de sugestões geradas
    pub suggestions_count: usize,
    /// Metadados adicionais
    pub metadata: HashMap<String, String>,
    /// Duração interna (não serializado)
    #[serde(skip)]
    start_instant: Option<Instant>,
}

impl EvaluationTrace {
    /// Cria um novo trace de avaliação
    pub fn new(
        execution_id: Uuid,
        eval_type: EvaluationType,
        question: &str,
        answer: &str,
    ) -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            execution_id,
            eval_type,
            question: question.to_string(),
            answer_hash: Self::hash_answer(answer),
            answer_length: answer.len(),
            start_time: Utc::now(),
            end_time: None,
            input_tokens: Self::estimate_tokens(question, answer),
            output_tokens: 0,
            passed: false,
            confidence: 0.0,
            reasoning_length: 0,
            suggestions_count: 0,
            metadata: HashMap::new(),
            start_instant: Some(Instant::now()),
        }
    }

    /// Calcula hash simples da resposta (primeiros 8 chars do hash)
    fn hash_answer(answer: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        answer.hash(&mut hasher);
        format!("{:x}", hasher.finish())[..8].to_string()
    }

    /// Estima tokens (aproximação: 4 chars = 1 token)
    fn estimate_tokens(question: &str, answer: &str) -> u32 {
        ((question.len() + answer.len()) / 4) as u32
    }

    /// Marca o trace como concluído
    pub fn complete(
        &mut self,
        passed: bool,
        confidence: f32,
        reasoning: &str,
        suggestions_count: usize,
    ) {
        self.end_time = Some(Utc::now());
        self.passed = passed;
        self.confidence = confidence;
        self.reasoning_length = reasoning.len();
        self.suggestions_count = suggestions_count;
        self.output_tokens = (reasoning.len() / 4) as u32;
    }

    /// Adiciona metadados ao trace
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Calcula duração da avaliação
    pub fn duration(&self) -> Duration {
        if let Some(start) = self.start_instant {
            start.elapsed()
        } else if let Some(end) = self.end_time {
            let diff = end - self.start_time;
            Duration::from_millis(diff.num_milliseconds().max(0) as u64)
        } else {
            Duration::ZERO
        }
    }

    /// Verifica se está em andamento
    pub fn is_in_progress(&self) -> bool {
        self.end_time.is_none()
    }

    /// Retorna total de tokens usados
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    /// Retorna resumo formatado do trace
    pub fn summary(&self) -> String {
        let status = if self.passed { "✓ PASS" } else { "✗ FAIL" };
        let duration = self.duration().as_millis();
        
        format!(
            "[{}] {} {} | conf={:.0}% | {}ms | {} tokens | reason={}chars",
            &self.trace_id.to_string()[..8],
            self.eval_type,
            status,
            self.confidence * 100.0,
            duration,
            self.total_tokens(),
            self.reasoning_length
        )
    }
}

/// Relatório agregado de avaliações
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationReport {
    /// ID da execução
    pub execution_id: Uuid,
    /// Timestamp de criação
    pub timestamp: DateTime<Utc>,
    /// Pergunta original
    pub question: String,
    /// Traces de cada avaliação
    pub traces: Vec<EvaluationTrace>,
    /// Se todas as avaliações passaram
    pub overall_passed: bool,
    /// Tipo onde falhou (se aplicável)
    pub failed_at: Option<EvaluationType>,
    /// Total de tokens usados
    pub total_tokens: u32,
    /// Tempo total de execução
    pub total_duration_ms: u64,
    /// Confiança média
    pub avg_confidence: f32,
}

impl EvaluationReport {
    /// Cria um novo relatório
    pub fn new(execution_id: Uuid, question: &str) -> Self {
        Self {
            execution_id,
            timestamp: Utc::now(),
            question: question.to_string(),
            traces: Vec::new(),
            overall_passed: true,
            failed_at: None,
            total_tokens: 0,
            total_duration_ms: 0,
            avg_confidence: 0.0,
        }
    }

    /// Adiciona um trace ao relatório
    pub fn add_trace(&mut self, trace: EvaluationTrace) {
        if !trace.passed && self.overall_passed {
            self.overall_passed = false;
            self.failed_at = Some(trace.eval_type);
        }
        
        self.total_tokens += trace.total_tokens();
        self.total_duration_ms += trace.duration().as_millis() as u64;
        self.traces.push(trace);
    }

    /// Finaliza o relatório calculando métricas agregadas
    pub fn finalize(&mut self) {
        if self.traces.is_empty() {
            return;
        }

        let sum_confidence: f32 = self.traces.iter().map(|t| t.confidence).sum();
        self.avg_confidence = sum_confidence / self.traces.len() as f32;
    }

    /// Retorna número de avaliações que passaram
    pub fn passed_count(&self) -> usize {
        self.traces.iter().filter(|t| t.passed).count()
    }

    /// Retorna número de avaliações que falharam
    pub fn failed_count(&self) -> usize {
        self.traces.iter().filter(|t| !t.passed).count()
    }

    /// Retorna taxa de sucesso
    pub fn success_rate(&self) -> f32 {
        if self.traces.is_empty() {
            return 0.0;
        }
        self.passed_count() as f32 / self.traces.len() as f32
    }

    /// Retorna traces por tipo de avaliação
    pub fn traces_by_type(&self) -> HashMap<EvaluationType, Vec<&EvaluationTrace>> {
        let mut map: HashMap<EvaluationType, Vec<&EvaluationTrace>> = HashMap::new();
        for trace in &self.traces {
            map.entry(trace.eval_type).or_default().push(trace);
        }
        map
    }

    /// Retorna resumo formatado do relatório
    pub fn summary(&self) -> String {
        let status = if self.overall_passed { "✓ PASSED" } else { "✗ FAILED" };
        
        format!(
            "EvaluationReport [{}] {}\n\
             Question: '{}'\n\
             Evaluations: {} ({} passed, {} failed)\n\
             Avg confidence: {:.0}%\n\
             Total tokens: {} | Duration: {}ms\n\
             Failed at: {:?}",
            &self.execution_id.to_string()[..8],
            status,
            if self.question.len() > 50 {
                format!("{}...", &self.question[..50])
            } else {
                self.question.clone()
            },
            self.traces.len(),
            self.passed_count(),
            self.failed_count(),
            self.avg_confidence * 100.0,
            self.total_tokens,
            self.total_duration_ms,
            self.failed_at
        )
    }

    /// Retorna detalhes de cada avaliação
    pub fn details(&self) -> String {
        let mut output = self.summary();
        output.push_str("\n\nDetails:\n");
        
        for trace in &self.traces {
            output.push_str(&format!("  {}\n", trace.summary()));
        }
        
        output
    }
}

/// Coletor de traces de avaliação
#[derive(Debug, Clone)]
pub struct EvaluationTraceCollector {
    /// Relatórios coletados
    reports: Vec<EvaluationReport>,
    /// Relatório atual em construção
    current_report: Option<EvaluationReport>,
}

impl EvaluationTraceCollector {
    /// Cria um novo coletor
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            current_report: None,
        }
    }

    /// Inicia um novo relatório
    pub fn start_report(&mut self, execution_id: Uuid, question: &str) {
        // Finaliza relatório anterior se existir
        if let Some(mut report) = self.current_report.take() {
            report.finalize();
            self.reports.push(report);
        }
        
        self.current_report = Some(EvaluationReport::new(execution_id, question));
    }

    /// Adiciona trace ao relatório atual
    pub fn add_trace(&mut self, trace: EvaluationTrace) {
        if let Some(ref mut report) = self.current_report {
            report.add_trace(trace);
        }
    }

    /// Finaliza relatório atual
    pub fn finish_report(&mut self) -> Option<EvaluationReport> {
        if let Some(mut report) = self.current_report.take() {
            report.finalize();
            let result = report.clone();
            self.reports.push(report);
            return Some(result);
        }
        None
    }

    /// Retorna relatório atual
    pub fn current(&self) -> Option<&EvaluationReport> {
        self.current_report.as_ref()
    }

    /// Retorna todos os relatórios
    pub fn all_reports(&self) -> &[EvaluationReport] {
        &self.reports
    }

    /// Retorna número de relatórios
    pub fn count(&self) -> usize {
        self.reports.len()
    }

    /// Calcula estatísticas agregadas
    pub fn stats(&self) -> CollectorStats {
        let total_reports = self.reports.len();
        let passed_reports = self.reports.iter().filter(|r| r.overall_passed).count();
        let total_evals: usize = self.reports.iter().map(|r| r.traces.len()).sum();
        let total_tokens: u32 = self.reports.iter().map(|r| r.total_tokens).sum();
        let total_duration: u64 = self.reports.iter().map(|r| r.total_duration_ms).sum();

        let avg_confidence = if total_reports > 0 {
            self.reports.iter().map(|r| r.avg_confidence).sum::<f32>() / total_reports as f32
        } else {
            0.0
        };

        CollectorStats {
            total_reports,
            passed_reports,
            failed_reports: total_reports - passed_reports,
            total_evaluations: total_evals,
            total_tokens,
            total_duration_ms: total_duration,
            avg_confidence,
            success_rate: if total_reports > 0 {
                passed_reports as f32 / total_reports as f32
            } else {
                0.0
            },
        }
    }
}

impl Default for EvaluationTraceCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Estatísticas agregadas do coletor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorStats {
    /// Total de relatórios
    pub total_reports: usize,
    /// Relatórios que passaram
    pub passed_reports: usize,
    /// Relatórios que falharam
    pub failed_reports: usize,
    /// Total de avaliações executadas
    pub total_evaluations: usize,
    /// Total de tokens usados
    pub total_tokens: u32,
    /// Tempo total de execução em ms
    pub total_duration_ms: u64,
    /// Confiança média
    pub avg_confidence: f32,
    /// Taxa de sucesso
    pub success_rate: f32,
}

impl CollectorStats {
    /// Retorna resumo formatado
    pub fn summary(&self) -> String {
        format!(
            "CollectorStats: {} reports ({} passed, {} failed) | {} evals | {} tokens | {}ms | {:.0}% success | {:.0}% avg confidence",
            self.total_reports,
            self.passed_reports,
            self.failed_reports,
            self.total_evaluations,
            self.total_tokens,
            self.total_duration_ms,
            self.success_rate * 100.0,
            self.avg_confidence * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluation_trace_new() {
        let execution_id = Uuid::new_v4();
        let trace = EvaluationTrace::new(
            execution_id,
            EvaluationType::Definitive,
            "What is Rust?",
            "Rust is a systems programming language.",
        );

        assert_eq!(trace.execution_id, execution_id);
        assert_eq!(trace.eval_type, EvaluationType::Definitive);
        assert!(!trace.passed);
        assert!(trace.is_in_progress());
    }

    #[test]
    fn test_evaluation_trace_complete() {
        let execution_id = Uuid::new_v4();
        let mut trace = EvaluationTrace::new(
            execution_id,
            EvaluationType::Definitive,
            "What is Rust?",
            "Rust is a systems programming language.",
        );

        trace.complete(true, 0.95, "The answer is definitive and clear.", 0);

        assert!(trace.passed);
        assert!((trace.confidence - 0.95).abs() < 0.01);
        assert!(!trace.is_in_progress());
        assert!(trace.reasoning_length > 0);
    }

    #[test]
    fn test_evaluation_trace_hash() {
        let trace1 = EvaluationTrace::new(
            Uuid::new_v4(),
            EvaluationType::Definitive,
            "Q",
            "Same answer",
        );
        let trace2 = EvaluationTrace::new(
            Uuid::new_v4(),
            EvaluationType::Definitive,
            "Q",
            "Same answer",
        );

        assert_eq!(trace1.answer_hash, trace2.answer_hash);
    }

    #[test]
    fn test_evaluation_trace_metadata() {
        let mut trace = EvaluationTrace::new(
            Uuid::new_v4(),
            EvaluationType::Definitive,
            "Q",
            "A",
        );

        trace.add_metadata("model", "gpt-4");
        trace.add_metadata("temperature", "0.7");

        assert_eq!(trace.metadata.get("model"), Some(&"gpt-4".to_string()));
    }

    #[test]
    fn test_evaluation_trace_summary() {
        let mut trace = EvaluationTrace::new(
            Uuid::new_v4(),
            EvaluationType::Definitive,
            "Q",
            "A",
        );
        trace.complete(true, 0.9, "Good", 0);

        let summary = trace.summary();
        assert!(summary.contains("PASS"));
        assert!(summary.contains("definitive")); // as_str() retorna lowercase
    }

    #[test]
    fn test_evaluation_report_new() {
        let execution_id = Uuid::new_v4();
        let report = EvaluationReport::new(execution_id, "What is Rust?");

        assert_eq!(report.execution_id, execution_id);
        assert!(report.overall_passed);
        assert!(report.traces.is_empty());
    }

    #[test]
    fn test_evaluation_report_add_trace() {
        let execution_id = Uuid::new_v4();
        let mut report = EvaluationReport::new(execution_id, "What is Rust?");

        let mut trace = EvaluationTrace::new(
            execution_id,
            EvaluationType::Definitive,
            "What is Rust?",
            "Answer",
        );
        trace.complete(true, 0.9, "Good", 0);
        
        report.add_trace(trace);

        assert_eq!(report.traces.len(), 1);
        assert!(report.overall_passed);
    }

    #[test]
    fn test_evaluation_report_failure() {
        let execution_id = Uuid::new_v4();
        let mut report = EvaluationReport::new(execution_id, "What is Rust?");

        let mut trace1 = EvaluationTrace::new(
            execution_id,
            EvaluationType::Definitive,
            "Q",
            "A",
        );
        trace1.complete(true, 0.9, "Good", 0);
        report.add_trace(trace1);

        let mut trace2 = EvaluationTrace::new(
            execution_id,
            EvaluationType::Freshness,
            "Q",
            "A",
        );
        trace2.complete(false, 0.3, "Outdated", 2);
        report.add_trace(trace2);

        assert!(!report.overall_passed);
        assert_eq!(report.failed_at, Some(EvaluationType::Freshness));
    }

    #[test]
    fn test_evaluation_report_finalize() {
        let execution_id = Uuid::new_v4();
        let mut report = EvaluationReport::new(execution_id, "Q");

        let mut trace1 = EvaluationTrace::new(execution_id, EvaluationType::Definitive, "Q", "A");
        trace1.complete(true, 0.8, "R", 0);
        report.add_trace(trace1);

        let mut trace2 = EvaluationTrace::new(execution_id, EvaluationType::Completeness, "Q", "A");
        trace2.complete(true, 0.9, "R", 0);
        report.add_trace(trace2);

        report.finalize();

        assert!((report.avg_confidence - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_evaluation_report_stats() {
        let execution_id = Uuid::new_v4();
        let mut report = EvaluationReport::new(execution_id, "Q");

        let mut trace1 = EvaluationTrace::new(execution_id, EvaluationType::Definitive, "Q", "A");
        trace1.complete(true, 0.9, "R", 0);
        report.add_trace(trace1);

        let mut trace2 = EvaluationTrace::new(execution_id, EvaluationType::Freshness, "Q", "A");
        trace2.complete(false, 0.3, "R", 2);
        report.add_trace(trace2);

        assert_eq!(report.passed_count(), 1);
        assert_eq!(report.failed_count(), 1);
        assert!((report.success_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_collector_new() {
        let collector = EvaluationTraceCollector::new();
        assert_eq!(collector.count(), 0);
        assert!(collector.current().is_none());
    }

    #[test]
    fn test_collector_start_report() {
        let mut collector = EvaluationTraceCollector::new();
        let execution_id = Uuid::new_v4();
        
        collector.start_report(execution_id, "What is Rust?");
        
        assert!(collector.current().is_some());
        assert_eq!(collector.current().unwrap().question, "What is Rust?");
    }

    #[test]
    fn test_collector_add_trace() {
        let mut collector = EvaluationTraceCollector::new();
        let execution_id = Uuid::new_v4();
        
        collector.start_report(execution_id, "Q");
        
        let mut trace = EvaluationTrace::new(execution_id, EvaluationType::Definitive, "Q", "A");
        trace.complete(true, 0.9, "R", 0);
        collector.add_trace(trace);

        assert_eq!(collector.current().unwrap().traces.len(), 1);
    }

    #[test]
    fn test_collector_finish_report() {
        let mut collector = EvaluationTraceCollector::new();
        let execution_id = Uuid::new_v4();
        
        collector.start_report(execution_id, "Q");
        
        let mut trace = EvaluationTrace::new(execution_id, EvaluationType::Definitive, "Q", "A");
        trace.complete(true, 0.9, "R", 0);
        collector.add_trace(trace);

        let report = collector.finish_report();
        
        assert!(report.is_some());
        assert_eq!(collector.count(), 1);
        assert!(collector.current().is_none());
    }

    #[test]
    fn test_collector_stats() {
        let mut collector = EvaluationTraceCollector::new();
        
        // Report 1 - passa
        let id1 = Uuid::new_v4();
        collector.start_report(id1, "Q1");
        let mut t1 = EvaluationTrace::new(id1, EvaluationType::Definitive, "Q1", "A1");
        t1.complete(true, 0.9, "R", 0);
        collector.add_trace(t1);
        collector.finish_report();

        // Report 2 - falha
        let id2 = Uuid::new_v4();
        collector.start_report(id2, "Q2");
        let mut t2 = EvaluationTrace::new(id2, EvaluationType::Definitive, "Q2", "A2");
        t2.complete(false, 0.3, "R", 2);
        collector.add_trace(t2);
        collector.finish_report();

        let stats = collector.stats();
        
        assert_eq!(stats.total_reports, 2);
        assert_eq!(stats.passed_reports, 1);
        assert_eq!(stats.failed_reports, 1);
        assert!((stats.success_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_collector_auto_finalize_previous() {
        let mut collector = EvaluationTraceCollector::new();
        
        // Inicia report 1
        collector.start_report(Uuid::new_v4(), "Q1");
        
        // Inicia report 2 sem finalizar o 1
        collector.start_report(Uuid::new_v4(), "Q2");
        
        // Report 1 deve ter sido finalizado automaticamente
        assert_eq!(collector.count(), 1);
    }

    #[test]
    fn test_traces_by_type() {
        let execution_id = Uuid::new_v4();
        let mut report = EvaluationReport::new(execution_id, "Q");

        let mut t1 = EvaluationTrace::new(execution_id, EvaluationType::Definitive, "Q", "A");
        t1.complete(true, 0.9, "R", 0);
        report.add_trace(t1);

        let mut t2 = EvaluationTrace::new(execution_id, EvaluationType::Definitive, "Q", "A");
        t2.complete(true, 0.85, "R", 0);
        report.add_trace(t2);

        let mut t3 = EvaluationTrace::new(execution_id, EvaluationType::Freshness, "Q", "A");
        t3.complete(true, 0.8, "R", 0);
        report.add_trace(t3);

        let by_type = report.traces_by_type();
        
        assert_eq!(by_type.get(&EvaluationType::Definitive).unwrap().len(), 2);
        assert_eq!(by_type.get(&EvaluationType::Freshness).unwrap().len(), 1);
    }
}

