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
