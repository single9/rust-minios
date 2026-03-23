use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
};
use crate::ui::{App, AppMode};

pub fn render_dashboard(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Tab bar
    render_tabs(f, chunks[0], &app.mode);

    match app.mode {
        AppMode::Dashboard => render_dashboard_content(f, chunks[1], app),
        AppMode::Memory => {
            crate::ui::memory_view::render_memory_view(f, chunks[1], &app.kernel.memory);
        }
        AppMode::Processes => {
            crate::ui::process_view::render_process_view(f, chunks[1], &app.kernel);
        }
        AppMode::FileSystem => {
            crate::ui::fs_view::render_fs_view(f, chunks[1], &app.kernel.fs, &mut app.fs_state);
        }
        AppMode::Editor => render_editor_view(f, chunks[1], app),
        AppMode::Shell => render_shell_view(f, chunks[1], app),
    }
}

fn render_tabs(f: &mut Frame, area: Rect, mode: &AppMode) {
    let titles = vec![
        "F1:Dashboard",
        "F2:Memory",
        "F3:Processes",
        "F4:FileSystem",
        "F5:Editor",
        "F6:Shell",
    ];
    let selected = match mode {
        AppMode::Dashboard => 0,
        AppMode::Memory => 1,
        AppMode::Processes => 2,
        AppMode::FileSystem => 3,
        AppMode::Editor => 4,
        AppMode::Shell => 5,
    };
    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title(" rust-minios "));
    f.render_widget(tabs, area);
}

fn render_dashboard_content(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Boot messages
    let boot_text: Vec<Line> = app.kernel.boot_messages.iter()
        .map(|s| Line::from(Span::styled(s.as_str(), Style::default().fg(Color::Green))))
        .collect();
    let boot_widget = Paragraph::new(boot_text)
        .block(Block::bordered().title(" Boot Log "));
    f.render_widget(boot_widget, chunks[0]);

    // Stats panel
    let stats = app.kernel.memory.get_stats();
    let proc_count = app.kernel.processes.list().len();
    let running = app.kernel.scheduler.current
        .and_then(|pid| app.kernel.processes.get(pid))
        .map(|p| p.name.as_str())
        .unwrap_or("idle");

    let stat_lines = vec![
        Line::from(vec![
            Span::styled("System Tick: ", Style::default().fg(Color::Cyan)),
            Span::raw(app.kernel.tick.to_string()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Memory:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(format!("  Total:   {} KB", stats.total * 4)),
        Line::from(format!("  Used:    {} KB", (stats.used_kernel + stats.used_process) * 4)),
        Line::from(format!("  Free:    {} KB", stats.free * 4)),
        Line::from(""),
        Line::from(vec![
            Span::styled("Processes:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(format!("  Total:   {}", proc_count)),
        Line::from(format!("  Running: {}", running)),
        Line::from(format!("  Blocked: {}", app.kernel.scheduler.blocked.len())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Controls:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  F1-F6: Switch views"),
        Line::from("  q: Quit (Dashboard)"),
        Line::from("  Ctrl+C: Force quit"),
    ];

    let stats_widget = Paragraph::new(stat_lines)
        .block(Block::bordered().title(" System Stats "));
    f.render_widget(stats_widget, chunks[1]);
}

fn render_shell_view(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    let total_lines = app.shell.output_lines.len();
    let max_start = total_lines.saturating_sub(visible_height);
    let start = app.shell.scroll_offset.min(max_start);
    // Normalize so ↑↓ work immediately from the current display position
    app.shell.scroll_offset = start;

    let output_text: Vec<Line> = app.shell.output_lines[start..]
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();

    let output_widget = Paragraph::new(output_text)
        .block(Block::bordered().title(" Shell (↑↓ scroll) "));
    f.render_widget(output_widget, chunks[0]);

    // Input line
    let input_line = format!("{}$ {}", app.shell.cwd, app.shell.current_input);
    let input_widget = Paragraph::new(input_line)
        .block(Block::default().borders(Borders::ALL).title(" Input "));
    f.render_widget(input_widget, chunks[1]);
}

fn render_editor_view(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let line_num_width: u16 = 5; // "NNNN " = 5 chars
    let visible_rows = chunks[0].height.saturating_sub(2) as usize;
    let visible_cols = chunks[0].width.saturating_sub(2 + line_num_width) as usize;

    app.editor.adjust_scroll(visible_rows, visible_cols);

    let editor = &app.editor;
    let start_row = editor.scroll_offset;
    let start_col = editor.scroll_col;

    let lines: Vec<Line> = editor.lines
        .iter()
        .enumerate()
        .skip(start_row)
        .take(visible_rows)
        .map(|(i, line)| {
            let line_num = format!("{:4} ", i + 1);
            let visible_line: String = line.chars().skip(start_col).collect();
            if i == editor.cursor_row {
                Line::from(vec![
                    Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                    Span::styled(visible_line, Style::default().bg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                    Span::raw(visible_line),
                ])
            }
        })
        .collect();

    let fname = editor.filename.as_deref().unwrap_or("[No Name]");
    let title = if editor.modified {
        format!(" Editor: {}* ", fname)
    } else {
        format!(" Editor: {} ", fname)
    };

    let editor_widget = Paragraph::new(lines)
        .block(Block::bordered().title(title));
    f.render_widget(editor_widget, chunks[0]);

    // Status bar
    let status = editor.status_bar_text();
    let status_widget = Paragraph::new(status)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(status_widget, chunks[1]);
}
