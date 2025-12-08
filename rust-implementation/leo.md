- id: setup-person-2
  content: Configurar ambiente local para Pessoa 2 (Personas/Busca) e rodar benchmarks de personas
  status: pending
- id: setup-person-3
  content: Configurar ambiente local para Pessoa 3 (Performance) e rodar bench E2E para baseline
  status: pending

---

# Plano de Testes Distribuídos: DeepResearch AI (Rust)

Objetivo - micro passos
Quais testes vão ser necessários
Evidências - estabalecer as evidências necessárias
comparação de benchmark com antes e depois

## 👥 Divisão de Responsabilidades

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
