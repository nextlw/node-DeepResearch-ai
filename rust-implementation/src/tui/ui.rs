//! Renderização da interface TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, AppScreen, LogLevel, ReadMethod, TaskStatus, AgentAnalyzerState};

/// Renderiza a interface completa
pub fn render(frame: &mut Frame<'_>, app: &App) {
    match app.screen {
        AppScreen::Input => render_input_screen(frame, app),
        AppScreen::Research => render_research_screen(frame, app),
        AppScreen::Result => render_result_screen(frame, app),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE INPUT
// ═══════════════════════════════════════════════════════════════════════════════

fn render_input_screen(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    // Layout vertical
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Header/Logo
            Constraint::Length(5),   // Input box
            Constraint::Min(5),      // Histórico
            Constraint::Length(3),   // Ajuda
        ])
        .margin(2)
        .split(area);

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
        .take(8)
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
        Span::styled("Esc/q", Style::default().fg(Color::Red)),
        Span::raw(" Sair"),
    ]))
    .alignment(ratatui::layout::Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELA DE PESQUISA
// ═══════════════════════════════════════════════════════════════════════════════

fn render_research_screen(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Header
            Constraint::Length(8),  // Raciocínio e Ação (em cima)
            Constraint::Min(8),     // Logs e Stats (em baixo)
            Constraint::Length(3),  // Barra de progresso
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_thinking_panel(frame, app, chunks[1]);
    render_main_content(frame, app, chunks[2]);
    render_progress(frame, app, chunks[3]);
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

    let question_display = if app.question.len() > 80 {
        format!("{}...", &app.question[..77])
    } else {
        app.question.clone()
    };

    let header_text = Text::from(vec![
        Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw(" Pergunta: "),
            Span::styled(question_display, Style::default().fg(Color::Yellow)),
        ]),
    ]);

    let header = Paragraph::new(header_text)
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

    // Mostrar steps completados (últimos 4 no máximo)
    let completed_to_show: Vec<_> = app.completed_steps.iter().rev().take(4).collect();
    for step in completed_to_show.into_iter().rev() {
        let action_short = truncate(&step.action, 18);
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
        let action_short = truncate(&app.current_action, 18);
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
    // Se AgentAnalyzer está ativo ou tem resultado, mostrar painel dedicado
    let has_analyzer = app.agent_analyzer.is_active || app.agent_analyzer.last_improvement.is_some();

    if has_analyzer {
        // Layout com AgentAnalyzer: logs (35%), analyzer (20%), tasks (15%), stats (15%), personas (15%)
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
    } else {
        // Layout padrão: logs (40%), tasks (20%), stats (20%), personas (20%)
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

    for entry in logs_to_show.into_iter().rev() {
        let style = match entry.level {
            LogLevel::Info => Style::default().fg(Color::White),
            LogLevel::Success => Style::default().fg(Color::Green),
            LogLevel::Warning => Style::default().fg(Color::Yellow),
            LogLevel::Error => Style::default().fg(Color::Red),
            LogLevel::Debug => Style::default().fg(Color::DarkGray),
        };

        // Quebrar mensagem em múltiplas linhas se necessário
        let max_width = (area.width as usize).saturating_sub(4);
        let msg = &entry.message;

        if msg.len() <= max_width {
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", entry.level.symbol()), style),
                Span::styled(msg.clone(), style),
            ]));
        } else {
            // Primeira linha com símbolo
            let first_chunk: String = msg.chars().take(max_width - 3).collect();
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", entry.level.symbol()), style),
                Span::styled(first_chunk, style),
            ]));

            // Linhas de continuação
            let mut remaining: String = msg.chars().skip(max_width - 3).collect();
            while !remaining.is_empty() {
                let chunk_size = max_width.saturating_sub(3);
                let chunk: String = remaining.chars().take(chunk_size).collect();
                lines.push(Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(chunk.clone(), style),
                ]));
                remaining = remaining.chars().skip(chunk_size).collect();
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
        let max_width = (area.width as usize).saturating_sub(4);
        let chunks: Vec<String> = improvement
            .chars()
            .collect::<Vec<_>>()
            .chunks(max_width)
            .map(|c| c.iter().collect())
            .collect();

        for chunk in chunks.iter().take(3) {
            lines.push(Line::from(vec![
                Span::styled(format!(" {}", chunk), Style::default().fg(Color::Cyan)),
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

/// Renderiza o painel de tarefas paralelas em execução
fn render_parallel_tasks(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut lines: Vec<Line<'_>> = Vec::new();

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

            // Truncar URL para caber (mais curto para dar espaço ao progresso)
            let url_display = if task.description.len() > 20 {
                format!("{}...", &task.description[..17])
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
    // Largura disponível para a mensagem (descontando bordas, timestamp e símbolo)
    let max_msg_width = (area.width as usize).saturating_sub(18);
    // Largura para linhas de continuação (sem timestamp/símbolo)
    let continuation_width = (area.width as usize).saturating_sub(6);

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

        if msg.len() <= max_msg_width {
            // Mensagem cabe em uma linha
            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", entry.level.symbol()), style),
                Span::styled(msg.clone(), style),
            ]);
            items.push(ListItem::new(content));
            line_count += 1;
        } else {
            // Mensagem precisa de wrap - primeira linha com timestamp/símbolo
            let first_chunk: String = msg.chars().take(max_msg_width).collect();
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
            let mut remaining: String = msg.chars().skip(max_msg_width).collect();
            while !remaining.is_empty() && line_count < visible_height {
                let chunk: String = remaining.chars().take(continuation_width).collect();
                let continuation_line = Line::from(vec![
                    Span::styled("     ", Style::default()), // Indentação
                    Span::styled("↳ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(chunk.clone(), style),
                ]);
                items.push(ListItem::new(continuation_line));
                remaining = remaining.chars().skip(continuation_width).collect();
                line_count += 1;
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

    let stats = Paragraph::new(stats_text).block(
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

            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", indicator), Style::default().fg(color)),
                Span::styled(
                    format!("{:<10}", name),
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

    let personas = Paragraph::new(Text::from(lines)).block(
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
    let label = if app.is_complete {
        if app.error.is_some() {
            "Erro!".to_string()
        } else {
            "Concluído!".to_string()
        }
    } else {
        format!("Step {} - {}", app.current_step, &app.current_action)
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
// TELA DE RESULTADO
// ═══════════════════════════════════════════════════════════════════════════════

fn render_result_screen(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),   // Header com UUID e JSON path
            Constraint::Min(5),      // Resposta
            Constraint::Length(5),   // Referências
            Constraint::Length(5),   // URLs visitadas
            Constraint::Length(4),   // Stats finais
            Constraint::Length(2),   // Ajuda
        ])
        .margin(1)
        .split(frame.area());

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
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
                .title(" 📊 Estatísticas ")
            .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(stats, chunks[4]);

    // Ajuda
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓/PgUp/Dn", Style::default().fg(Color::Yellow)),
        Span::raw(" Scroll  "),
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::raw(" Nova  "),
        Span::styled("q", Style::default().fg(Color::Red)),
        Span::raw(" Sair"),
    ]))
    .alignment(ratatui::layout::Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[5]);
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
