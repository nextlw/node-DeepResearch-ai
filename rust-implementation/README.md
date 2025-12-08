# 🔬 Deep Research CLI

> Agente de pesquisa profunda com IA - Implementação em Rust

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 📋 Índice

- [Instalação](#-instalação)
- [Comandos CLI](#-comandos-cli)
- [Atalhos TUI](#-atalhos-tui)
- [Interface TUI](#-interface-tui)
- [Ações do Agente](#-ações-do-agente)
- [Eventos](#-eventos)
- [Configuração](#-configuração)
- [Exemplos](#-exemplos)

---

## 🚀 Instalação

```bash
# Clonar e compilar
cd rust-implementation
cargo build --release

# Executar
./target/release/deep-research-cli "sua pergunta"
```

### Variáveis de Ambiente Necessárias

```bash
# Criar arquivo .env na raiz do projeto
OPENAI_API_KEY=sua-chave-openai
JINA_API_KEY=sua-chave-jina
```

---

## 💻 Comandos CLI

### `[básico]` Modo Padrão

Executa uma pesquisa direta via linha de comando.

```bash
deep-research-cli "Qual é a população do Brasil?"
```

### `[tui]` Modo Interface Interativa

Abre a interface TUI (Terminal User Interface) para interação visual.

```bash
# Abrir TUI vazia (com campo de input)
deep-research-cli --tui

# Abrir TUI com pergunta pré-definida
deep-research-cli --tui "Qual é a capital da França?"
```

### `[budget]` Controle de Tokens

Define um limite de tokens para a pesquisa.

```bash
deep-research-cli --budget 500000 "pergunta complexa"
```

| Flag       | Tipo  | Padrão    | Descrição               |
| ---------- | ----- | --------- | ----------------------- |
| `--budget` | `u64` | 1.000.000 | Budget máximo de tokens |

### `[compare]` Modo Comparação de Readers

Compara performance entre Jina Reader e Rust+OpenAI para extração de conteúdo.

```bash
# Comparar URLs específicas
deep-research-cli --compare "https://example.com,https://rust-lang.org"
```

### `[compare-live]` Comparação em Tempo Real

Executa pesquisa com comparação Jina vs Rust local durante o processo.

```bash
deep-research-cli --compare-live "Qual é a linguagem de programação mais usada?"
```

---

## ⌨️ Atalhos TUI

### `[input]` Tela de Input

| Tecla       | Ação                          |
| ----------- | ----------------------------- |
| `Enter`     | Iniciar pesquisa              |
| `Esc`       | Sair da aplicação             |
| `Char`      | Digitar caractere             |
| `Backspace` | Apagar caractere anterior     |
| `Delete`    | Apagar caractere atual        |
| `←` / `→`   | Mover cursor esquerda/direita |
| `Home`      | Início da linha               |
| `End`       | Fim da linha                  |
| `↑`         | Histórico anterior            |
| `↓`         | Histórico próximo             |

### `[research]` Tela de Pesquisa

| Tecla       | Ação                     |
| ----------- | ------------------------ |
| `q` / `Esc` | Sair da aplicação        |
| `↑` / `k`   | Scroll para cima (logs)  |
| `↓` / `j`   | Scroll para baixo (logs) |
| `PageUp`    | Scroll 5 linhas acima    |
| `PageDown`  | Scroll 5 linhas abaixo   |

### `[result]` Tela de Resultado

| Tecla       | Ação                       |
| ----------- | -------------------------- |
| `Enter`     | Nova pesquisa (reset)      |
| `q` / `Esc` | Sair da aplicação          |
| `↑` / `k`   | Scroll resposta para cima  |
| `↓` / `j`   | Scroll resposta para baixo |
| `PageUp`    | Page up na resposta        |
| `PageDown`  | Page down na resposta      |
| `Home`      | Início da resposta         |
| `End`       | Fim da resposta            |

---

## 🖥️ Interface TUI

A TUI (Terminal User Interface) oferece uma experiência visual rica para acompanhar a pesquisa em tempo real.

### `[tui-layout]` Layout Visual

```
┌─────────────────────────────────────────────────────────────────┐
│   🔬 DEEP RESEARCH v0.1.0 - Pesquisa Inteligente com IA        │
│                   Pergunta: [query here]                        │
├───────────────────────────────────────┬─────────────────────────┤
│ 💭 Raciocínio do Agente              │ 🎯 Ação Atual           │
│                                       │    Step: 3              │
│ Buscando informações sobre...         │    Ação: SEARCH         │
├───────────────────────────────────────┼─────────────┬───────────┤
│ 📋 Logs                              │ 📊 Stats    │ 👥 Personas│
│                                       │ URLs: 45    │ ● Agente  │
│ [17:30:01] ℹ️ Buscando...            │ Visit: 4    │   S:2 R:3 │
│ [17:30:02] ✅ 72 URLs encontradas    │ Tokens:1234 │           │
│ [17:30:03] ℹ️ Lendo Wikipedia...     │ Tempo: 5.2s │           │
├───────────────────────────────────────┴─────────────┴───────────┤
│ ████████████████████░░░░░░░░░░░░░░░░░░░░░░  40%  Step 4 SEARCH │
└─────────────────────────────────────────────────────────────────┘
```

### `[tui-screens]` Telas Implementadas

| Tela       | Descrição                                                | Componentes                                      |
| ---------- | -------------------------------------------------------- | ------------------------------------------------ |
| `Input`    | Entrada de pergunta com histórico                        | Logo, campo de input, lista de histórico, ajuda  |
| `Research` | Pesquisa em andamento com métricas em tempo real         | Header, raciocínio, logs, stats, personas, gauge |
| `Result`   | Resultado final com resposta, referências e estatísticas | Header, resposta scrollável, refs, URLs, stats   |

### `[tui-components]` Componentes da Interface

#### Header (Todas as Telas)

| Elemento    | Descrição                               |
| ----------- | --------------------------------------- |
| Logo        | `🔬 DEEP RESEARCH v0.1.0`               |
| Status Icon | 🔍 Pesquisando / ✅ Concluído / ❌ Erro |
| Pergunta    | Exibe a query atual (truncada)          |

#### Tela de Input

| Componente      | Funcionalidade                                |
| --------------- | --------------------------------------------- |
| Campo de Input  | Cursor UTF-8, placeholder, borda amarela      |
| Cursor Animado  | `│` com RAPID_BLINK                           |
| Lista Histórico | Últimas 8 perguntas, seleção com ▶, navegável |
| Barra de Ajuda  | Atalhos: Enter, ↑↓, Esc                       |

#### Tela de Pesquisa

| Painel          | Conteúdo                                        |
| --------------- | ----------------------------------------------- |
| 💭 Raciocínio   | Pensamento atual do agente (70% largura)        |
| 🎯 Ação Atual   | Step e ação sendo executada (30% largura)       |
| 📋 Logs         | Lista de eventos com scroll (55% largura)       |
| 📊 Stats        | Steps, URLs, tokens, tempo, sistema (22%)       |
| 👥 Personas     | Stats por persona: S(buscas), R(leituras) (23%) |
| Barra Progresso | Gauge 0-100% com status colorido                |

#### Tela de Resultado

| Seção             | Conteúdo                                      |
| ----------------- | --------------------------------------------- |
| Header            | Status, UUID da sessão, caminhos dos arquivos |
| 📝 Resposta       | Texto completo com scroll vertical            |
| 📚 Referências    | Top 3 referências com URLs clicáveis          |
| 🔗 URLs Visitadas | Top 3 URLs acessadas durante pesquisa         |
| 📊 Estatísticas   | Tokens, URLs, steps, tempos detalhados        |

### `[tui-state]` Estado da Aplicação (App)

```rust
pub struct App {
    // Identificação
    session_id: String,           // UUID único da sessão
    started_at: String,           // Timestamp ISO 8601

    // Tela e Input
    screen: AppScreen,            // Input | Research | Result
    input_text: String,           // Texto sendo digitado
    cursor_pos: usize,            // Posição do cursor (UTF-8 safe)

    // Pesquisa
    question: String,             // Pergunta atual
    current_step: usize,          // Step do agente
    current_action: String,       // Ação sendo executada
    current_think: String,        // Raciocínio do agente

    // Dados Coletados
    logs: VecDeque<LogEntry>,     // Logs da sessão (max 100)
    url_count: usize,             // Total de URLs encontradas
    visited_count: usize,         // URLs visitadas
    visited_urls: Vec<String>,    // Lista de URLs visitadas
    tokens_used: u64,             // Tokens consumidos

    // Resultado
    answer: Option<String>,       // Resposta final
    references: Vec<String>,      // Referências
    is_complete: bool,            // Pesquisa concluída
    error: Option<String>,        // Mensagem de erro

    // Tempos
    start_time: Option<Instant>,  // Início da pesquisa
    total_time_ms: u128,          // Tempo total
    search_time_ms: u128,         // Tempo em buscas
    read_time_ms: u128,           // Tempo em leituras
    llm_time_ms: u128,            // Tempo em LLM

    // UI State
    log_scroll: usize,            // Scroll dos logs
    result_scroll: usize,         // Scroll da resposta
    history: Vec<String>,         // Histórico de perguntas
    history_index: Option<usize>, // Índice no histórico
    history_selected: Option<usize>, // Seleção visual

    // Métricas e Personas
    metrics: SystemMetrics,       // threads, memory_mb, cpu_percent
    personas: HashMap<String, PersonaStats>,

    // Tarefas Paralelas
    active_batches: HashMap<String, ParallelBatch>,
    completed_batches: Vec<ParallelBatch>,
    all_tasks: Vec<ParallelTask>,

    // Persistência
    saved_sessions: Vec<ResearchSession>,
}
```

### `[tui-metrics]` Métricas do Sistema

| Métrica       | Tipo  | Descrição            |
| ------------- | ----- | -------------------- |
| `threads`     | usize | Threads ativas       |
| `memory_mb`   | f64   | Uso de memória em MB |
| `cpu_percent` | f32   | Uso de CPU (%)       |

### `[tui-personas]` Estatísticas de Personas

| Campo       | Tipo   | Descrição              |
| ----------- | ------ | ---------------------- |
| `name`      | String | Nome da persona        |
| `searches`  | usize  | Buscas realizadas      |
| `reads`     | usize  | Leituras realizadas    |
| `answers`   | usize  | Respostas geradas      |
| `tokens`    | u64    | Tokens consumidos      |
| `is_active` | bool   | Se está ativa (● vs ○) |

### `[tui-parallel]` Tarefas Paralelas

#### TaskStatus

| Status      | Símbolo | Descrição             |
| ----------- | ------- | --------------------- |
| `Pending`   | ⏳      | Aguardando início     |
| `Running`   | 🔄      | Em execução           |
| `Completed` | ✅      | Concluída com sucesso |
| `Failed`    | ❌      | Falhou                |

#### ParallelTask

| Campo         | Tipo           | Descrição                |
| ------------- | -------------- | ------------------------ |
| `id`          | String         | ID único da tarefa       |
| `batch_id`    | String         | ID do batch pai          |
| `task_type`   | String         | Tipo (Read, Search)      |
| `description` | String         | URL/descrição processada |
| `data_info`   | String         | Dados alocados           |
| `status`      | TaskStatus     | Status atual             |
| `started_at`  | u128           | Timestamp início (ms)    |
| `elapsed_ms`  | u128           | Tempo de execução        |
| `thread_id`   | Option<String> | ID da thread             |

#### ParallelBatch

| Campo              | Tipo              | Descrição            |
| ------------------ | ----------------- | -------------------- |
| `id`               | String            | ID do batch          |
| `batch_type`       | String            | Tipo do batch        |
| `tasks`            | Vec<ParallelTask> | Tarefas no batch     |
| `started_at`       | u128              | Timestamp início     |
| `total_elapsed_ms` | u128              | Tempo total          |
| `completed`        | usize             | Tarefas completadas  |
| `failed`           | usize             | Tarefas que falharam |

### `[tui-persistence]` Persistência de Sessões

#### Arquivos Salvos

| Tipo | Diretório   | Formato                      | Conteúdo                     |
| ---- | ----------- | ---------------------------- | ---------------------------- |
| JSON | `sessions/` | `YYYYMMDD_HHMMSS_UUID8.json` | Sessão completa serializada  |
| TXT  | `logs/`     | `YYYYMMDD_HHMMSS_UUID8.txt`  | Logs formatados para leitura |

#### ResearchSession (JSON)

```json
{
  "id": "uuid-da-sessao",
  "started_at": "2024-01-15T10:30:00Z",
  "finished_at": "2024-01-15T10:31:45Z",
  "question": "Qual é a população do Brasil?",
  "answer": "A população do Brasil...",
  "references": ["Título - URL", ...],
  "visited_urls": ["https://...", ...],
  "logs": [{"timestamp": "10:30:01", "level": "Info", "message": "..."}],
  "personas": {"Agente": {"searches": 2, "reads": 5, ...}},
  "timing": {"total_ms": 105000, "search_ms": 20000, ...},
  "stats": {"steps": 5, "urls_found": 45, "tokens_used": 18000},
  "success": true,
  "error": null,
  "parallel_batches": [...],
  "all_tasks": [...]
}
```

#### Formato TXT de Logs

```
═══════════════════════════════════════════════════════════════════
 DEEP RESEARCH - Session abc12345
═══════════════════════════════════════════════════════════════════

📅 Início: 2024-01-15T10:30:00Z
❓ Pergunta: Qual é a população do Brasil?
📊 Steps: 5 | URLs: 4 | Tokens: 18000
⏱️  Tempo: 105.0s total | 20.0s busca | 50.0s leitura | 35.0s LLM

───────────────────────────────────────────────────────────────────
 LOGS
───────────────────────────────────────────────────────────────────

[10:30:01] INFO Iniciando pesquisa...
[10:30:05] OK   72 URLs encontradas
[10:30:10] INFO Lendo Wikipedia...
...

───────────────────────────────────────────────────────────────────
 URLs VISITADAS / REFERÊNCIAS / PERSONAS / TAREFAS PARALELAS
───────────────────────────────────────────────────────────────────

═══════════════════════════════════════════════════════════════════
 RESPOSTA FINAL
═══════════════════════════════════════════════════════════════════

A população do Brasil é de aproximadamente...
```

### `[tui-colors]` Esquema de Cores

| Elemento          | Cor            | Uso                        |
| ----------------- | -------------- | -------------------------- |
| Header/Logo       | Cyan           | Título e bordas principais |
| Input Border      | Yellow         | Campo de entrada focado    |
| Cursor            | Yellow         | Cursor piscante            |
| Logs Info         | White          | Mensagens informativas     |
| Logs Success      | Green          | Operações bem sucedidas    |
| Logs Warning      | Yellow         | Avisos                     |
| Logs Error        | Red            | Erros                      |
| Stats             | Magenta        | Painel de estatísticas     |
| Personas Active   | Green          | Persona ativa (●)          |
| Personas Inactive | DarkGray       | Persona inativa (○)        |
| Progress Bar      | Cyan/Green/Red | Baseado no estado          |
| References        | Blue           | Links de referência        |
| URLs Visited      | Cyan           | URLs visitadas             |

### `[tui-input]` Manipulação de Input UTF-8

| Método            | Descrição                             |
| ----------------- | ------------------------------------- |
| `input_char(c)`   | Insere caractere na posição do cursor |
| `input_backspace` | Remove caractere antes do cursor      |
| `input_delete`    | Remove caractere na posição do cursor |
| `cursor_left`     | Move cursor para esquerda             |
| `cursor_right`    | Move cursor para direita              |
| `cursor_home`     | Move cursor para início               |
| `cursor_end`      | Move cursor para fim                  |
| `history_up`      | Navega histórico anterior             |
| `history_down`    | Navega histórico seguinte             |
| `clear_input`     | Limpa todo o input                    |

### `[tui-scroll]` Sistema de Scroll

| Método               | Área     | Descrição             |
| -------------------- | -------- | --------------------- |
| `scroll_up`          | Logs     | Scroll 1 linha acima  |
| `scroll_down`        | Logs     | Scroll 1 linha abaixo |
| `result_scroll_up`   | Resposta | Scroll 1 linha acima  |
| `result_scroll_down` | Resposta | Scroll 1 linha abaixo |
| `result_page_up`     | Resposta | Page up (10 linhas)   |
| `result_page_down`   | Resposta | Page down (10 linhas) |

### `[tui-history]` Sistema de Histórico

| Funcionalidade | Descrição                            |
| -------------- | ------------------------------------ |
| Auto-save      | Perguntas salvas ao iniciar pesquisa |
| Navegação ↑/↓  | Navega pelo histórico no input       |
| Seleção visual | Destaque com ▶ e fundo cinza         |
| Carregamento   | Carrega de sessões JSON anteriores   |
| Limite         | Últimas 50 sessões / 8 visíveis      |

### `[tui-logger]` TuiLogger Wrapper

Helper para enviar eventos formatados:

```rust
impl TuiLogger {
    pub fn info(&self, msg: impl Into<String>);
    pub fn success(&self, msg: impl Into<String>);
    pub fn warning(&self, msg: impl Into<String>);
    pub fn error(&self, msg: impl Into<String>);
    pub fn set_step(&self, step: usize);
    pub fn set_action(&self, action: impl Into<String>);
    pub fn set_think(&self, think: impl Into<String>);
    pub fn set_urls(&self, total: usize, visited: usize);
    pub fn set_tokens(&self, tokens: u64);
    pub fn complete(&self, answer: String, references: Vec<String>);
}
```

---

## 🤖 Ações do Agente

O agente de pesquisa executa ações baseadas em uma máquina de estados.

### `[search]` Buscar na Web

Executa buscas paralelas usando a API Jina.

```
SEARCH: Search the web (only if current URLs are insufficient)
```

**Parâmetros:**

- `queries`: Lista de `SerpQuery` (query, tbs, location)
- `think`: Raciocínio do agente

**Limites:**

- Máximo 5 queries por step
- Execução em paralelo

### `[read]` Ler Conteúdo

Extrai conteúdo de URLs (suporta múltiplos formatos).

```
READ: Read URLs from the available list
```

**Formatos Suportados:**
| Tipo | Extensões |
|------|-----------|
| Web Pages | `.html`, `.htm` |
| PDF | `.pdf` |
| JSON | `.json` |
| XML | `.xml` |
| Texto | `.txt` |
| Markdown | `.md` |

**Parâmetros:**

- `urls`: Lista de URLs para ler
- `think`: Raciocínio do agente

**Limites:**

- Máximo 5 URLs por step
- Execução em paralelo
- URLs já visitadas são ignoradas

### `[reflect]` Refletir/Gerar Sub-perguntas

Gera novas perguntas para expandir a pesquisa.

```
REFLECT: Generate sub-questions (use sparingly)
```

**Parâmetros:**

- `gap_questions`: Lista de novas perguntas
- `think`: Raciocínio do agente

### `[answer]` Responder

Fornece a resposta final com referências.

```
ANSWER: Provide the final answer
```

**Parâmetros:**

- `answer`: Texto da resposta
- `references`: Lista de referências
- `think`: Raciocínio do agente

**Avaliações:**

- Passa por pipeline de avaliação
- Verifica qualidade e precisão
- Pode ser rejeitada se insuficiente

### `[coding]` Executar Código

Executa código em sandbox seguro (reservado).

```
CODING: Execute code for data processing
```

**Parâmetros:**

- `code`: Código para executar
- `think`: Raciocínio do agente

---

## 📡 Eventos

### `[agent-progress]` Eventos de Progresso do Agente

Enviados via callback durante execução.

| Evento               | Descrição           | Dados                                             |
| -------------------- | ------------------- | ------------------------------------------------- |
| `Info(String)`       | Log informativo     | Mensagem                                          |
| `Success(String)`    | Log de sucesso      | Mensagem                                          |
| `Warning(String)`    | Log de aviso        | Mensagem                                          |
| `Error(String)`      | Log de erro         | Mensagem                                          |
| `Step(usize)`        | Atualiza step atual | Número do step                                    |
| `Action(String)`     | Atualiza ação atual | Nome da ação                                      |
| `Think(String)`      | Raciocínio atual    | Texto do raciocínio                               |
| `Urls(usize, usize)` | Contagem de URLs    | (total, visitadas)                                |
| `Tokens(u64)`        | Tokens usados       | Quantidade                                        |
| `Persona`            | Stats de persona    | name, searches, reads, answers, tokens, is_active |
| `VisitedUrl(String)` | URL visitada        | URL                                               |

### `[app-event]` Eventos da Interface TUI

Eventos internos para atualização da UI.

| Evento                         | Descrição                |
| ------------------------------ | ------------------------ |
| `Log(LogEntry)`                | Novo log                 |
| `SetStep(usize)`               | Define step              |
| `SetAction(String)`            | Define ação              |
| `SetThink(String)`             | Define raciocínio        |
| `SetUrlCount(usize)`           | Define total URLs        |
| `SetVisitedCount(usize)`       | Define URLs visitadas    |
| `SetTokens(u64)`               | Define tokens            |
| `SetAnswer(String)`            | Define resposta          |
| `SetReferences(Vec<String>)`   | Define referências       |
| `UpdateMetrics(SystemMetrics)` | Métricas do sistema      |
| `UpdatePersona(PersonaStats)`  | Stats de persona         |
| `SetTimes{...}`                | Tempos detalhados        |
| `Complete`                     | Pesquisa concluída       |
| `Error(String)`                | Erro fatal               |
| `AddVisitedUrl(String)`        | Adiciona URL visitada    |
| `StartBatch{...}`              | Inicia batch de tarefas  |
| `UpdateTask(ParallelTask)`     | Atualiza tarefa paralela |
| `EndBatch{...}`                | Finaliza batch           |

### `[log-level]` Níveis de Log

| Nível     | Símbolo | Uso                   |
| --------- | ------- | --------------------- |
| `Info`    | ℹ️      | Informação geral      |
| `Success` | ✅      | Operação bem sucedida |
| `Warning` | ⚠️      | Aviso                 |
| `Error`   | ❌      | Erro                  |
| `Debug`   | 🔍      | Debug                 |

---

## ⚙️ Configuração

### Estados do Agente

| Estado       | Descrição                       |
| ------------ | ------------------------------- |
| `Processing` | Processando (step, budget_used) |
| `BeastMode`  | Modo forçado (>85% budget)      |
| `Completed`  | Concluído com sucesso           |
| `Failed`     | Falha definitiva                |

### Telas da TUI

| Tela       | Descrição             |
| ---------- | --------------------- |
| `Input`    | Entrada de pergunta   |
| `Research` | Pesquisa em andamento |
| `Result`   | Resultado final       |

### Constantes

```rust
const MAX_URLS_PER_STEP: usize = 5;       // URLs por step
const MAX_REFLECT_PER_STEP: usize = 5;    // Perguntas por reflexão
const BEAST_MODE_THRESHOLD: f64 = 0.85;   // 85% do budget
```

---

## 📚 Exemplos

### Pesquisa Simples

```bash
deep-research-cli "Qual é a capital da França?"
```

### Pesquisa com Budget Limitado

```bash
deep-research-cli --budget 100000 "Explique mecânica quântica"
```

### Interface Interativa

```bash
deep-research-cli --tui
# Digite sua pergunta e pressione Enter
```

### Comparar Métodos de Leitura

```bash
# Comparar extração de conteúdo
deep-research-cli --compare "https://rust-lang.org,https://docs.rs"

# Comparar durante pesquisa
deep-research-cli --compare-live "O que é Rust?"
```

---

## 📊 Saída do Resultado

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 RESULTADO
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✓ Pesquisa concluída com sucesso!

Resposta:
[texto da resposta...]

Referências:
  1. Título - URL
  2. ...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 ESTATÍSTICAS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

⏱️  Tempo total: 12.34s
    - Busca:   2000ms
    - Leitura: 5000ms
    - LLM:     5000ms

🎫 Tokens utilizados:
    - Prompt:     15000
    - Completion: 3000
    - Total:      18000

🔗 URLs visitadas: 5
```

---

## 🗂️ Estrutura de Arquivos

```
rust-implementation/
├── src/
│   ├── main.rs          # CLI e TUI entry point
│   ├── lib.rs           # Biblioteca principal
│   ├── agent/           # Máquina de estados
│   │   ├── mod.rs       # Agente principal
│   │   ├── actions.rs   # Ações do agente
│   │   ├── context.rs   # Contexto de pesquisa
│   │   ├── state.rs     # Estados
│   │   └── permissions.rs
│   ├── search.rs        # Cliente de busca (Jina)
│   ├── llm.rs           # Cliente LLM (OpenAI)
│   ├── tui/             # Interface TUI
│   │   ├── app.rs       # Estado da aplicação
│   │   ├── ui.rs        # Renderização
│   │   └── runner.rs    # Loop principal
│   ├── evaluation/      # Avaliação de respostas
│   ├── personas/        # Personas cognitivas
│   └── utils/           # Utilitários
├── sessions/            # Sessões salvas (JSON)
├── logs/                # Logs de sessões (TXT)
└── Cargo.toml
```

---

## 📝 Licença

MIT License - Veja [LICENSE](LICENSE) para detalhes.
