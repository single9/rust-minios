use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};
use crate::kernel::fs::FileSystem;

pub struct FsViewState {
    pub selected: usize,
    pub scroll: usize,
}

impl FsViewState {
    pub fn new() -> Self {
        FsViewState { selected: 0, scroll: 0 }
    }
}

pub fn render_fs_view(f: &mut Frame, area: Rect, fs: &FileSystem, state: &mut FsViewState) {
    let tree = fs.get_tree();
    let lines: Vec<&str> = tree.lines().collect();
    let total = lines.len();

    // Clamp scroll
    if state.scroll > total.saturating_sub(1) {
        state.scroll = total.saturating_sub(1);
    }

    let visible_height = area.height.saturating_sub(2) as usize;
    let display_lines: Vec<String> = lines
        .iter()
        .skip(state.scroll)
        .take(visible_height)
        .enumerate()
        .map(|(i, &line)| {
            if i + state.scroll == state.selected {
                format!("> {}", line)
            } else {
                format!("  {}", line)
            }
        })
        .collect();

    let text = display_lines.join("\n");
    let widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" File System Browser "));
    f.render_widget(widget, area);

    let _ = state; // used mutably above
}
