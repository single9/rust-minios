use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
};
use crate::kernel::Kernel;
use crate::kernel::process::ProcessState;

pub fn render_process_view(f: &mut Frame, area: Rect, kernel: &Kernel) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(8),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(area);

    // ── Process table ──
    let processes = kernel.processes.list();
    let rows: Vec<Row> = processes.iter().map(|p| {
        let state_color = match p.state {
            ProcessState::Running => Color::Green,
            ProcessState::Ready => Color::Yellow,
            ProcessState::Blocked => Color::Red,
            ProcessState::Terminated => Color::DarkGray,
            ProcessState::New => Color::Cyan,
        };
        let mem_kb: u32 = p.memory_pages.len() as u32 * 4;
        Row::new(vec![
            Cell::from(p.pid.to_string()),
            Cell::from(p.name.clone()),
            Cell::from(p.state.to_string()).style(Style::default().fg(state_color)),
            Cell::from(p.priority.to_string()),
            Cell::from(p.cpu_time.to_string()),
            Cell::from(format!("{} KB", mem_kb)),
        ])
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(5),
        Constraint::Length(15),
        Constraint::Length(12),
        Constraint::Length(5),
        Constraint::Length(10),
        Constraint::Length(10),
    ])
    .header(Row::new(vec!["PID", "NAME", "STATE", "PRI", "CPU TIME", "MEMORY"])
        .style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)))
    .block(Block::bordered().title(" Process Table "));

    f.render_widget(table, chunks[0]);

    // ── Visual Scheduler ──
    let sched_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Left: Running process + quantum gauge
    render_running_box(f, sched_chunks[0], kernel);

    // Right: Ready queue boxes
    render_ready_queue(f, sched_chunks[1], kernel);

    // ── Gantt chart (state history) ──
    render_gantt_chart(f, chunks[2], kernel);

    // ── Stats ──
    let tick_widget = Paragraph::new(format!(
        "  System tick: {} | Processes: {} | Blocked: {}",
        kernel.scheduler.tick,
        kernel.processes.list().len(),
        kernel.scheduler.blocked.len(),
    ))
    .block(Block::default().borders(Borders::ALL).title(" Stats "));
    f.render_widget(tick_widget, chunks[3]);
}

fn render_running_box(f: &mut Frame, area: Rect, kernel: &Kernel) {
    let current_pid = kernel.scheduler.current;
    let (name, pid_str, quantum, time_quantum) = match current_pid.and_then(|pid| kernel.processes.get(pid)) {
        Some(p) => (p.name.clone(), p.pid.to_string(), kernel.scheduler.current_quantum, kernel.scheduler.time_quantum),
        None => ("idle".to_string(), "-".to_string(), 0, 1),
    };

    let ratio = quantum as f64 / time_quantum as f64;
    let gauge_color = if ratio > 0.7 { Color::Red }
                      else if ratio > 0.4 { Color::Yellow }
                      else { Color::Green };

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .margin(1)
        .split(area);

    let title = format!(" Running: {} (PID={}) ", name, pid_str);
    let gauge = Gauge::default()
        .block(Block::bordered().title(title))
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(ratio)
        .label(format!("quantum: {}/{}", quantum, time_quantum));
    f.render_widget(gauge, inner[0]);
}

fn render_ready_queue(f: &mut Frame, area: Rect, kernel: &Kernel) {
    let mut lines: Vec<Line> = Vec::new();

    let queue: Vec<(u32, String)> = kernel.scheduler.ready_queue.iter()
        .filter_map(|&pid| {
            kernel.processes.get(pid).map(|p| (pid, p.name.clone()))
        })
        .collect();

    if queue.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (empty)  ",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // Render as visual boxes side by side (up to 4 per row)
        for chunk in queue.chunks(3) {
            let mut spans: Vec<Span> = Vec::new();
            for (i, (pid, name)) in chunk.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(
                    format!(" {}:{} ", name, pid),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
                spans.push(Span::raw(" "));
            }
            lines.push(Line::from(spans));
        }
    }

    let widget = Paragraph::new(lines)
        .block(Block::bordered().title(" Ready Queue "));
    f.render_widget(widget, area);
}

fn render_gantt_chart(f: &mut Frame, area: Rect, kernel: &Kernel) {
    let processes = kernel.processes.list();
    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(Span::styled(
        "State History (last 40 ticks):",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));

    for p in &processes {
        if p.pid == 0 { continue; } // skip kernel
        let mut spans: Vec<Span> = vec![
            Span::styled(
                format!(" {:<8} ", p.name),
                Style::default().fg(Color::Cyan),
            ),
        ];

        for state in &p.state_history {
            let (color, ch) = match state {
                ProcessState::Running => (Color::Green, '█'),
                ProcessState::Ready => (Color::Yellow, '▓'),
                ProcessState::Blocked => (Color::Red, '▒'),
                ProcessState::Terminated => (Color::DarkGray, '░'),
                ProcessState::New => (Color::Cyan, '·'),
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }

        // Pad with '·' to fill 40 chars
        while spans.len() < 1 + 40 {
            spans.push(Span::styled("·", Style::default().fg(Color::DarkGray)));
        }

        lines.push(Line::from(spans));
    }

    if processes.len() <= 1 {
        lines.push(Line::from(Span::styled(
            "  (no user processes)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Legend
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" █", Style::default().fg(Color::Green)),
        Span::raw("Running  "),
        Span::styled("▓", Style::default().fg(Color::Yellow)),
        Span::raw("Ready  "),
        Span::styled("▒", Style::default().fg(Color::Red)),
        Span::raw("Blocked  "),
        Span::styled("░", Style::default().fg(Color::DarkGray)),
        Span::raw("Terminated  "),
        Span::styled("·", Style::default().fg(Color::Cyan)),
        Span::raw("New"),
    ]));

    let widget = Paragraph::new(lines)
        .block(Block::bordered().title(" Gantt "));
    f.render_widget(widget, area);
}
