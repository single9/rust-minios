use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
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
        Span::styled("Memory Map (256 pages × 4KB = 1MB)", Style::default().fg(Color::White)),
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
    let legend_text = format!(
        "  {} Free ({} KB)   {} Kernel ({} KB)   {} Process ({} KB)   {} Reserved ({} KB)\n  Total: {} KB | Used: {} KB | Free: {} KB",
        "██", stats.free * 4,
        "██", stats.used_kernel * 4,
        "██", stats.used_process * 4,
        "██", stats.reserved * 4,
        stats.total * 4,
        (stats.used_kernel + stats.used_process + stats.reserved) * 4,
        stats.free * 4,
    );

    let legend_lines: Vec<Line> = vec![
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
            stats.total * 4,
            (stats.used_kernel + stats.used_process + stats.reserved) * 4,
            stats.free * 4,
        )),
    ];
    let _ = legend_text; // suppress unused warning

    let legend_widget = Paragraph::new(legend_lines)
        .block(Block::default().borders(Borders::ALL).title(" Legend "));
    f.render_widget(legend_widget, chunks[1]);
}
