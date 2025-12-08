// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// AGENT ANALYZER - Análise de Erros em Background
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Analisa padrões de falha do agente após 2+ falhas consecutivas.
// Roda em ParallelTask (tokio::spawn) para não bloquear a pipeline principal.
// Gera hints de melhoria que são injetados no próximo prompt do LLM.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::DiaryEntry;
use crate::llm::{LlmClient, LlmError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Resultado da análise de erros do agente.
///
/// Contém um resumo cronológico (recap), identificação do problema (blame),
/// e sugestões acionáveis de melhoria (improvement).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalysis {
    /// Resumo cronológico das ações tomadas.
    ///
    /// Destaca padrões, repetições e onde o processo começou a dar errado.
    pub recap: String,

    /// Identificação específica do que deu errado.
    ///
    /// Aponta para passos ou padrões que levaram à resposta inadequada.
    pub blame: String,

    /// Sugestões acionáveis de melhoria.
    ///
    /// Fornece orientações concretas que poderiam levar a um melhor resultado.
    pub improvement: String,

    /// Tempo de execução da análise em milissegundos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
}

impl Default for AgentAnalysis {
    fn default() -> Self {
        Self {
            recap: String::new(),
            blame: String::new(),
            improvement: String::new(),
            duration_ms: None,
        }
    }
}

/// Formata o diário do agente para análise.
///
/// Converte as entradas do diário em um formato textual legível
/// que será enviado ao LLM para análise.
fn format_diary_for_analysis(diary: &[DiaryEntry], original_question: &str) -> String {
    let mut output = String::new();
    output.push_str("<steps>\n\n");

    for (i, entry) in diary.iter().enumerate() {
        let step_num = i + 1;
        match entry {
            DiaryEntry::Search {
                queries,
                think,
                urls_found,
            } => {
                let keywords: Vec<_> = queries.iter().map(|q| q.q.as_str()).collect();
                output.push_str(&format!(
                    "At step {}, you took the **search** action and look for external information for the question: \"{}\".\n\
                    In particular, you tried to search for the following keywords: {}.\n\
                    You found {} URLs and add them to your URL list and **visit** them later when needed.\n\
                    Think: {}\n\n",
                    step_num,
                    original_question,
                    keywords.join(", "),
                    urls_found,
                    think
                ));
            }
            DiaryEntry::Read { urls, think } => {
                let url_list: Vec<_> = urls.iter().take(3).collect();
                output.push_str(&format!(
                    "At step {}, you took the **visit** action and deep dive into the following URLs:\n\
                    {}\n\
                    You found some useful information on the web and add them to your knowledge for future reference.\n\
                    Think: {}\n\n",
                    step_num,
                    url_list
                        .iter()
                        .map(|u| u.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    think
                ));
            }
            DiaryEntry::Reflect { questions, think } => {
                output.push_str(&format!(
                    "At step {}, you took the **reflect** action and identified {} gap questions:\n\
                    {}\n\
                    Think: {}\n\n",
                    step_num,
                    questions.len(),
                    questions
                        .iter()
                        .take(3)
                        .map(|q| format!("- {}", q))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    think
                ));
            }
            DiaryEntry::FailedAnswer {
                answer,
                eval_type,
                reason,
            } => {
                output.push_str(&format!(
                    "At step {}, you took **answer** action but evaluator thinks it is not a good answer:\n\
                    Answer (truncated): {}...\n\
                    Failed evaluation: {:?}\n\
                    Reason: {}\n\n",
                    step_num,
                    answer.chars().take(200).collect::<String>(),
                    eval_type,
                    reason
                ));
            }
            DiaryEntry::Coding { code, think } => {
                output.push_str(&format!(
                    "At step {}, you took the **coding** action and executed code:\n\
                    Code (truncated): {}...\n\
                    Think: {}\n\n",
                    step_num,
                    code.chars().take(100).collect::<String>(),
                    think
                ));
            }
        }
    }

    output.push_str("</steps>");
    output
}

/// Constrói o prompt do sistema para análise de erros.
fn build_system_prompt() -> String {
    r#"You are an expert at analyzing search and reasoning processes. Your task is to analyze the given sequence of steps and identify what went wrong in the search process.

<rules>
1. The sequence of actions taken
2. The effectiveness of each step
3. The logic between consecutive steps
4. Alternative approaches that could have been taken
5. Signs of getting stuck in repetitive patterns
6. Whether the final answer matches the accumulated information

Analyze the steps and provide detailed feedback following these guidelines:
- In the recap: Summarize key actions chronologically, highlight patterns, and identify where the process started to go wrong
- In the blame: Point to specific steps or patterns that led to the inadequate answer
- In the improvement: Provide actionable suggestions that could have led to a better outcome
</rules>

<example>
<input>
<steps>

At step 1, you took the **search** action and look for external information for the question: "how old is jina ai ceo?".
In particular, you tried to search for the following keywords: "jina ai ceo age".
You found quite some information and add them to your URL list and **visit** them later when needed.


At step 2, you took the **visit** action and deep dive into the following URLs:
https://www.linkedin.com/in/hxiao87
https://www.crunchbase.com/person/han-xiao
You found some useful information on the web and add them to your knowledge for future reference.


At step 3, you took the **search** action and look for external information for the question: "how old is jina ai ceo?".
In particular, you tried to search for the following keywords: "Han Xiao birthdate, Jina AI founder birthdate".
You found quite some information and add them to your URL list and **visit** them later when needed.


At step 4, you took the **search** action and look for external information for the question: "how old is jina ai ceo?".
In particular, you tried to search for the following keywords: han xiao birthday.
But then you realized you have already searched for these keywords before.
You decided to think out of the box or cut from a completely different angle.


At step 5, you took the **search** action and look for external information for the question: "how old is jina ai ceo?".
In particular, you tried to search for the following keywords: han xiao birthday.
But then you realized you have already searched for these keywords before.
You decided to think out of the box or cut from a completely different angle.


At step 6, you took the **visit** action and deep dive into the following URLs:
https://kpopwall.com/han-xiao/
https://www.idolbirthdays.net/han-xiao
You found some useful information on the web and add them to your knowledge for future reference.


At step 7, you took **answer** action but evaluator thinks it is not a good answer:

</steps>

Original question:
how old is jina ai ceo?

Your answer:
The age of the Jina AI CEO cannot be definitively determined from the provided information.

The evaluator thinks your answer is bad because:
The answer is not definitive and fails to provide the requested information.  Lack of information is unacceptable, more search and deep reasoning is needed.
</input>


<output>
{
  "recap": "The search process consisted of 7 steps with multiple search and visit actions. The initial searches focused on basic biographical information through LinkedIn and Crunchbase (steps 1-2). When this didn't yield the specific age information, additional searches were conducted for birthdate information (steps 3-5). The process showed signs of repetition in steps 4-5 with identical searches. Final visits to entertainment websites (step 6) suggested a loss of focus on reliable business sources.",

  "blame": "The root cause of failure was getting stuck in a repetitive search pattern without adapting the strategy. Steps 4-5 repeated the same search, and step 6 deviated to less reliable entertainment sources instead of exploring business journals, news articles, or professional databases. Additionally, the process didn't attempt to triangulate age through indirect information like education history or career milestones.",

  "improvement": "1. Avoid repeating identical searches and implement a strategy to track previously searched terms. 2. When direct age/birthdate searches fail, try indirect approaches like: searching for earliest career mentions, finding university graduation years, or identifying first company founding dates. 3. Focus on high-quality business sources and avoid entertainment websites for professional information. 4. Consider using industry event appearances or conference presentations where age-related context might be mentioned. 5. If exact age cannot be determined, provide an estimated range based on career timeline and professional achievements."
}
</output>
</example>

IMPORTANT: You MUST respond ONLY with valid JSON in the exact format shown above. No markdown, no explanations, just the JSON object with "recap", "blame", and "improvement" fields."#.to_string()
}

/// Constrói o prompt do usuário para análise.
fn build_user_prompt(
    diary_text: &str,
    original_question: &str,
    failed_answer: &str,
    failure_reason: &str,
) -> String {
    format!(
        "{}\n\nOriginal question:\n{}\n\nYour answer:\n{}\n\nThe evaluator thinks your answer is bad because:\n{}",
        diary_text,
        original_question,
        failed_answer,
        failure_reason
    )
}

/// Faz o parsing do JSON retornado pelo LLM.
fn parse_analysis_response(response: &str) -> Result<AgentAnalysis, LlmError> {
    // Tentar extrair JSON da resposta (pode vir com markdown)
    let json_str = if response.contains("```json") {
        response
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(response)
            .trim()
    } else if response.contains("```") {
        response
            .split("```")
            .nth(1)
            .unwrap_or(response)
            .trim()
    } else {
        response.trim()
    };

    serde_json::from_str(json_str).map_err(|e| {
        LlmError::ParseError(format!(
            "Failed to parse AgentAnalysis JSON: {}. Response was: {}",
            e,
            json_str.chars().take(500).collect::<String>()
        ))
    })
}

/// Analisa os passos do agente para identificar padrões de erro.
///
/// Esta função é projetada para rodar em `tokio::spawn` de forma assíncrona,
/// sem bloquear a pipeline principal do agente.
///
/// # Argumentos
/// * `diary` - Histórico de ações do agente
/// * `original_question` - Pergunta original do usuário
/// * `failed_answer` - Última resposta que falhou na avaliação
/// * `failure_reason` - Motivo da falha na avaliação
/// * `llm_client` - Cliente LLM para gerar a análise
///
/// # Retorno
/// `AgentAnalysis` com recap, blame e improvement, ou erro.
pub async fn analyze_steps(
    diary: &[DiaryEntry],
    original_question: &str,
    failed_answer: &str,
    failure_reason: &str,
    llm_client: Arc<dyn LlmClient>,
) -> Result<AgentAnalysis, LlmError> {
    let start = Instant::now();

    log::info!(
        "🔍 AgentAnalyzer: Iniciando análise de {} entradas do diário",
        diary.len()
    );

    // Formatar diário para análise
    let diary_text = format_diary_for_analysis(diary, original_question);

    // Construir prompts
    let system_prompt = build_system_prompt();
    let user_prompt = build_user_prompt(&diary_text, original_question, failed_answer, failure_reason);

    // Criar prompt para o LLM (usando AgentPrompt existente)
    let prompt = crate::agent::AgentPrompt {
        system: system_prompt,
        user: user_prompt,
        diary: vec![], // Não precisa do diário aqui, já está no user prompt
    };

    // Chamar LLM para gerar análise
    let response = llm_client.generate_answer(&prompt, 0.3).await?;

    // Parse da resposta
    let mut analysis = parse_analysis_response(&response.answer)?;
    analysis.duration_ms = Some(start.elapsed().as_millis());

    log::info!(
        "✅ AgentAnalyzer: Análise concluída em {}ms",
        analysis.duration_ms.unwrap_or(0)
    );
    log::debug!("📊 Recap: {}", analysis.recap.chars().take(100).collect::<String>());
    log::debug!("🎯 Blame: {}", analysis.blame.chars().take(100).collect::<String>());
    log::debug!("💡 Improvement: {}", analysis.improvement.chars().take(100).collect::<String>());

    Ok(analysis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SerpQuery;

    #[test]
    fn test_format_diary_search() {
        let diary = vec![DiaryEntry::Search {
            queries: vec![SerpQuery {
                q: "rust programming".into(),
                ..Default::default()
            }],
            think: "Need to find info about Rust".into(),
            urls_found: 5,
        }];

        let formatted = format_diary_for_analysis(&diary, "What is Rust?");

        assert!(formatted.contains("step 1"));
        assert!(formatted.contains("search"));
        assert!(formatted.contains("rust programming"));
        assert!(formatted.contains("5 URLs"));
    }

    #[test]
    fn test_format_diary_read() {
        let diary = vec![DiaryEntry::Read {
            urls: vec![
                "https://example.com".into(),
                "https://rust-lang.org".into(),
            ],
            think: "Reading relevant pages".into(),
        }];

        let formatted = format_diary_for_analysis(&diary, "Test question");

        assert!(formatted.contains("step 1"));
        assert!(formatted.contains("visit"));
        assert!(formatted.contains("example.com"));
        assert!(formatted.contains("Reading relevant pages"));
    }

    #[test]
    fn test_format_diary_reflect() {
        let diary = vec![DiaryEntry::Reflect {
            questions: vec![
                "What is the main topic?".into(),
                "Who is involved?".into(),
            ],
            think: "Need more information".into(),
        }];

        let formatted = format_diary_for_analysis(&diary, "Original question");

        assert!(formatted.contains("step 1"));
        assert!(formatted.contains("reflect"));
        assert!(formatted.contains("2 gap questions"));
    }

    #[test]
    fn test_format_diary_failed_answer() {
        let diary = vec![DiaryEntry::FailedAnswer {
            answer: "I don't know the answer".into(),
            eval_type: crate::evaluation::EvaluationType::Definitive,
            reason: "Answer is not definitive".into(),
        }];

        let formatted = format_diary_for_analysis(&diary, "Test question");

        assert!(formatted.contains("answer"));
        assert!(formatted.contains("not a good answer"));
        assert!(formatted.contains("Definitive"));
    }

    #[test]
    fn test_format_diary_multiple_entries() {
        let diary = vec![
            DiaryEntry::Search {
                queries: vec![SerpQuery {
                    q: "query 1".into(),
                    ..Default::default()
                }],
                think: "First search".into(),
                urls_found: 3,
            },
            DiaryEntry::Read {
                urls: vec!["https://test.com".into()],
                think: "Reading page".into(),
            },
            DiaryEntry::FailedAnswer {
                answer: "Bad answer".into(),
                eval_type: crate::evaluation::EvaluationType::Completeness,
                reason: "Not complete".into(),
            },
        ];

        let formatted = format_diary_for_analysis(&diary, "Multi-step question");

        assert!(formatted.contains("step 1"));
        assert!(formatted.contains("step 2"));
        assert!(formatted.contains("step 3"));
        assert!(formatted.contains("search"));
        assert!(formatted.contains("visit"));
        assert!(formatted.contains("answer"));
    }

    #[test]
    fn test_format_diary_empty() {
        let diary: Vec<DiaryEntry> = vec![];
        let formatted = format_diary_for_analysis(&diary, "Empty diary test");

        assert!(formatted.contains("<steps>"));
        assert!(formatted.contains("</steps>"));
    }

    #[test]
    fn test_agent_analysis_default() {
        let analysis = AgentAnalysis::default();

        assert!(analysis.recap.is_empty());
        assert!(analysis.blame.is_empty());
        assert!(analysis.improvement.is_empty());
        assert!(analysis.duration_ms.is_none());
    }

    #[test]
    fn test_parse_analysis_response_clean_json() {
        let json = r#"{"recap": "Test recap", "blame": "Test blame", "improvement": "Test improvement"}"#;
        let result = parse_analysis_response(json);

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.recap, "Test recap");
        assert_eq!(analysis.blame, "Test blame");
        assert_eq!(analysis.improvement, "Test improvement");
    }

    #[test]
    fn test_parse_analysis_response_with_markdown() {
        let json = r#"```json
{"recap": "Markdown recap", "blame": "Markdown blame", "improvement": "Markdown improvement"}
```"#;
        let result = parse_analysis_response(json);

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.recap, "Markdown recap");
    }

    #[test]
    fn test_parse_analysis_response_with_code_block() {
        let json = r#"```
{"recap": "Code block", "blame": "blame text", "improvement": "improve text"}
```"#;
        let result = parse_analysis_response(json);

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.recap, "Code block");
    }

    #[test]
    fn test_parse_analysis_response_invalid() {
        let invalid = "This is not JSON";
        let result = parse_analysis_response(invalid);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_analysis_response_missing_fields() {
        let json = r#"{"recap": "Only recap"}"#;
        let result = parse_analysis_response(json);

        // Deve falhar pois campos obrigatórios estão faltando
        assert!(result.is_err());
    }

    #[test]
    fn test_build_system_prompt_contains_rules() {
        let prompt = build_system_prompt();

        assert!(prompt.contains("rules"));
        assert!(prompt.contains("recap"));
        assert!(prompt.contains("blame"));
        assert!(prompt.contains("improvement"));
        assert!(prompt.contains("example"));
    }

    #[test]
    fn test_build_user_prompt() {
        let diary_text = "<steps>test</steps>";
        let prompt = build_user_prompt(diary_text, "Question?", "Bad answer", "Not good");

        assert!(prompt.contains("Question?"));
        assert!(prompt.contains("Bad answer"));
        assert!(prompt.contains("Not good"));
        assert!(prompt.contains("<steps>test</steps>"));
    }
}
