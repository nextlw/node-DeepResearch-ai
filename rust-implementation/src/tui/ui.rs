//! Renderização da interface TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, LogLevel};

/// Renderiza a interface completa
pub fn render(frame: &mut Frame<'_>, app: &App) {
    // Layout principal
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Header
            Constraint::Min(10),    // Conteúdo principal
            Constraint::Length(3),  // Barra de progresso
            Constraint::Length(3),  // Think atual
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_main_content(frame, app, chunks[1]);
    render_progress(frame, app, chunks[2]);
    render_think(frame, app, chunks[3]);
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

/// Renderiza o conteúdo principal (logs + stats)
fn render_main_content(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Se temos resposta, mostrar ela
    if app.is_complete && app.answer.is_some() {
        render_result(frame, app, area);
        return;
    }

    // Divide em logs (70%) e stats (30%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    render_logs(frame, app, chunks[0]);
    render_stats(frame, app, chunks[1]);
}

/// Renderiza a área de logs
fn render_logs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = app.logs
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

            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", entry.level.symbol()), style),
                Span::styled(&entry.message, style),
            ]);

            ListItem::new(content)
        })
        .collect();

    let scroll_info = if app.logs.len() > visible_height {
        format!(" [{}/{}]", app.log_scroll + 1, app.logs.len().saturating_sub(visible_height) + 1)
    } else {
        String::new()
    };

    let logs = List::new(items)
        .block(
            Block::default()
                .title(format!(" 📋 Logs{} ", scroll_info))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
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
            Span::styled(format!("{}", app.current_step), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" URLs:      "),
            Span::styled(format!("{}", app.url_count), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw(" Visitadas: "),
            Span::styled(format!("{}", app.visited_count), Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Tokens:    "),
            Span::styled(format!("{}", app.tokens_used), Style::default().fg(Color::Magenta)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Tempo:     "),
            Span::styled(format!("{:.1}s", elapsed), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Ação:      "),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {}", truncate(&app.current_action, 20)),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]);

    let stats = Paragraph::new(stats_text)
        .block(
            Block::default()
                .title(" 📊 Stats ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
        );

    frame.render_widget(stats, area);
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
        if app.error.is_some() { Color::Red } else { Color::Green }
    } else {
        Color::Cyan
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(color)))
        .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
        .percent((progress * 100.0) as u16)
        .label(label);

    frame.render_widget(gauge, area);
}

/// Renderiza o think atual
fn render_think(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let think_display = if app.current_think.is_empty() {
        "Aguardando...".to_string()
    } else {
        truncate(&app.current_think, (area.width as usize).saturating_sub(10))
    };

    let think = Paragraph::new(Line::from(vec![
        Span::styled(" 💭 ", Style::default().fg(Color::Yellow)),
        Span::styled(think_display, Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
    );

    frame.render_widget(think, area);
}

/// Renderiza o resultado final
fn render_result(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let answer = app.answer.as_deref().unwrap_or("Sem resposta");

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" ✅ RESPOSTA ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    // Quebra a resposta em linhas
    let max_width = (area.width as usize).saturating_sub(4);
    for chunk in answer.chars().collect::<Vec<_>>().chunks(max_width) {
        let line_text: String = chunk.iter().collect();
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(line_text, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));

    // Referências
    if !app.references.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(" 📚 REFERÊNCIAS ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));

        for (i, ref_url) in app.references.iter().take(5).enumerate() {
            let truncated = truncate(ref_url, max_width.saturating_sub(5));
            lines.push(Line::from(vec![
                Span::styled(format!(" {}. ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(truncated, Style::default().fg(Color::Blue)),
            ]));
        }
    }

    // Stats finais
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!(" ⏱️  {:.2}s │ 🎫 {} tokens │ 🔗 {} URLs ",
                app.elapsed_secs(),
                app.tokens_used,
                app.visited_count
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    let result = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(" 🎯 Resultado ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(result, area);
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
