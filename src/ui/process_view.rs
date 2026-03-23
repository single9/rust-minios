use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use crate::kernel::Kernel;
use crate::kernel::process::ProcessState;

pub fn render_process_view(f: &mut Frame, area: Rect, kernel: &Kernel) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(5), Constraint::Length(3)])
        .split(area);

    // Process table
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

    // Ready queue visualization
    let ready_queue: Vec<String> = kernel.scheduler.ready_queue.iter()
        .map(|&pid| {
            let name = kernel.processes.get(pid)
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            format!("[{}:{}]", pid, name)
        })
        .collect();

    let current_str = kernel.scheduler.current.map(|pid| {
        let name = kernel.processes.get(pid)
            .map(|p| p.name.as_str())
            .unwrap_or("?");
        format!("* {}:{}", pid, name)
    }).unwrap_or_else(|| "idle".to_string());

    let queue_text = if ready_queue.is_empty() {
        "(empty)".to_string()
    } else {
        ready_queue.join(" -> ")
    };

    let queue_lines = vec![
        Line::from(vec![
            Span::styled("Running: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(current_str, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Queue:   ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(queue_text),
        ]),
        Line::from(vec![
            Span::styled("Quantum: ", Style::default().fg(Color::White)),
            Span::raw(format!("{}/{}", kernel.scheduler.current_quantum, kernel.scheduler.time_quantum)),
        ]),
    ];

    let queue_widget = Paragraph::new(queue_lines)
        .block(Block::default().borders(Borders::ALL).title(" Scheduler (Round-Robin) "));
    f.render_widget(queue_widget, chunks[1]);

    // Tick counter
    let tick_widget = Paragraph::new(format!(
        "  System tick: {} | Processes: {} | Blocked: {}",
        kernel.scheduler.tick,
        kernel.processes.list().len(),
        kernel.scheduler.blocked.len(),
    ))
    .block(Block::default().borders(Borders::ALL).title(" Stats "));
    f.render_widget(tick_widget, chunks[2]);
}
