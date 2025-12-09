name: plano-testes-distribuidos-deepresearch
overview: Este plano divide a responsabilidade de testes do projeto DeepResearch AI entre 3 pessoas, garantindo isolamento de contexto e execução independente. O foco é separado em Lógica do Agente, Domínio/Personas e Performance/Sistema.
todos:

- id: setup-person-1
  content: Configurar ambiente local para Pessoa 1 (Agente) e rodar testes unitários iniciais
  status: pending
- id: setup-person-2
  content: Configurar ambiente local para Pessoa 2 (Personas/Busca) e rodar benchmarks de personas
  status: pending
- id: setup-person-3
  content: Configurar ambiente local para Pessoa 3 (Performance) e rodar bench E2E para baseline
  status: pending

---

# Plano de Testes Distribuídos: DeepResearch AI (Rust)

Este plano divide o trabalho de QA e testes em três frentes independentes. Cada pessoa pode executar seus testes sem bloquear as outras, utilizando comandos específicos do Cargo.

## 👥 Divisão de Responsabilidades

### 👤 Pessoa 1: Lógica do Agente e Estado (Core)

**Foco:** Garantir que o "cérebro" do agente, suas permissões e gerenciamento de estado funcionem corretamente, independente da personalidade ou velocidade.

- **Arquivos Principais:**
  - `src/agent/*` (actions, context, permissions, state)
  - `src/llm.rs` (Integração base com LLM)
- **Comandos de Teste (Isolados):**
  - Testes Unitários: `cargo test agent::`
  - Benchmarks: `cargo bench --bench agent_bench`

### 👤 Pessoa 2: Personas, Busca e Avaliação (Domínio)

**Foco:** Garantir que as diferentes personalidades (Research, Academic, etc.) se comportem como esperado, que a busca retorne resultados relevantes e que o sistema de avaliação julgue corretamente as respostas.

- **Arquivos Principais:**
  - `src/personas/*` (all_personas, orchestrator, traits)
  - `src/search.rs` (Lógica de busca)
  - `src/evaluation/*` (pipeline de avaliação)
- **Comandos de Teste (Isolados):**
  - Testes Unitários Personas: `cargo test personas::`
  - Testes Unitários Busca: `cargo test search::`
  - Testes Unitários Avaliação: `cargo test evaluation::`
  - Benchmarks:
    - `cargo bench --bench personas_bench`
    - `cargo bench --bench search_bench`
    - `cargo bench --bench evaluation_bench`

### 👤 Pessoa 3: Performance, SIMD e End-to-End (Sistema)

**Foco:** Garantir que o sistema seja rápido (otimizações de baixo nível/SIMD), que a CLI funcione e que o fluxo completo (E2E) não quebre sob carga.

- **Arquivos Principais:**
  - `src/performance/*` (simd.rs)
  - `src/main.rs` (CLI e entrypoint)
  - `src/lib.rs` (Integração geral)
- **Comandos de Teste (Isolados):**
  - Testes de Performance: `cargo test performance::`
  - Benchmark SIMD: `cargo bench --bench simd_bench`
  - Benchmark E2E (Fluxo Completo): `cargo bench --bench e2e_bench`
  - Verificação de Build Final: `cargo build --release`

## 🚀 Fluxo de Trabalho Sugerido

1. Cada pessoa deve criar uma branch separada (ex: `test/agent-fix`, `test/persona-update`).
2. Utilizar os comandos de teste isolados listados acima para não esperar a suíte inteira rodar.
3. Reportar falhas categorizadas por área (Agente vs. Domínio vs. Performance).

Vou analisar as ferramentas TypeScript e comparar com as implementações Rust:

[13 tools called]

Agora tenho todas as informações! Aqui está a análise completa:

## 📊 Comparação: Tools TypeScript vs Rust

| Tool TypeScript           | Função                        | Rust Impl                  | Status Pipeline    | Notas                                       |
| ------------------------- | ----------------------------- | -------------------------- | ------------------ | ------------------------------------------- |
| **cosine.ts**             | Similaridade cosseno          | `simd.rs`                  | ✅ **REAL + SIMD** | AVX2 10x mais rápido                        |
| **embeddings.ts**         | Gerar embeddings Jina         | `llm.rs`                   | ✅ **REAL**        | OpenAI `text-embedding-3-small`             |
| **jina-dedup.ts**         | Dedup queries semântica       | `simd.rs` + `agent/mod.rs` | ✅ **REAL**        | SIMD + embeddings                           |
| **evaluator.ts**          | Validação fast-fail           | `evaluation/pipeline.rs`   | ✅ **REAL**        | Definitive/Freshness/Plurality/Completeness |
| **query-rewriter.ts**     | Expandir queries com personas | `personas/`                | ✅ **REAL**        | 7 personas cognitivas                       |
| **jina-search.ts**        | Busca web                     | `search.rs`                | ✅ **REAL**        | JinaClient `search`                         |
| **read.ts**               | Ler conteúdo URL              | `search.rs`                | ✅ **REAL**        | `read_url`, `read_urls_batch`               |
| **jina-rerank.ts**        | Reranking de URLs             | `search.rs`                | ✅ **REAL**        | `rerank` com Jina                           |
| **code-sandbox.ts**       | Execução de código            | `agent/mod.rs`             | ⚠️ **STUB**        | `execute_sandbox` retorna fixo              |
| **error-analyzer.ts**     | Analisar falhas               | ❌ **NÃO IMPL**            | ❌                 | Não existe em Rust                          |
| **build-ref.ts**          | Construir referências         | `agent/mod.rs`             | ⚠️ **PARCIAL**     | `extract_references_from_knowledge` básico  |
| **finalizer.ts**          | Polir resposta final          | ❌ **NÃO IMPL**            | ❌                 | Não existe em Rust                          |
| **reducer.ts**            | Mesclar múltiplas respostas   | ❌ **NÃO IMPL**            | ❌                 | Para multi-agente                           |
| **research-planner.ts**   | Dividir em subproblemas       | ❌ **NÃO IMPL**            | ❌                 | Para multi-agente                           |
| **segment.ts**            | Chunking de texto             | ❌ **NÃO IMPL**            | ❌                 | Usado em build-ref                          |
| **jina-classify-spam.ts** | Classificar spam              | ❌ **NÃO IMPL**            | ❌                 | Filtragem de URLs                           |
| **jina-latechunk.ts**     | Late chunking                 | ❌ **NÃO IMPL**            | ❌                 | Chunking semântico                          |

---

## 📈 Resumo

### ✅ Implementados e na Pipeline (8/17)

```
cosine.ts        → simd.rs               ⚡ SIMD AVX2
embeddings.ts    → llm.rs                🧠 OpenAI embeddings
jina-dedup.ts    → simd.rs + agent       🔄 Dedup semântica
evaluator.ts     → evaluation/pipeline   🔍 Fast-fail validation
query-rewriter.ts→ personas/             🎭 7 personas cognitivas
jina-search.ts   → search.rs             🔍 Jina Search
read.ts          → search.rs             📖 URL reading
jina-rerank.ts   → search.rs             📊 Jina Rerank
```

### ⚠️ Stub/Parcial (2/17)

```
code-sandbox.ts  → agent/mod.rs          ❌ Retorna fixo "Sandbox output"
build-ref.ts     → agent/mod.rs          ⚠️ Básico, sem chunking
```

### ❌ Não Implementados (7/17)

```
error-analyzer.ts     Analisar erros e sugerir melhorias
finalizer.ts          Polir resposta final (editor sênior)
reducer.ts            Mesclar respostas de multi-agente
research-planner.ts   Dividir em subproblemas (multi-agente)
segment.ts            Chunking de texto para referências
jina-classify-spam.ts Classificar URLs spam
jina-latechunk.ts     Late chunking semântico
```

---

## 🎯 Prioridade de Implementação

1. **Alta** - `error-analyzer.ts` → Melhorar loop de retry
2. **Alta** - `segment.ts` + `build-ref.ts` → Referências precisas
3. **Média** - `finalizer.ts` → Qualidade de resposta
4. **Média** - `jina-classify-spam.ts` → Filtrar URLs ruins
5. **Baixa** - `reducer.ts` + `research-planner.ts` → Multi-agente (futuro)

Quer que eu implemente alguma dessas ferramentas faltantes?
