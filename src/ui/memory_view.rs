use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::kernel::memory::{MemoryManager, PageOwner};

pub fn render_memory_view(f: &mut Frame, area: Rect, memory: &MemoryManager) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(20), Constraint::Length(5)])
        .split(area);

    let pages = &memory.pages;
    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(vec![
        Span::styled("Memory Map (256 pages x 4KB = 1MB)", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(""));

    for row in 0..16 {
        let mut spans: Vec<Span> = Vec::new();
        // Row number
        spans.push(Span::styled(format!("{:02X} ", row), Style::default().fg(Color::DarkGray)));
        for col in 0..16 {
            let page_idx = row * 16 + col;
            let color = match pages[page_idx].owner {
                PageOwner::Free => Color::DarkGray,
                PageOwner::Kernel => Color::Blue,
                PageOwner::Process(_) => Color::Green,
                PageOwner::Reserved => Color::Red,
            };
            spans.push(Span::styled("██", Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }

    let map_widget = Paragraph::new(lines)
        .block(Block::bordered().title(" Memory Map "));
    f.render_widget(map_widget, chunks[0]);

    // Stats
    let stats = memory.get_stats();
    let (frag_ratio, max_free, _) = memory.get_fragmentation();
    let frag_pct = (frag_ratio * 100.0) as u8;
    let total_kb = stats.total * 4;
    let used_kb = (stats.used_kernel + stats.used_process) * 4;
    let frag_color = if frag_pct > 70 { Color::Red }
                     else if frag_pct > 30 { Color::Yellow }
                     else { Color::Green };

    let legend_lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ██", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(" Free ({} KB)   ", stats.free * 4)),
            Span::styled("██", Style::default().fg(Color::Blue)),
            Span::raw(format!(" Kernel ({} KB)   ", stats.used_kernel * 4)),
            Span::styled("██", Style::default().fg(Color::Green)),
            Span::raw(format!(" Process ({} KB)   ", stats.used_process * 4)),
            Span::styled("██", Style::default().fg(Color::Red)),
            Span::raw(format!(" Reserved ({} KB)", stats.reserved * 4)),
        ]),
        Line::from(format!(
            "  Total: {} KB | Used: {} KB | Free: {} KB",
            total_kb, used_kb, stats.free * 4,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Fragmentation: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}%", frag_pct), Style::default().fg(frag_color)),
            Span::raw(format!("  (largest free block: {} KB)", max_free * 4)),
        ]),
    ];

    let legend_widget = Paragraph::new(legend_lines)
        .block(Block::default().borders(Borders::ALL).title(" Legend "));
    f.render_widget(legend_widget, chunks[1]);
}
