//! Renderização da interface TUI

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs, Wrap},
    Frame,
};

use super::app::{App, AppScreen, LogLevel, ReadMethod, TaskStatus, AgentAnalyzerState};

/// Renderiza a interface completa
pub fn render(frame: &mut Frame<'_>, app: &App) {
    match &app.screen {
        AppScreen::Input => render_input_screen(frame, app),
        AppScreen::Research => render_research_screen(frame, app),
        AppScreen::Result => render_result_screen(frame, app),
        AppScreen::Config => render_config_screen(frame, app),
        AppScreen::Benchmarks => render_benchmarks_screen(frame, app),
        AppScreen::InputRequired { question_id, question_type, question, options } => {
            render_input_required_screen(frame, app, question_id, question_type, question, options.as_ref());
        }
    }
}

/// Renderiza a barra de tabs no topo
fn render_tabs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Títulos das tabs com indicadores de estado
    let search_title = match &app.screen {
        AppScreen::Input => "🔍 Pesquisa [Input]",
        AppScreen::Research => {
            if app.is_complete {
                "🔍 Pesquisa [Concluído]"
            } else {
                "🔍 Pesquisa [Em andamento...]"
            }
        }
        AppScreen::Result => "🔍 Pesquisa [Resultado]",
        _ => "🔍 Pesquisa",
    };

    let titles = vec![search_title, "⚙️  Configurações", "📊 Benchmarks"];

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(app.active_tab.index())
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )
        .divider(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));

    frame.render_widget(tabs, area);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE INPUT
// ═══════════════════════════════════════════════════════════════════════════════

fn render_input_screen(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    // Layout vertical com tabs
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),   // Tabs
            Constraint::Min(10),     // Conteúdo
        ])
        .split(area);

    // Renderizar tabs
    render_tabs(frame, app, main_chunks[0]);

    // Layout do conteúdo
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),   // Header/Logo (reduzido)
            Constraint::Length(5),   // Input box
            Constraint::Min(5),      // Histórico
            Constraint::Length(3),   // Ajuda
        ])
        .margin(1)
        .split(main_chunks[1]);

    // Header com logo
    let logo = r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║   🔬 DEEP RESEARCH v0.1.0 - Pesquisa Inteligente com IA      ║
    ╚═══════════════════════════════════════════════════════════════╝"#;

    let header = Paragraph::new(logo)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Campo de input
    let input_block = Block::default()
        .title(" Digite sua pergunta ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    // Texto com cursor (usando índices de caracteres, não bytes)
    let chars: Vec<char> = app.input_text.chars().collect();
    let cursor_pos = app.cursor_pos.min(chars.len());
    let before: String = chars[..cursor_pos].iter().collect();
    let after: String = chars[cursor_pos..].iter().collect();
    let input_text = Line::from(vec![
        Span::raw(before),
        Span::styled("│", Style::default().fg(Color::Yellow).add_modifier(Modifier::RAPID_BLINK)),
        Span::raw(after),
    ]);

    let placeholder = if app.input_text.is_empty() {
        Line::from(vec![
            Span::styled("│", Style::default().fg(Color::Yellow).add_modifier(Modifier::RAPID_BLINK)),
            Span::styled(
                " Ex: Qual é a população do Brasil em 2024?",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        input_text
    };

    let input = Paragraph::new(placeholder)
        .block(input_block)
        .style(Style::default().fg(Color::White));
    frame.render_widget(input, chunks[1]);

    // Histórico de perguntas com seleção visual
    let history_len = app.history.len();
    let history_items: Vec<ListItem<'_>> = app
        .history
        .iter()
        .enumerate()
        .rev()
        .take(50)
        .map(|(original_idx, q)| {
            let truncated = if q.len() > 70 {
                format!("{}...", &q[..67])
            } else {
                q.clone()
            };

            // Verificar se este item está selecionado
            let is_selected = app.history_selected == Some(original_idx);
            let display_num = history_len - original_idx;

            if is_selected {
                ListItem::new(Line::from(vec![
                    Span::styled(" ▶ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{} ", display_num),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        truncated,
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]))
                .style(Style::default().bg(Color::DarkGray))
            } else {
            ListItem::new(Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(format!("{} ", display_num), Style::default().fg(Color::DarkGray)),
                Span::raw(truncated),
            ]))
            }
        })
        .collect();

    let history_title = if app.history_selected.is_some() {
        " 📜 Histórico (Enter para usar, Esc para cancelar) "
    } else {
        " 📜 Histórico (↑/↓ para navegar) "
    };

    let history = List::new(history_items)
        .block(
            Block::default()
                .title(history_title)
                .borders(Borders::ALL)
                .border_style(if app.history_selected.is_some() {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(history, chunks[2]);

    // Barra de ajuda
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" Pesquisar  "),
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Histórico  "),
        Span::styled("Tab/1-2", Style::default().fg(Color::Cyan)),
        Span::raw(" Tabs  "),
        Span::styled("Esc/q", Style::default().fg(Color::Red)),
        Span::raw(" Sair"),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE PESQUISA
// ═══════════════════════════════════════════════════════════════════════════════

fn render_research_screen(frame: &mut Frame<'_>, app: &App) {
    // Layout principal com tabs
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Tabs
            Constraint::Min(10),    // Conteúdo
        ])
        .split(frame.area());

    // Renderizar tabs
    render_tabs(frame, app, main_chunks[0]);

    // Layout do conteúdo
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Header
            Constraint::Length(8),  // Raciocínio e Ação (em cima)
            Constraint::Min(8),     // Logs e Stats (em baixo)
            Constraint::Length(3),  // Barra de progresso
            Constraint::Length(4),  // Input do usuário + ajuda navegação
        ])
        .split(main_chunks[1]);

    render_header(frame, app, chunks[0]);
    render_thinking_panel(frame, app, chunks[1]);
    render_main_content(frame, app, chunks[2]);
    render_progress(frame, app, chunks[3]);
    render_user_input_with_nav(frame, app, chunks[4]);
}

/// Renderiza o campo de input do usuário (sempre visível durante pesquisa)
fn render_user_input(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Indicador de mensagens pendentes na fila
    let queue_indicator = if app.pending_user_messages > 0 {
        format!(" 📨 {} msg(s) na fila ", app.pending_user_messages)
    } else {
        String::new()
    };

    let title = format!(
        " 💬 Enviar mensagem ao agente{} │ Enter: enviar │ Tab: focar ",
        queue_indicator
    );

    // Texto com cursor
    let chars: Vec<char> = app.input_text.chars().collect();
    let cursor_pos = app.cursor_pos.min(chars.len());
    let before: String = chars[..cursor_pos].iter().collect();
    let after: String = chars[cursor_pos..].iter().collect();

    let input_content = if app.input_text.is_empty() && !app.input_focused {
        Line::from(vec![
            Span::styled(
                "Pressione Tab para digitar uma mensagem...",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::raw(before),
            Span::styled(
                if app.input_focused { "│" } else { "" },
                Style::default().fg(Color::Yellow).add_modifier(Modifier::RAPID_BLINK),
            ),
            Span::raw(after),
        ])
    };

    let border_color = if app.input_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let input = Paragraph::new(input_content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(input, area);
}

/// Renderiza o campo de input do usuário com barra de navegação
fn render_user_input_with_nav(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Dividir área entre input e navegação
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Navegação
        ])
        .split(area);

    // Renderizar input normal
    render_user_input(frame, app, chunks[0]);

    // Barra de navegação
    let nav_hint = if app.is_complete {
        Line::from(vec![
            Span::styled("r", Style::default().fg(Color::Green)),
            Span::raw(" Ver resultado  "),
            Span::styled("1-2", Style::default().fg(Color::Cyan)),
            Span::raw(" Tabs  "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" Sair"),
        ])
    } else {
        Line::from(vec![
            Span::styled("1-2", Style::default().fg(Color::Cyan)),
            Span::raw(" Tabs  "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Scroll logs  "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" Sair"),
        ])
    };

    let nav = Paragraph::new(nav_hint)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(nav, chunks[1]);
}

/// Renderiza o header
fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let status_icon = if app.is_complete {
        if app.error.is_some() { "❌" } else { "✅" }
    } else {
        "🔍"
    };

    let title = format!(
        " {} DEEP RESEARCH v0.1.0 │ {} ",
        status_icon,
        if app.is_complete { "Concluído" } else { "Pesquisando..." }
    );

    let header_text = Text::from(vec![
        Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw(" Pergunta: "),
            Span::styled(&app.question, Style::default().fg(Color::Yellow)),
        ]),
    ]);

    let header = Paragraph::new(header_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));

    frame.render_widget(header, area);
}

/// Renderiza o painel de raciocínio e ação atual (em cima)
fn render_thinking_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Raciocínio
            Constraint::Percentage(30),  // Ação atual
        ])
        .split(area);

    // Painel de raciocínio
    let think_display = if app.current_think.is_empty() {
        "Aguardando raciocínio do agente...".to_string()
    } else {
        app.current_think.clone()
    };

    let think = Paragraph::new(think_display)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 💭 Raciocínio do Agente ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(think, chunks[0]);

    // Painel de ação atual
    // Construir lista de steps (completados + atual)
    let mut action_lines: Vec<Line<'_>> = Vec::new();

    // Calcular largura disponível para ações
    let border_width = 2; // bordas do Block
    let padding = 2; // margem de segurança
    let prefix_width = 8; // "🔄 #123 " = aproximadamente 8 caracteres
    let max_action_width = (chunks[1].width as usize)
        .saturating_sub(border_width)
        .saturating_sub(padding)
        .saturating_sub(prefix_width);

    // Mostrar steps completados (últimos 4 no máximo)
    let completed_to_show: Vec<_> = app.completed_steps.iter().rev().take(4).collect();
    for step in completed_to_show.into_iter().rev() {
        let action_short = truncate(&step.action, max_action_width);
        action_lines.push(Line::from(vec![
            Span::styled(
                format!("✅ #{} ", step.step_num),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                action_short,
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    // Mostrar step atual (destacado)
    if app.current_step > 0 {
        let action_short = truncate(&app.current_action, max_action_width);
        action_lines.push(Line::from(vec![
            Span::styled(
                format!("🔄 #{} ", app.current_step),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                action_short,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // Se não houver nada, mostrar aguardando
    if action_lines.is_empty() {
        action_lines.push(Line::from(vec![
            Span::styled(
                "⏳ Aguardando...",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    let action = Paragraph::new(action_lines).block(
        Block::default()
            .title(format!(" 🎯 Steps ({}) ", app.current_step))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(action, chunks[1]);
}

/// Renderiza o conteúdo principal (logs + analyzer + tasks + stats + personas)
fn render_main_content(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Determinar quais painéis especiais estão ativos
    let has_analyzer = app.agent_analyzer.is_active || app.agent_analyzer.last_improvement.is_some();
    let has_sandbox = app.sandbox.is_active || app.sandbox.status == "success" || app.sandbox.status == "error";

    match (has_analyzer, has_sandbox) {
        // Ambos ativos: logs (30%), analyzer (18%), sandbox (18%), tasks (12%), stats (12%), personas (10%)
        (true, true) => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(28),
                    Constraint::Percentage(17),
                    Constraint::Percentage(17),
                    Constraint::Percentage(13),
                    Constraint::Percentage(13),
                    Constraint::Percentage(12),
                ])
                .split(area);

            render_logs(frame, app, chunks[0]);
            render_agent_analyzer(frame, &app.agent_analyzer, chunks[1]);
            render_sandbox(frame, &app.sandbox, chunks[2]);
            render_parallel_tasks(frame, app, chunks[3]);
            render_stats(frame, app, chunks[4]);
            render_personas(frame, app, chunks[5]);
        }
        // Apenas AgentAnalyzer: logs (35%), analyzer (20%), tasks (15%), stats (15%), personas (15%)
        (true, false) => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(35),
                    Constraint::Percentage(20),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                ])
                .split(area);

            render_logs(frame, app, chunks[0]);
            render_agent_analyzer(frame, &app.agent_analyzer, chunks[1]);
            render_parallel_tasks(frame, app, chunks[2]);
            render_stats(frame, app, chunks[3]);
            render_personas(frame, app, chunks[4]);
        }
        // Apenas Sandbox: logs (35%), sandbox (20%), tasks (15%), stats (15%), personas (15%)
        (false, true) => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(35),
                    Constraint::Percentage(20),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                ])
                .split(area);

            render_logs(frame, app, chunks[0]);
            render_sandbox(frame, &app.sandbox, chunks[1]);
            render_parallel_tasks(frame, app, chunks[2]);
            render_stats(frame, app, chunks[3]);
            render_personas(frame, app, chunks[4]);
        }
        // Layout padrão: logs (40%), tasks (20%), stats (20%), personas (20%)
        (false, false) => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                ])
                .split(area);

            render_logs(frame, app, chunks[0]);
            render_parallel_tasks(frame, app, chunks[1]);
            render_stats(frame, app, chunks[2]);
            render_personas(frame, app, chunks[3]);
        }
    }
}

/// Renderiza o painel do AgentAnalyzer (segundo agente)
fn render_agent_analyzer(frame: &mut Frame<'_>, analyzer: &AgentAnalyzerState, area: Rect) {
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Header com status
    if analyzer.is_active {
        lines.push(Line::from(vec![
            Span::styled("🔬 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "ANALISANDO...",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("   {} falhas", analyzer.failures_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("   {} entradas", analyzer.diary_entries),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    } else if analyzer.last_improvement.is_some() {
        lines.push(Line::from(vec![
            Span::styled("✅ ", Style::default().fg(Color::Green)),
            Span::styled(
                "CONCLUÍDO",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
        if let Some(ms) = analyzer.duration_ms {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {}ms", ms),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Mostrar logs do analyzer com wrap
    let visible_height = (area.height as usize).saturating_sub(6);
    let logs_to_show: Vec<_> = analyzer.logs.iter().rev().take(visible_height).collect();

    // Calcular largura disponível (bordas + padding)
    let border_width = 2; // bordas do Block
    let padding = 2; // margem de segurança
    let symbol_width = 4; // emoji + espaços
    let max_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(padding)
        .saturating_sub(symbol_width);
    let continuation_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(padding)
        .saturating_sub(3); // indentação "   "

    for entry in logs_to_show.into_iter().rev() {
        let style = match entry.level {
            LogLevel::Info => Style::default().fg(Color::White),
            LogLevel::Success => Style::default().fg(Color::Green),
            LogLevel::Warning => Style::default().fg(Color::Yellow),
            LogLevel::Error => Style::default().fg(Color::Red),
            LogLevel::Debug => Style::default().fg(Color::DarkGray),
        };

        let msg = &entry.message;
        let msg_char_count = msg.chars().count();

        if msg_char_count <= max_width {
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", entry.level.symbol()), style),
                Span::styled(msg.clone(), style),
            ]));
        } else {
            // Primeira linha com símbolo
            let first_chunk: String = msg.chars().take(max_width.saturating_sub(3)).collect();
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", entry.level.symbol()), style),
                Span::styled(first_chunk, style),
            ]));

            // Linhas de continuação
            let msg_chars: Vec<char> = msg.chars().collect();
            let mut char_idx = max_width.saturating_sub(3).min(msg_chars.len());

            while char_idx < msg_chars.len() {
                let remaining_chars: Vec<char> = msg_chars[char_idx..].iter().cloned().collect();
                let chunk: String = remaining_chars
                    .iter()
                    .take(continuation_width.min(remaining_chars.len()))
                    .collect();

                if chunk.is_empty() {
                    break;
                }

                lines.push(Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(chunk.clone(), style),
                ]));

                char_idx += chunk.chars().count();

                // Limite de segurança para evitar loops infinitos
                if lines.len() > visible_height * 2 {
                    break;
                }
            }
        }
    }

    // Se tiver resultado, mostrar resumo
    if let Some(improvement) = &analyzer.last_improvement {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("─── Aplicado ───", Style::default().fg(Color::DarkGray)),
        ]));

        // Mostrar melhoria com wrap
        let improvement_chars: Vec<char> = improvement.chars().collect();
        let chunks: Vec<String> = improvement_chars
            .chunks(max_width)
            .map(|c| c.iter().collect())
            .collect();

        for chunk in chunks.iter().take(3) {
            let truncated = if chunk.chars().count() > max_width {
                chunk.chars().take(max_width.saturating_sub(3)).collect::<String>() + "..."
            } else {
                chunk.clone()
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {}", truncated), Style::default().fg(Color::Cyan)),
            ]));
        }
        if chunks.len() > 3 {
            lines.push(Line::from(vec![
                Span::styled(" ...", Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    let border_color = if analyzer.is_active {
        Color::Yellow
    } else if analyzer.last_improvement.is_some() {
        Color::Green
    } else {
        Color::DarkGray
    };

    let content = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(" 🔬 Analyzer ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );

    frame.render_widget(content, area);
}

/// Renderiza o painel do Sandbox (execução de código)
fn render_sandbox(frame: &mut Frame<'_>, sandbox: &super::app::SandboxState, area: Rect) {
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Determinar emoji da linguagem
    let lang_emoji = if sandbox.language == "Python" { "🐍" } else { "📜" };
    let lang_color = if sandbox.language == "Python" { Color::Yellow } else { Color::Cyan };

    // Header com status
    let (status_icon, status_text, status_color) = match sandbox.status.as_str() {
        "generating" => ("⏳", "GERANDO...", Color::Yellow),
        "executing" => ("🔄", "EXECUTANDO...", Color::Cyan),
        "success" => ("✅", "SUCESSO", Color::Green),
        "error" => ("❌", "FALHOU", Color::Red),
        _ => ("⏸️", "IDLE", Color::DarkGray),
    };

    if sandbox.is_active || sandbox.status == "success" || sandbox.status == "error" {
        // Linguagem no topo
        if !sandbox.language.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("{} ", lang_emoji), Style::default().fg(lang_color)),
                Span::styled(
                    sandbox.language.clone(),
                    Style::default().fg(lang_color).add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
            Span::styled(
                status_text,
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Tentativas
        if sandbox.max_attempts > 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {}/{} tentativas", sandbox.current_attempt, sandbox.max_attempts),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        // Tempo
        if sandbox.execution_time_ms > 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {}ms", sandbox.execution_time_ms),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else if sandbox.is_active {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   timeout: {}ms", sandbox.timeout_ms),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        lines.push(Line::from(""));

        // Problema truncado
        if !sandbox.problem.is_empty() {
            let problem_preview = if sandbox.problem.len() > 30 {
                format!("{}...", &sandbox.problem[..27])
            } else {
                sandbox.problem.clone()
            };
            lines.push(Line::from(vec![
                Span::styled("📝 ", Style::default().fg(Color::White)),
                Span::styled(problem_preview, Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(""));

        // Preview do código (truncado)
        if !sandbox.code_preview.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("💻 Código:", Style::default().fg(Color::Cyan)),
            ]));

            // Mostrar primeiras linhas do código
            let code_lines: Vec<&str> = sandbox.code_preview.lines().take(4).collect();
            for code_line in code_lines {
                let line_preview = if code_line.len() > 25 {
                    format!("  {}...", &code_line[..22])
                } else {
                    format!("  {}", code_line)
                };
                lines.push(Line::from(vec![
                    Span::styled(line_preview, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        // Output ou Erro
        if let Some(output) = &sandbox.output {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("📤 Output:", Style::default().fg(Color::Green)),
            ]));
            let out_preview = if output.len() > 50 { format!("{}...", &output[..47]) } else { output.clone() };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}", out_preview), Style::default().fg(Color::White)),
            ]));
        }

        if let Some(error) = &sandbox.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("❌ Erro:", Style::default().fg(Color::Red)),
            ]));
            let err_preview = if error.len() > 50 { format!("{}...", &error[..47]) } else { error.clone() };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}", err_preview), Style::default().fg(Color::Red)),
            ]));
        }
    } else {
        // Estado idle
        lines.push(Line::from(vec![
            Span::styled("⏸️ ", Style::default().fg(Color::DarkGray)),
            Span::styled("IDLE", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Aguardando execução",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "de código...",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    let border_color = match sandbox.status.as_str() {
        "generating" | "executing" => Color::Yellow,
        "success" => Color::Green,
        "error" => Color::Red,
        _ => Color::DarkGray,
    };

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" 🖥️ Sandbox ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(content, area);
}

/// Renderiza o painel de tarefas paralelas em execução
fn render_parallel_tasks(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Calcular largura disponível para URLs
    let border_width = 2; // bordas do Block
    let padding = 2; // margem de segurança
    let icon_width = 4; // "  ⏳ " = 4 caracteres
    let method_width = 4; // "[J] " = 4 caracteres
    let progress_width = 5; // " 100%" = 5 caracteres (máximo)
    let max_url_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(padding)
        .saturating_sub(icon_width)
        .saturating_sub(method_width)
        .saturating_sub(progress_width);

    // Iterar sobre batches ativos
    for (batch_id, batch) in &app.active_batches {
        // Calcular progresso geral do batch
        let total_progress: u32 = batch.tasks.iter().map(|t| t.progress as u32).sum();
        let avg_progress = if batch.tasks.is_empty() {
            0
        } else {
            total_progress / batch.tasks.len() as u32
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("⚡ {} ", batch.batch_type),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}] ", &batch_id[..8]),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{}%", avg_progress),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Barra de progresso do batch
        let progress_bar = create_progress_bar(avg_progress as u8, 15);
        lines.push(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled(progress_bar, Style::default().fg(Color::Cyan)),
        ]));

        // Mostrar tarefas do batch
        for task in &batch.tasks {
            let (icon, color) = match &task.status {
                TaskStatus::Pending => ("⏳", Color::DarkGray),
                TaskStatus::Running => ("🔄", Color::Yellow),
                TaskStatus::Completed => ("✅", Color::Green),
                TaskStatus::Failed(_) => ("❌", Color::Red),
            };

            // Método de leitura com cor
            let method_display = match task.read_method {
                ReadMethod::Jina => ("J", Color::Blue),      // Jina API
                ReadMethod::RustLocal => ("R", Color::Magenta), // Rust+LLM
                ReadMethod::FileRead => ("F", Color::Cyan),  // File
                ReadMethod::Unknown => ("?", Color::DarkGray),
            };

            // Truncar URL para caber na largura disponível
            let url_display = if task.description.chars().count() > max_url_width {
                let truncated: String = task.description.chars().take(max_url_width.saturating_sub(3)).collect();
                format!("{}...", truncated)
            } else {
                task.description.clone()
            };

            // Progresso individual
            let progress_str = if task.progress < 100 && matches!(task.status, TaskStatus::Running) {
                format!(" {}%", task.progress)
            } else {
                String::new()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                Span::styled(
                    format!("[{}] ", method_display.0),
                    Style::default().fg(method_display.1).add_modifier(Modifier::BOLD),
                ),
                Span::styled(url_display, Style::default().fg(color)),
                Span::styled(progress_str, Style::default().fg(Color::Cyan)),
            ]));

            // Mostrar bytes processados se disponível
            if task.bytes_processed > 0 || task.bytes_total > 0 {
                let bytes_info = if task.bytes_total > 0 {
                    format!("{}/{} bytes",
                        format_bytes(task.bytes_processed),
                        format_bytes(task.bytes_total))
                } else {
                    format!("{} bytes", format_bytes(task.bytes_processed))
                };
                lines.push(Line::from(vec![
                    Span::styled("      └─ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(bytes_info, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        lines.push(Line::from(""));
    }

    // Se não houver batches ativos, mostrar histórico dos completados
    if lines.is_empty() && !app.completed_batches.is_empty() {
        // Mostrar resumo de todos os batches completados
        let total_tasks: usize = app.completed_batches.iter().map(|b| b.tasks.len()).sum();
        let total_success: usize = app.completed_batches.iter().map(|b| b.completed).sum();
        let total_failed: usize = app.completed_batches.iter().map(|b| b.failed).sum();
        let total_time: u128 = app.completed_batches.iter().map(|b| b.total_elapsed_ms).sum();

        lines.push(Line::from(vec![
            Span::styled("📊 Histórico", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} batches", app.completed_batches.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} tarefas", total_tasks),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(format!("  ✅{}", total_success), Style::default().fg(Color::Green)),
            Span::styled(format!(" ❌{}", total_failed), Style::default().fg(Color::Red)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:.1}s total", total_time as f64 / 1000.0),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        // Mostrar últimos 3 batches
        lines.push(Line::from(""));
        for batch in app.completed_batches.iter().rev().take(3) {
            lines.push(Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{}ms", batch.total_elapsed_ms),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!(" ({}t)", batch.tasks.len()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    // Se não houver nada, mostrar legenda
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("⏳ Aguardando", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   tarefas...", Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Sempre mostrar legenda no final
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("─────────────", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("[J]", Style::default().fg(Color::Blue)),
        Span::styled(" Jina ", Style::default().fg(Color::DarkGray)),
        Span::styled("[R]", Style::default().fg(Color::Magenta)),
        Span::styled(" Rust", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("[F]", Style::default().fg(Color::Cyan)),
        Span::styled(" File", Style::default().fg(Color::DarkGray)),
    ]));

    let content = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" ⚡ Paralelo ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );

    frame.render_widget(content, area);
}

/// Cria uma barra de progresso ASCII
fn create_progress_bar(progress: u8, width: usize) -> String {
    let filled = (progress as usize * width) / 100;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Formata bytes para exibição legível
fn format_bytes(bytes: usize) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1}MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}KB", bytes as f64 / 1_000.0)
    } else {
        format!("{}B", bytes)
    }
}

/// Renderiza a área de logs (com wrap de mensagens longas)
fn render_logs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;

    // Calcular larguras disponíveis com mais precisão
    // Bordas do Block: 2 caracteres (1 de cada lado)
    // Timestamp: "[HH:MM:SS] " = 10 caracteres
    // Símbolo do nível: emoji pode ter 2-4 caracteres, usar 4 para segurança
    // Espaços: 2
    let timestamp_width = 10; // "[HH:MM:SS] "
    let symbol_width = 4; // emoji + espaço
    let border_width = 2; // bordas do Block
    let padding = 2; // margem de segurança

    // Largura disponível para a mensagem na primeira linha
    let max_msg_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(timestamp_width)
        .saturating_sub(symbol_width)
        .saturating_sub(padding);

    // Largura para linhas de continuação (sem timestamp/símbolo, apenas indentação)
    let indent_width = 7; // "     ↳ " = 7 caracteres
    let continuation_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(indent_width)
        .saturating_sub(padding);

    // Construir lista de linhas com wrap
    let mut items: Vec<ListItem<'_>> = Vec::new();
    let mut line_count = 0;

    for entry in app.logs.iter().skip(app.log_scroll) {
        if line_count >= visible_height {
            break;
        }

        let style = match entry.level {
            LogLevel::Info => Style::default().fg(Color::White),
            LogLevel::Success => Style::default().fg(Color::Green),
            LogLevel::Warning => Style::default().fg(Color::Yellow),
            LogLevel::Error => Style::default().fg(Color::Red),
            LogLevel::Debug => Style::default().fg(Color::DarkGray),
        };

        let msg = &entry.message;

        // Função auxiliar para truncar string respeitando limite de largura
        fn truncate_to_width(s: &str, max_width: usize) -> String {
            if s.chars().count() <= max_width {
                s.to_string()
            } else {
                s.chars().take(max_width.saturating_sub(3)).collect::<String>() + "..."
            }
        }

        if msg.chars().count() <= max_msg_width {
            // Mensagem cabe em uma linha
            let truncated_msg = truncate_to_width(msg, max_msg_width);
            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", entry.level.symbol()), style),
                Span::styled(truncated_msg, style),
            ]);
            items.push(ListItem::new(content));
            line_count += 1;
        } else {
            // Mensagem precisa de wrap - primeira linha com timestamp/símbolo
            let first_chunk = truncate_to_width(msg, max_msg_width);
            let first_line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", entry.level.symbol()), style),
                Span::styled(first_chunk, style),
            ]);
            items.push(ListItem::new(first_line));
            line_count += 1;

            // Linhas de continuação (indentadas)
            let msg_chars: Vec<char> = msg.chars().collect();
            let mut char_idx = max_msg_width.min(msg_chars.len());

            while char_idx < msg_chars.len() && line_count < visible_height {
                let remaining_chars: Vec<char> = msg_chars[char_idx..].iter().cloned().collect();
                let chunk: String = remaining_chars
                    .iter()
                    .take(continuation_width.min(remaining_chars.len()))
                    .collect();

                // Truncar se necessário
                let truncated_chunk = if chunk.chars().count() > continuation_width {
                    truncate_to_width(&chunk, continuation_width)
                } else {
                    chunk
                };

                let continuation_line = Line::from(vec![
                    Span::styled("     ", Style::default()), // Indentação
                    Span::styled("↳ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(truncated_chunk.clone(), style),
                ]);
                items.push(ListItem::new(continuation_line));

                char_idx += truncated_chunk.chars().count();
                line_count += 1;

                // Se truncamos, parar para evitar loop infinito
                if truncated_chunk.ends_with("...") {
                    break;
                }
            }
        }
    }

    let scroll_info = if app.logs.len() > visible_height {
        format!(
            " [{}/{}]",
            app.log_scroll + 1,
            app.logs.len().saturating_sub(visible_height) + 1
        )
    } else {
        String::new()
    };

    let logs = List::new(items).block(
        Block::default()
            .title(format!(" 📋 Logs{} ", scroll_info))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(logs, area);
}

/// Renderiza o painel de estatísticas
fn render_stats(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let elapsed = app.elapsed_secs();

    let stats_text = Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(" Step:      "),
            Span::styled(
                format!("{}", app.current_step),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw(" URLs:      "),
            Span::styled(format!("{}", app.url_count), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw(" Visitadas: "),
            Span::styled(
                format!("{}", app.visited_count),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Tokens:    "),
            Span::styled(
                format!("{}", app.tokens_used),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::raw(" Tempo:     "),
            Span::styled(format!("{:.1}s", elapsed), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ═══ Sistema ═══ ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::raw(" Threads:   "),
            Span::styled(
                format!("{}", app.metrics.threads),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw(" RAM:       "),
            Span::styled(
                format!("{:.1}MB", app.metrics.memory_mb),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ]);

    let stats = Paragraph::new(stats_text)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 📊 Stats ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );

    frame.render_widget(stats, area);
}

/// Renderiza o painel de personas
fn render_personas(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut lines = vec![Line::from("")];

    // Calcular largura disponível para nomes
    let border_width = 2;
    let padding = 2;
    let indicator_width = 4; // " ● "
    let max_name_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(padding)
        .saturating_sub(indicator_width);

    if app.personas.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            " Aguardando...",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for (name, stats) in &app.personas {
            let indicator = if stats.is_active { "●" } else { "○" };
            let color = if stats.is_active {
                Color::Green
            } else {
                Color::DarkGray
            };

            // Truncar nome se necessário
            let name_display = if name.chars().count() > max_name_width {
                name.chars().take(max_name_width.saturating_sub(3)).collect::<String>() + "..."
            } else {
                name.clone()
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", indicator), Style::default().fg(color)),
                Span::styled(
                    name_display,
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("   S:"),
                Span::styled(format!("{} ", stats.searches), Style::default().fg(Color::Cyan)),
                Span::raw("R:"),
                Span::styled(format!("{} ", stats.reads), Style::default().fg(Color::Yellow)),
                Span::raw("A:"),
                Span::styled(format!("{}", stats.answers), Style::default().fg(Color::Green)),
            ]));
        }
    }

    let personas = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 👥 Personas ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(personas, area);
}

/// Renderiza a barra de progresso
fn render_progress(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let progress = app.progress();

    // Calcular largura disponível para o label
    let border_width = 2;
    let padding = 4;
    let max_label_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(padding);

    let label = if app.is_complete {
        if app.error.is_some() {
            "Erro!".to_string()
        } else {
            "Concluído!".to_string()
        }
    } else {
        let base_label = format!("Step {} - {}", app.current_step, &app.current_action);
        // Truncar label se necessário
        if base_label.chars().count() > max_label_width {
            base_label.chars().take(max_label_width.saturating_sub(3)).collect::<String>() + "..."
        } else {
            base_label
        }
    };

    let color = if app.is_complete {
        if app.error.is_some() {
            Color::Red
        } else {
            Color::Green
        }
    } else {
        Color::Cyan
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        )
        .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
        .percent((progress * 100.0) as u16)
        .label(label);

    frame.render_widget(gauge, area);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE INPUT REQUERIDO (PERGUNTA DO AGENTE)
// ═══════════════════════════════════════════════════════════════════════════════

/// Renderiza tela quando o agente precisa de input do usuário
///
/// Compatível com OpenAI Responses API (input_required state).
fn render_input_required_screen(
    frame: &mut Frame<'_>,
    app: &App,
    question_id: &str,
    question_type: &str,
    question: &str,
    options: Option<&Vec<String>>,
) {
    let area = frame.area();

    // Layout vertical
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Length(3),   // Status
            Constraint::Min(5),      // Pergunta e opções
            Constraint::Length(4),   // Input
            Constraint::Length(2),   // Ajuda
        ])
        .margin(2)
        .split(area);

    // Header
    let header_text = format!("❓ ENTRADA REQUERIDA - {}", question_type.to_uppercase());
    let header = Paragraph::new(header_text)
        .style(Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" 🤖 Agente aguardando resposta "));
    frame.render_widget(header, chunks[0]);

    // Status
    let status = Paragraph::new(format!(
        "ID: {} │ Step: {} │ Tokens: {}",
        &question_id[..8],
        app.current_step,
        app.tokens_used
    ))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(status, chunks[1]);

    // Pergunta e opções
    let mut question_lines = vec![
        Line::from(vec![
            Span::styled("Pergunta: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(question, Style::default().fg(Color::White))),
        Line::from(""),
    ];

    // Adicionar opções se houver
    if let Some(opts) = options {
        question_lines.push(Line::from(vec![
            Span::styled("Opções:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        for (i, opt) in opts.iter().enumerate() {
            question_lines.push(Line::from(vec![
                Span::styled(format!("  {}. ", i + 1), Style::default().fg(Color::Yellow)),
                Span::styled(opt, Style::default().fg(Color::White)),
            ]));
        }
    }

    let question_widget = Paragraph::new(question_lines)
        .wrap(Wrap { trim: false })
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Pergunta do Agente "));
    frame.render_widget(question_widget, chunks[2]);

    // Input área
    let input_title = if options.is_some() {
        " Digite número ou resposta "
    } else {
        " Sua resposta "
    };

    let input_text = if app.input_text.is_empty() {
        vec![
            Span::styled("█", Style::default().fg(Color::Green).add_modifier(Modifier::SLOW_BLINK)),
        ]
    } else {
        vec![
            Span::styled(&app.input_text, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Green).add_modifier(Modifier::SLOW_BLINK)),
        ]
    };

    let input_widget = Paragraph::new(Line::from(input_text))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(input_title));
    frame.render_widget(input_widget, chunks[3]);

    // Ajuda
    let help = Paragraph::new("Enter: Enviar │ Esc: Cancelar (continua sem resposta)")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help, chunks[4]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE RESULTADO
// ═══════════════════════════════════════════════════════════════════════════════

fn render_result_screen(frame: &mut Frame<'_>, app: &App) {
    // Layout principal com tabs
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Tabs
            Constraint::Min(10),    // Conteúdo
        ])
        .split(frame.area());

    // Renderizar tabs
    render_tabs(frame, app, main_chunks[0]);

    // Layout do conteúdo
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),   // Header com UUID e JSON path
            Constraint::Min(5),      // Resposta
            Constraint::Length(4),   // Referências (reduzido)
            Constraint::Length(4),   // URLs visitadas (reduzido)
            Constraint::Length(3),   // Stats finais (reduzido)
            Constraint::Length(3),   // Input para follow-up
            Constraint::Length(2),   // Ajuda
        ])
        .split(main_chunks[1]);

    // Header com UUID e caminhos dos arquivos
    let session_id_short = &app.session_id[..8];
    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![
        Span::styled(
            " ✅ PESQUISA CONCLUÍDA ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
            Span::raw("  │  "),
            Span::styled("🆔 ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                session_id_short,
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled(" 💾 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("sessions/*_{}.json", session_id_short),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  │  "),
            Span::styled("📄 ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("logs/*_{}.txt", session_id_short),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(header, chunks[0]);

    // Resposta com scroll
    let answer_text = app.answer.as_deref().unwrap_or("Sem resposta");
    let answer_lines: Vec<&str> = answer_text.lines().collect();
    let total_lines = answer_lines.len();
    let visible_height = (chunks[1].height as usize).saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_pos = app.result_scroll.min(max_scroll);

    let scroll_info = if total_lines > visible_height {
        format!(" [{}/{}] ", scroll_pos + 1, max_scroll + 1)
    } else {
        String::new()
    };

    let answer = Paragraph::new(answer_text)
        .wrap(Wrap { trim: false })
        .scroll((scroll_pos as u16, 0))
        .block(
            Block::default()
                .title(format!(" 📝 Resposta{} ", scroll_info))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(answer, chunks[1]);

    // Scrollbar visual para resposta
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(max_scroll)
            .position(scroll_pos);

        // Área interna (sem bordas)
        let scrollbar_area = Rect {
            x: chunks[1].x + chunks[1].width - 1,
            y: chunks[1].y + 1,
            width: 1,
            height: chunks[1].height.saturating_sub(2),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // Referências (URLs das fontes)
    let refs_items: Vec<ListItem<'_>> = app
        .references
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, r)| {
            let truncated = truncate(r, (chunks[2].width as usize).saturating_sub(8));
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {}. ", i + 1),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(truncated, Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)),
            ]))
        })
        .collect();

    let refs = List::new(refs_items).block(
        Block::default()
            .title(format!(" 📚 Referências ({}) - copie para acessar ", app.references.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(refs, chunks[2]);

    // URLs visitadas
    let urls_items: Vec<ListItem<'_>> = app
        .visited_urls
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, url)| {
            let truncated = truncate(url, (chunks[3].width as usize).saturating_sub(8));
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {}. ", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(truncated, Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
            ]))
        })
        .collect();

    let urls = List::new(urls_items).block(
        Block::default()
            .title(format!(" 🔗 URLs Visitadas ({}) ", app.visited_urls.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(urls, chunks[3]);

    // Stats finais (com tempos detalhados)
    let stats_text = Text::from(vec![
        // Linha 1: Tokens e URLs
        Line::from(vec![
            Span::styled(" 🎫 ", Style::default().fg(Color::Magenta)),
            Span::raw(format!("{} tokens", app.tokens_used)),
            Span::raw("  │  "),
            Span::styled("🔗 ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} URLs", app.visited_count)),
            Span::raw("  │  "),
            Span::styled("📊 ", Style::default().fg(Color::Green)),
            Span::raw(format!("{} steps", app.current_step)),
        ]),
        // Linha 2: Tempos detalhados
        Line::from(vec![
            Span::styled(" ⏱️  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:.1}s", app.total_time_ms as f64 / 1000.0),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" total  │  "),
            Span::styled("🔍", Style::default().fg(Color::Blue)),
            Span::raw(format!(" {:.1}s", app.search_time_ms as f64 / 1000.0)),
            Span::raw("  │  "),
            Span::styled("📖", Style::default().fg(Color::Green)),
            Span::raw(format!(" {:.1}s", app.read_time_ms as f64 / 1000.0)),
            Span::raw("  │  "),
            Span::styled("🤖", Style::default().fg(Color::Magenta)),
            Span::raw(format!(" {:.1}s", app.llm_time_ms as f64 / 1000.0)),
        ]),
    ]);

    let stats = Paragraph::new(stats_text)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" 📊 Estatísticas ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
    frame.render_widget(stats, chunks[4]);

    // Campo de input para follow-up
    render_followup_input(frame, app, chunks[5]);

    // Ajuda (com mensagem de clipboard se houver)
    let clipboard_msg = app.clipboard_message.as_deref().unwrap_or("");
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" Focar input  "),
        Span::styled("↑↓", Style::default().fg(Color::DarkGray)),
        Span::raw(" Scroll  "),
        Span::styled("c", Style::default().fg(Color::Cyan)),
        Span::raw(" Copiar  "),
        Span::styled("r", Style::default().fg(Color::Magenta)),
        Span::raw(" Logs  "),
        Span::styled("q", Style::default().fg(Color::Red)),
        Span::raw(" Sair"),
        if !clipboard_msg.is_empty() {
            Span::styled(format!("  {}", clipboard_msg), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("")
        },
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[6]);
}

/// Renderiza o campo de input para follow-up na tela de resultado
fn render_followup_input(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let title = " 💬 Continuar conversa │ Tab: focar │ Enter: enviar ";

    // Texto com cursor
    let chars: Vec<char> = app.input_text.chars().collect();
    let cursor_pos = app.cursor_pos.min(chars.len());
    let before: String = chars[..cursor_pos].iter().collect();
    let after: String = chars[cursor_pos..].iter().collect();

    let input_content = if app.input_text.is_empty() && !app.input_focused {
        Line::from(vec![
            Span::styled(
                "Digite uma pergunta de follow-up ou nova pesquisa...",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::raw(before),
            Span::styled(
                if app.input_focused { "│" } else { "" },
                Style::default().fg(Color::Green).add_modifier(Modifier::RAPID_BLINK),
            ),
            Span::raw(after),
        ])
    };

    let border_color = if app.input_focused {
        Color::Green
    } else {
        Color::DarkGray
    };

    let input = Paragraph::new(input_content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(input, area);
}

/// Trunca uma string para o tamanho máximo
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE BENCHMARKS
// ═══════════════════════════════════════════════════════════════════════════════

fn render_benchmarks_screen(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    // Layout principal
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),   // Tabs
            Constraint::Length(3),   // Header
            Constraint::Min(10),      // Conteúdo (lista de benchmarks + resultado)
            Constraint::Length(2),   // Ajuda
        ])
        .split(area);

    // Tabs no topo
    render_tabs(frame, app, chunks[0]);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " 📊 BENCHMARKS DE PERFORMANCE ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " │ Execute benchmarks para medir performance do sistema",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, chunks[1]);

    // Área de conteúdo dividida horizontalmente
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Lista de benchmarks
            Constraint::Percentage(60),   // Resultado/logs
        ])
        .margin(1)
        .split(chunks[2]);

    // Lista de benchmarks disponíveis
    render_benchmarks_list(frame, app, content_chunks[0]);

    // Resultado/logs do benchmark
    render_benchmark_result(frame, app, content_chunks[1]);

    // Ajuda
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Selecionar  "),
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::raw(" Executar  "),
        Span::styled("Tab/1-3", Style::default().fg(Color::Cyan)),
        Span::raw(" Tabs  "),
        Span::styled("q/Esc", Style::default().fg(Color::Red)),
        Span::raw(" Sair"),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

/// Renderiza a lista de benchmarks disponíveis
fn render_benchmarks_list(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let benchmarks = &app.benchmarks;
    let items: Vec<ListItem<'_>> = benchmarks
        .available
        .iter()
        .enumerate()
        .map(|(idx, bench)| {
            let is_selected = benchmarks.selected == Some(idx);
            let is_running = benchmarks.running.as_ref().map(|r| r == &bench.bench_file).unwrap_or(false);

            let status_icon = if is_running {
                "🔄"
            } else if benchmarks.last_result.as_ref()
                .map(|r| r.name == bench.name && r.success)
                .unwrap_or(false) {
                "✅"
            } else if benchmarks.last_result.as_ref()
                .map(|r| r.name == bench.name && !r.success)
                .unwrap_or(false) {
                "❌"
            } else {
                "⏸️"
            };

            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let name_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", status_icon), style),
                Span::styled(&bench.name, name_style),
            ]))
            .style(style)
        })
        .collect();

    let title = if benchmarks.running.is_some() {
        " 📋 Benchmarks (Executando...) "
    } else {
        " 📋 Benchmarks Disponíveis "
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(list, area);
}

/// Renderiza o resultado/logs do benchmark
fn render_benchmark_result(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let benchmarks = &app.benchmarks;

    // Dividir área entre descrição/resultado e logs/resultados dinâmicos
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),   // Descrição e resultado
            Constraint::Min(5),      // Logs + Resultados dinâmicos
        ])
        .split(area);

    // Dividir a área de logs entre logs (esquerda) e resultados dinâmicos (direita)
    let logs_results_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),  // Logs de execução
            Constraint::Percentage(45),  // Resultados dinâmicos
        ])
        .split(chunks[1]);

    // Descrição e resultado
    let mut result_lines = Vec::new();

    if let Some(selected) = benchmarks.get_selected() {
        result_lines.push(Line::from(vec![
            Span::styled("📊 ", Style::default().fg(Color::Cyan)),
            Span::styled(&selected.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        result_lines.push(Line::from(""));
        result_lines.push(Line::from(vec![
            Span::styled("📝 ", Style::default().fg(Color::DarkGray)),
            Span::styled(&selected.description, Style::default().fg(Color::White)),
        ]));
        result_lines.push(Line::from(""));

        if let Some(result) = &benchmarks.last_result {
            if result.name == selected.name {
                result_lines.push(Line::from(vec![
                    Span::styled("⏱️  ", Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("Duração: {:.2}s", result.duration_secs),
                        Style::default().fg(Color::White),
                    ),
                ]));
                result_lines.push(Line::from(vec![
                    Span::styled("📅 ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("Executado em: {}", result.finished_at),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                if let Some(error) = &result.error {
                    result_lines.push(Line::from(""));
                    result_lines.push(Line::from(vec![
                        Span::styled("❌ Erro: ", Style::default().fg(Color::Red)),
                        Span::styled(error.clone(), Style::default().fg(Color::Red)),
                    ]));
                }
            }
        }

        if benchmarks.running.as_ref().map(|r| r == &selected.bench_file).unwrap_or(false) {
            result_lines.push(Line::from(""));
            result_lines.push(Line::from(vec![
                Span::styled("🔄 ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "Executando benchmark...",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    } else {
        result_lines.push(Line::from(vec![
            Span::styled(
                "Selecione um benchmark para ver detalhes",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    let result_widget = Paragraph::new(Text::from(result_lines))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 📈 Detalhes ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );
    frame.render_widget(result_widget, chunks[0]);

    // Logs da execução com wrap de mensagens longas
    let logs_area = logs_results_chunks[0];
    let visible_height = logs_area.height.saturating_sub(2) as usize;

    // Calcular larguras disponíveis para wrap
    let timestamp_width = 10; // "[HH:MM:SS] "
    let symbol_width = 4; // emoji + espaço
    let border_width = 2; // bordas do Block
    let padding = 2; // margem de segurança

    // Largura disponível para a mensagem na primeira linha
    let max_msg_width = (logs_area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(timestamp_width)
        .saturating_sub(symbol_width)
        .saturating_sub(padding);

    // Largura para linhas de continuação (indentadas)
    let indent_width = 7; // "     ↳ "
    let continuation_width = (logs_area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(indent_width)
        .saturating_sub(padding);

    // Função auxiliar para truncar string respeitando limite de largura
    fn truncate_msg(s: &str, max_width: usize) -> String {
        if s.chars().count() <= max_width {
            s.to_string()
        } else {
            s.chars().take(max_width.saturating_sub(3)).collect::<String>() + "..."
        }
    }

    let mut log_items: Vec<ListItem<'_>> = Vec::new();
    let mut line_count = 0;

    for entry in benchmarks.execution_logs.iter().skip(benchmarks.log_scroll) {
        if line_count >= visible_height {
            break;
        }

        let style = match entry.level {
            LogLevel::Info => Style::default().fg(Color::White),
            LogLevel::Success => Style::default().fg(Color::Green),
            LogLevel::Warning => Style::default().fg(Color::Yellow),
            LogLevel::Error => Style::default().fg(Color::Red),
            LogLevel::Debug => Style::default().fg(Color::DarkGray),
        };

        let msg = &entry.message;
        let msg_char_count = msg.chars().count();

        if msg_char_count <= max_msg_width {
            // Mensagem cabe em uma linha
            log_items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", entry.level.symbol()), style),
                Span::styled(msg.clone(), style),
            ])));
            line_count += 1;
        } else {
            // Mensagem precisa de wrap - primeira linha com timestamp/símbolo
            let first_chunk = truncate_msg(msg, max_msg_width);
            log_items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", entry.level.symbol()), style),
                Span::styled(first_chunk, style),
            ])));
            line_count += 1;

            // Linhas de continuação (indentadas)
            let msg_chars: Vec<char> = msg.chars().collect();
            let mut char_idx = max_msg_width.min(msg_chars.len());

            while char_idx < msg_chars.len() && line_count < visible_height {
                let remaining_chars: Vec<char> = msg_chars[char_idx..].iter().cloned().collect();
                let chunk: String = remaining_chars
                    .iter()
                    .take(continuation_width.min(remaining_chars.len()))
                    .collect();

                // Truncar se necessário
                let truncated_chunk = if chunk.chars().count() > continuation_width {
                    truncate_msg(&chunk, continuation_width)
                } else {
                    chunk
                };

                log_items.push(ListItem::new(Line::from(vec![
                    Span::styled("     ", Style::default()), // Indentação
                    Span::styled("↳ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(truncated_chunk.clone(), style),
                ])));

                char_idx += truncated_chunk.chars().count();
                line_count += 1;

                // Se truncamos, parar para evitar loop infinito
                if truncated_chunk.ends_with("...") {
                    break;
                }
            }
        }
    }

    let scroll_info = if benchmarks.execution_logs.len() > visible_height {
        format!(
            " [{}/{}]",
            benchmarks.log_scroll + 1,
            benchmarks.execution_logs.len().saturating_sub(visible_height) + 1
        )
    } else {
        String::new()
    };

    let logs = List::new(log_items)
        .block(
            Block::default()
                .title(format!(" 📋 Logs de Execução{} ", scroll_info))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
    frame.render_widget(logs, logs_area);

    // Renderizar resultados dinâmicos à direita
    render_benchmark_dynamic_results(frame, app, logs_results_chunks[1]);
}

/// Renderiza os resultados dinâmicos do benchmark
fn render_benchmark_dynamic_results(frame: &mut Frame<'_>, app: &App, area: Rect) {
    use crate::tui::app::FieldStatus;

    let benchmarks = &app.benchmarks;
    let results = &benchmarks.dynamic_results;

    let mut lines: Vec<Line<'_>> = Vec::new();

    // Header com nome do benchmark
    if !results.bench_name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" 📊 {} ", results.bench_name),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // Verificar se há campos definidos
    if results.fields.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "Aguardando resultados...",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Os resultados aparecerão aqui",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "conforme o benchmark executa.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    } else {
        // Agrupar campos por grupo
        let mut current_group: Option<String> = None;

        for field in results.sorted_fields() {
            // Verificar se mudou de grupo
            if field.group != current_group {
                if current_group.is_some() {
                    lines.push(Line::from("")); // Separador entre grupos
                }
                if let Some(ref group) = field.group {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(" ═══ {} ═══ ", group),
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    lines.push(Line::from(""));
                }
                current_group = field.group.clone();
            }

            // Status icon
            let (status_icon, status_style) = match field.status {
                FieldStatus::Pending => ("⏳", Style::default().fg(Color::DarkGray)),
                FieldStatus::Running => ("🔄", Style::default().fg(Color::Yellow)),
                FieldStatus::Success => ("✅", Style::default().fg(Color::Green)),
                FieldStatus::Failed => ("❌", Style::default().fg(Color::Red)),
                FieldStatus::Info => ("ℹ️ ", Style::default().fg(Color::Cyan)),
            };

            // Ícone personalizado ou padrão
            let icon = field.icon.as_deref().unwrap_or(status_icon);

            // Label
            let label_style = match field.status {
                FieldStatus::Pending => Style::default().fg(Color::DarkGray),
                FieldStatus::Running => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::White),
            };

            // Valor
            let value = field.value.as_deref().unwrap_or("...");
            let value_style = match field.status {
                FieldStatus::Pending => Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                FieldStatus::Running => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                FieldStatus::Success => Style::default().fg(Color::Green),
                FieldStatus::Failed => Style::default().fg(Color::Red),
                FieldStatus::Info => Style::default().fg(Color::White),
            };

            // Calcular largura máxima para valor
            let max_value_width = (area.width as usize).saturating_sub(20); // Reservar espaço para label e ícone
            let display_value = if value.chars().count() > max_value_width {
                format!("{}...", value.chars().take(max_value_width.saturating_sub(3)).collect::<String>())
            } else {
                value.to_string()
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", icon), status_style),
                Span::styled(format!("{}: ", field.label), label_style),
                Span::styled(display_value, value_style),
            ]));
        }

        // Footer com info de completude
        lines.push(Line::from(""));
        if results.is_complete {
            lines.push(Line::from(vec![
                Span::styled(
                    " ✨ Resultados completos ",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]));
        } else if benchmarks.running.is_some() {
            let completed = results.fields.iter().filter(|f| f.value.is_some()).count();
            let total = results.fields.len();
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" 🔄 {}/{} campos ", completed, total),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }

        // Timestamp de última atualização
        if let Some(ref last_update) = results.last_update {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" 🕐 Atualizado: {} ", last_update),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    let results_widget = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 📈 Resultados Dinâmicos ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );
    frame.render_widget(results_widget, area);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE CONFIGURAÇÕES
// ═══════════════════════════════════════════════════════════════════════════════

fn render_config_screen(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    // Layout principal
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),   // Tabs
            Constraint::Length(3),   // Header
            Constraint::Min(10),     // Conteúdo das configs
            Constraint::Length(2),   // Ajuda
        ])
        .split(area);

    // Tabs no topo
    render_tabs(frame, app, chunks[0]);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " ⚙️  CONFIGURAÇÕES CARREGADAS ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " │ Carregadas do arquivo .env e variáveis de ambiente",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, chunks[1]);

    // Área de conteúdo dividida em 3 colunas
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),  // Runtime
            Constraint::Percentage(34),  // LLM
            Constraint::Percentage(33),  // Agent
        ])
        .margin(1)
        .split(chunks[2]);

    // Coluna 1: Runtime Config
    render_runtime_config(frame, app, content_chunks[0]);

    // Coluna 2: LLM Config
    render_llm_config(frame, app, content_chunks[1]);

    // Coluna 3: Agent Config
    render_agent_config(frame, app, content_chunks[2]);

    // Ajuda
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Tab/1-2", Style::default().fg(Color::Yellow)),
        Span::raw(" Navegar tabs  "),
        Span::styled("Backspace", Style::default().fg(Color::Cyan)),
        Span::raw(" Voltar  "),
        Span::styled("q/Esc", Style::default().fg(Color::Red)),
        Span::raw(" Sair"),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

/// Renderiza configurações do Runtime
fn render_runtime_config(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let config = &app.loaded_config;

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" 🚀 RUNTIME ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Worker Threads: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&config.worker_threads, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Max Threads:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", config.max_threads), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Blocking:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", config.max_blocking_threads), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" WebReader:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&config.webreader, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ═══ API Keys ═══ ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" OpenAI:  ", Style::default().fg(Color::DarkGray)),
            if config.openai_key_present {
                Span::styled("✅ Configurada", Style::default().fg(Color::Green))
            } else {
                Span::styled("❌ Não encontrada", Style::default().fg(Color::Red))
            },
        ]),
        Line::from(vec![
            Span::styled(" Jina:    ", Style::default().fg(Color::DarkGray)),
            if config.jina_key_present {
                Span::styled("✅ Configurada", Style::default().fg(Color::Green))
            } else {
                Span::styled("❌ Não encontrada", Style::default().fg(Color::Red))
            },
        ]),
    ];

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 🖥️ Runtime ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

    frame.render_widget(content, area);
}

/// Renderiza configurações do LLM
fn render_llm_config(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let config = &app.loaded_config;

    // Calcular largura disponível para valores
    let border_width = 2;
    let padding = 2;
    let label_width = 14; // " Provider:    "
    let max_value_width = (area.width as usize)
        .saturating_sub(border_width)
        .saturating_sub(padding)
        .saturating_sub(label_width);

    // Função para truncar valores longos
    let truncate_value = |s: &str| -> String {
        if s.chars().count() > max_value_width {
            s.chars().take(max_value_width.saturating_sub(3)).collect::<String>() + "..."
        } else {
            s.to_string()
        }
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" 🤖 LLM ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Provider:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(truncate_value(&config.llm_provider), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Model:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(truncate_value(&config.llm_model), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled(" Temperature: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.2}", config.temperature), Style::default().fg(Color::White)),
        ]),
    ];

    // API Base URL se presente
    if let Some(ref url) = config.api_base_url {
        lines.push(Line::from(vec![
            Span::styled(" API Base:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(truncate_value(url), Style::default().fg(Color::Yellow)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" ═══ Embeddings ═══ ", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Provider:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(truncate_value(&config.embedding_provider), Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Model:       ", Style::default().fg(Color::DarkGray)),
        Span::styled(truncate_value(&config.embedding_model), Style::default().fg(Color::Cyan)),
    ]));

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 🧠 LLM ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );

    frame.render_widget(content, area);
}

/// Renderiza configurações do Agent
fn render_agent_config(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let config = &app.loaded_config;

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" 🕵️ AGENT ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Min Steps:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", config.min_steps_before_answer), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Direct Answer: ", Style::default().fg(Color::DarkGray)),
            if config.allow_direct_answer {
                Span::styled("✅ Sim", Style::default().fg(Color::Green))
            } else {
                Span::styled("❌ Não", Style::default().fg(Color::Red))
            },
        ]),
        Line::from(vec![
            Span::styled(" Token Budget:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", config.default_token_budget), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ═══ Limites ═══ ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" URLs/Step:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", config.max_urls_per_step), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Queries/Step:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", config.max_queries_per_step), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Max Failures:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", config.max_consecutive_failures), Style::default().fg(Color::Yellow)),
        ]),
    ];

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" 🤖 Agent ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );

    frame.render_widget(content, area);
}
