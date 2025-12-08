---
name: Personas Busca Avaliacao
overview: Plano detalhado para implementar e provar que o sistema de Personas, Busca e Avaliação funciona em Rust, com micro-passos, testes, evidências e comparação com TypeScript.
todos:
  - id: define-persona-metrics
    content: Criar struct PersonaExecutionMetrics e modificar trait CognitivePersona para rastrear execuções
    status: pending
  - id: impl-persona-registry
    content: Implementar PersonaRegistry para registro dinâmico de personas com validação
    status: pending
  - id: impl-search-trace
    content: Criar SearchTrace e modificar SearchClient para rastrear fluxo de dados
    status: pending
  - id: impl-eval-trace
    content: Criar EvaluationTrace e modificar EvaluationPipeline para coleta de métricas
    status: pending
  - id: port-eval-prompts
    content: Portar prompts de avaliação do TypeScript (definitive, freshness, completeness, plurality)
    status: pending
  - id: impl-evidence-reports
    content: Criar structs de evidência (PersonaEvidence, SearchEvidence, EvaluationEvidence)
    status: pending
  - id: add-unit-tests
    content: Adicionar testes unitários para todas as funcionalidades novas
    status: pending
  - id: add-integration-tests
    content: Criar testes de integração persona->search->evaluation
    status: pending
  - id: create-comparison-tool
    content: Criar ferramenta para gerar relatório de comparação TS vs Rust
    status: pending
---

# Plano: Sistema de Personas, Busca e Avaliacao (Pessoa 2)

## Objetivo Principal

Criar um sistema de raciocinio passo a passo que seja:

1. **Agnóstico de LLM** - Funciona com qualquer modelo de linguagem
2. **Escalável** - Permite adicionar novas personas seguindo um padrão
3. **Verificável** - Gera evidências de funcionamento em cada etapa
4. **Comparável** - Pode ser medido contra a implementação TypeScript existente

---

## Fase 1: Micro-Passos para Personas

### 1.1 Padronizar o Trait `CognitivePersona`

**Arquivo:** `rust-implementation/src/personas/traits.rs`

**Por que:** O trait atual define a interface mas falta:

- Sistema de logging estruturado
- Coleta de métricas por persona
- Contexto de execução rastreável

**Passos:**

1. Adicionar campo `execution_id: Uuid` no `QueryContext` para rastrear cada execução
2. Criar struct `PersonaExecutionMetrics` com:

   - `persona_name: String`
   - `start_time: Instant`
   - `end_time: Instant`
   - `input_tokens: usize`
   - `output_query: String`
   - `memory_allocated: usize`

3. Modificar `expand_query` para retornar `(SerpQuery, PersonaExecutionMetrics)`

### 1.2 Implementar Sistema de Registro de Personas

**Por que:** Para adicionar novas personas dinamicamente seguindo um padrão.

**Passos:**

1. Criar `PersonaRegistry` em `src/personas/mod.rs`:
   ```rust
   pub struct PersonaRegistry {
       personas: HashMap<String, PersonaBox>,
       schemas: HashMap<String, PersonaSchema>,
   }
   ```

2. Implementar métodos `register()`, `unregister()`, `list_available()`
3. Criar arquivo JSON de configuração para definir personas sem recompilar

### 1.3 Criar Sistema de Validação de Persona

**Por que:** Garantir que novas personas seguem o contrato esperado.

**Passos:**

1. Criar trait `PersonaValidator` com métodos:

   - `validate_name()` - Não vazio, único
   - `validate_focus()` - Descrição mínima de 10 caracteres
   - `validate_weight()` - Entre 0.0 e 2.0
   - `validate_output()` - Query não vazia, formato válido

2. Implementar testes automáticos de conformidade

---

## Fase 2: Micro-Passos para Busca

### 2.1 Implementar Rastreamento de Fluxo de Dados

**Arquivo:** `rust-implementation/src/search.rs`

**Por que:** Necessário saber onde cada dado foi acionado e para onde foi.

**Passos:**

1. Criar `SearchTrace`:
   ```rust
   pub struct SearchTrace {
       pub trace_id: Uuid,
       pub query_origin: QueryOrigin, // Persona, User, Reflection
       pub query_sent: SerpQuery,
       pub api_called: String,
       pub request_timestamp: DateTime<Utc>,
       pub response_timestamp: DateTime<Utc>,
       pub results_count: usize,
       pub bytes_received: usize,
       pub urls_extracted: Vec<String>,
   }
   ```

2. Modificar `SearchClient::search` para retornar `(SearchResult, SearchTrace)`
3. Implementar `SearchTraceCollector` que agrega todas as traces de uma execução

### 2.2 Implementar Métricas de Busca

**Por que:** Comparar performance com TypeScript.

**Passos:**

1. Criar `SearchMetrics`:

   - `latency_p50`, `latency_p95`, `latency_p99`
   - `success_rate`
   - `avg_results_per_query`
   - `cache_hit_rate`
   - `bytes_per_second`

2. Implementar coleta via middleware no `SearchClient`

### 2.3 Implementar Cache de Resultados

**Por que:** Reduzir chamadas repetidas e medir eficiência.

**Passos:**

1. Criar `SearchCache` com TTL configurável
2. Implementar serialização/deserialização de resultados
3. Adicionar métricas de cache hit/miss

---

## Fase 3: Micro-Passos para Avaliação

### 3.1 Implementar Pipeline de Avaliação Observável

**Arquivo:** `rust-implementation/src/evaluation/pipeline.rs`

**Por que:** Necessário saber como cada avaliação funcionou e quanto tempo durou.

**Passos:**

1. Criar `EvaluationTrace`:
   ```rust
   pub struct EvaluationTrace {
       pub trace_id: Uuid,
       pub eval_type: EvaluationType,
       pub question: String,
       pub answer_hash: String, // Não logar resposta inteira
       pub start_time: Instant,
       pub end_time: Instant,
       pub llm_tokens_used: u32,
       pub passed: bool,
       pub confidence: f32,
       pub reasoning_length: usize,
   }
   ```

2. Modificar `EvaluationPipeline::evaluate_single` para coletar traces
3. Criar `EvaluationReport` agregando todas as avaliações

### 3.2 Implementar Determinação de Tipos de Avaliação

**Por que:** O TypeScript tem `evaluateQuestion()` que determina quais avaliações aplicar.

**Passos:**

1. Portar lógica de `src/tools/evaluator.ts` linhas 560-596
2. Implementar `determine_required_evaluations()` sem depender de LLM:

   - Regras baseadas em keywords
   - Fallback para LLM se necessário

3. Criar testes com os mesmos exemplos do TypeScript

### 3.3 Implementar Prompts de Avaliação

**Por que:** Cada tipo de avaliação precisa de prompts específicos.

**Passos:**

1. Criar módulo `src/evaluation/prompts.rs`
2. Portar prompts de `evaluator.ts`:

   - `getDefinitivePrompt` -> linhas 49-154
   - `getFreshnessPrompt` -> linhas 156-219
   - `getCompletenessPrompt` -> linhas 221-310
   - `getPluralityPrompt` -> linhas 312-357
   - `getRejectAllAnswersPrompt` -> linhas 11-46

3. Parametrizar prompts para suportar múltiplos idiomas

---

## Testes Necessários

### Testes Unitários de Personas

| Teste | Descrição | Evidência Esperada |

|-------|-----------|-------------------|

| `test_persona_creation` | Cada persona pode ser instanciada | Todas 7 personas criadas sem panic |

| `test_persona_expand_query` | Cada persona expande query corretamente | Output não vazio, contém termos esperados |

| `test_persona_weight_range` | Peso dentro de 0.0-2.0 | Assert `weight >= 0.0 && weight <= 2.0` |

| `test_persona_thread_safety` | Personas são Send+Sync | Compilação com `par_iter()` funciona |

| `test_persona_uniqueness` | Queries expandidas são únicas entre personas | HashSet de queries tem 7 elementos |

| `test_persona_determinism` | Mesma entrada = mesma saída (exceto random) | 2 execuções com seed fixo são iguais |

### Testes Unitários de Busca

| Teste | Descrição | Evidência Esperada |

|-------|-----------|-------------------|

| `test_search_mock` | Mock retorna resultados esperados | Snippets e URLs presentes |

| `test_hostname_extraction` | Extração de hostname funciona | `"https://example.com/path" -> "example.com"` |

| `test_hostname_boost` | Sites confiáveis recebem boost | Wikipedia > 1.0, random = 1.0 |

| `test_path_boost` | Paths de docs recebem boost | `/docs/` > 1.0, `/about` = 1.0 |

| `test_score_calculation` | Score final calculado corretamente | `final = weight * freq * hostname * path * rerank` |

| `test_search_batch` | Busca em batch funciona | N queries -> N resultados |

### Testes Unitários de Avaliação

| Teste | Descrição | Evidência Esperada |

|-------|-----------|-------------------|

| `test_eval_type_config` | Cada tipo tem config padrão | Todos 5 tipos retornam `EvaluationConfig` |

| `test_freshness_threshold` | Thresholds por tópico corretos | Finance=0.1d, News=1d, Tech=7d, etc |

| `test_eval_result_success` | Resultado de sucesso criado | `passed=true`, `confidence>0` |

| `test_eval_result_failure` | Resultado de falha criado | `passed=false`, `suggestions.len()>0` |

| `test_pipeline_early_fail` | Pipeline para na primeira falha | 2 avaliações executadas, 3 ignoradas |

| `test_determine_eval_types` | Determina tipos corretos | "What is X?" -> `[Definitive]` |

### Testes de Integração

| Teste | Descrição | Evidência Esperada |

|-------|-----------|-------------------|

| `test_persona_to_search` | Personas geram queries que buscam resultados | 7 queries -> pelo menos 1 resultado cada |

| `test_search_to_eval` | Resultados de busca podem ser avaliados | SearchResult -> EvaluationContext válido |

| `test_full_pipeline` | Fluxo completo funciona | Question -> Answer com evidências |

---

## Evidências a Coletar

### 1. Evidências de Personas Funcionando

```rust
pub struct PersonaEvidenceReport {
    pub execution_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub original_query: String,
    pub personas_executed: Vec<PersonaEvidence>,
    pub total_queries_generated: usize,
    pub unique_queries: usize,
    pub total_execution_time: Duration,
    pub memory_peak: usize,
}

pub struct PersonaEvidence {
    pub persona_name: &'static str,
    pub focus: &'static str,
    pub weight: f32,
    pub input_received: String,
    pub output_generated: SerpQuery,
    pub execution_time: Duration,
    pub was_applicable: bool,
}
```

### 2. Evidências de Busca Funcionando

```rust
pub struct SearchEvidenceReport {
    pub execution_id: Uuid,
    pub queries_sent: Vec<SearchQueryEvidence>,
    pub total_api_calls: usize,
    pub total_bytes_transferred: usize,
    pub total_urls_discovered: usize,
    pub unique_hostnames: HashSet<String>,
    pub latency_stats: LatencyStats,
}

pub struct SearchQueryEvidence {
    pub query: SerpQuery,
    pub source_persona: Option<String>,
    pub api_endpoint: String,
    pub request_time: DateTime<Utc>,
    pub response_time: DateTime<Utc>,
    pub http_status: u16,
    pub results_count: usize,
    pub urls_extracted: Vec<UrlEvidence>,
}

pub struct UrlEvidence {
    pub url: String,
    pub hostname: String,
    pub hostname_boost: f32,
    pub path_boost: f32,
    pub final_score: f32,
}
```

### 3. Evidências de Avaliação Funcionando

```rust
pub struct EvaluationEvidenceReport {
    pub execution_id: Uuid,
    pub question: String,
    pub answer_length: usize,
    pub evaluations_required: Vec<EvaluationType>,
    pub evaluations_executed: Vec<EvaluationEvidence>,
    pub final_verdict: bool,
    pub total_evaluation_time: Duration,
    pub total_llm_tokens: u32,
}

pub struct EvaluationEvidence {
    pub eval_type: EvaluationType,
    pub prompt_generated: bool, // Não logar prompt inteiro
    pub prompt_length: usize,
    pub llm_called: bool,
    pub llm_latency: Duration,
    pub llm_tokens_used: u32,
    pub result_passed: bool,
    pub result_confidence: f32,
    pub reasoning_length: usize,
    pub suggestions_count: usize,
}
```

---

## Comparação com TypeScript

### Métricas a Comparar

| Métrica | Como Medir TS | Como Medir Rust | Benchmark |

|---------|---------------|-----------------|-----------|

| Tempo de expansão de queries | `console.time()` em `agent.ts` | `Instant::now()` + criterion | `cargo bench --bench personas_bench` |

| Latência de busca | `Date.now()` em `jina-search.ts` | `SearchTrace.response_time - request_time` | `cargo bench --bench search_bench` |

| Tempo de avaliação | Logs em `evaluator.ts` | `EvaluationTrace.end_time - start_time` | `cargo bench --bench evaluation_bench` |

| Uso de memória | `process.memoryUsage()` | `std::alloc::System` + metrics | `/proc/self/status` em Linux |

| Tokens LLM | `TokenTracker` | `TokenTracker` (já existe) | Comparar totais por execução |

| Throughput | Requests/segundo em produção | `Throughput::Elements` no criterion | Benchmark E2E |

### Formato do Relatório de Comparação

```markdown
# Relatório de Comparação: Rust vs TypeScript

## Resumo Executivo
- Rust X% mais rápido em expansão de queries
- Rust Y% menos memória em pico
- Rust Z% menos tokens LLM (otimização de prompts)

## Personas
| Persona | TS (ms) | Rust (ms) | Diferença |
|---------|---------|-----------|-----------|
| Expert Skeptic | 12.3 | 0.8 | -93% |
| Detail Analyst | 11.1 | 0.7 | -94% |
| ... | ... | ... | ... |

## Busca
| Operação | TS (ms) | Rust (ms) | Diferença |
|----------|---------|-----------|-----------|
| Hostname extraction | 0.5 | 0.001 | -99% |
| Score calculation | 2.1 | 0.003 | -99% |
| ... | ... | ... | ... |

## Avaliação
| Tipo | TS Tokens | Rust Tokens | Diferença |
|------|-----------|-------------|-----------|
| Definitive | 450 | 420 | -7% |
| Freshness | 380 | 350 | -8% |
| ... | ... | ... | ... |
```

---

## Comandos de Execução

```bash
# Testes unitários isolados
cargo test personas:: --release
cargo test search:: --release
cargo test evaluation:: --release

# Benchmarks
cargo bench --bench personas_bench -- --save-baseline rust_v1
cargo bench --bench search_bench -- --save-baseline rust_v1
cargo bench --bench evaluation_bench -- --save-baseline rust_v1

# Gerar relatório de evidências
cargo run --release --bin evidence_report -- --output evidence_report.json

# Comparar com TypeScript
npm run benchmark # No diretório TS
cargo run --release --bin compare_ts_rust -- --ts-results ts_bench.json --rust-results rust_bench.json
```

---

## Checklist de Conclusão

- [ ] Todas as 7 personas passam nos testes unitários
- [ ] Orquestrador executa em paralelo sem deadlock
- [ ] Busca retorna resultados e coleta métricas
- [ ] Avaliação determina tipos corretamente
- [ ] Pipeline de avaliação funciona com early-fail
- [ ] Evidências são coletadas em formato estruturado
- [ ] Benchmarks executam sem erros
- [ ] Relatório de comparação gerado
- [ ] Performance Rust >= TypeScript em todas métricas