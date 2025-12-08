//! Renderização da interface TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, AppScreen, LogLevel};

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
    let action_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Step: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.current_step),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Ação: ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {}", truncate(&app.current_action, 25)),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let action = Paragraph::new(action_lines).block(
        Block::default()
            .title(" 🎯 Ação Atual ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(action, chunks[1]);
}

/// Renderiza o conteúdo principal (logs + stats + personas)
fn render_main_content(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Divide em logs (55%), stats (22%), personas (23%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(22),
            Constraint::Percentage(23),
        ])
        .split(area);

    render_logs(frame, app, chunks[0]);
    render_stats(frame, app, chunks[1]);
    render_personas(frame, app, chunks[2]);
}

/// Renderiza a área de logs
fn render_logs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    // Largura disponível para a mensagem (descontando bordas, timestamp e símbolo)
    let max_msg_width = (area.width as usize).saturating_sub(18);

    let items: Vec<ListItem<'_>> = app
        .logs
        .iter()
        .skip(app.log_scroll)
        .take(visible_height)
        .map(|entry| {
            let style = match entry.level {
                LogLevel::Info => Style::default().fg(Color::White),
                LogLevel::Success => Style::default().fg(Color::Green),
                LogLevel::Warning => Style::default().fg(Color::Yellow),
                LogLevel::Error => Style::default().fg(Color::Red),
                LogLevel::Debug => Style::default().fg(Color::DarkGray),
            };

            // Truncar mensagem para caber na largura
            let truncated_msg = truncate(&entry.message, max_msg_width);

            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", entry.level.symbol()), style),
                Span::styled(truncated_msg, style),
            ]);

            ListItem::new(content)
        })
        .collect();

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
            Constraint::Length(3),   // Header com UUID
            Constraint::Min(6),      // Resposta
            Constraint::Length(6),   // Referências
            Constraint::Length(5),   // Personas
            Constraint::Length(5),   // Stats finais
            Constraint::Length(2),   // Ajuda
        ])
        .margin(1)
        .split(frame.area());

    // Header com UUID
    let session_id_short = &app.session_id[..8];
    let header = Paragraph::new(Line::from(vec![
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
        Span::raw("  │  "),
        Span::styled("💾 JSON salvo", Style::default().fg(Color::DarkGray)),
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

    // Referências
    let refs_items: Vec<ListItem<'_>> = app
        .references
        .iter()
        .take(4)
        .enumerate()
        .map(|(i, r)| {
            let truncated = truncate(r, (chunks[2].width as usize).saturating_sub(8));
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {}. ", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(truncated, Style::default().fg(Color::Blue)),
            ]))
        })
        .collect();

    let refs = List::new(refs_items).block(
        Block::default()
            .title(format!(" 📚 Referências ({}) ", app.references.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(refs, chunks[2]);

    // Personas utilizadas
    let persona_items: Vec<Span<'_>> = app
        .personas
        .iter()
        .map(|(name, stats)| {
            Span::styled(
                format!(" {} (🔍{} 📖{} 🎫{}) ", name, stats.searches, stats.reads, stats.tokens),
                Style::default().fg(Color::Cyan),
            )
        })
        .collect();

    let personas_line = if persona_items.is_empty() {
        Line::from(vec![Span::styled(" Nenhuma persona utilizada", Style::default().fg(Color::DarkGray))])
    } else {
        Line::from(persona_items)
    };

    let personas = Paragraph::new(Text::from(vec![Line::from(""), personas_line]))
        .block(
            Block::default()
                .title(format!(" 👥 Personas ({}) ", app.personas.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(personas, chunks[3]);

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
