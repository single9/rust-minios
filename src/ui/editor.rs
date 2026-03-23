use crate::kernel::{Kernel, Syscall};

pub struct Editor {
    pub filename: Option<String>,
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub scroll_col: usize,
    pub modified: bool,
    pub status_msg: String,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            filename: None,
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            scroll_col: 0,
            modified: false,
            status_msg: "Ctrl+S: Save | Ctrl+Q: Quit".to_string(),
        }
    }

    pub fn open(&mut self, filename: &str, kernel: &Kernel) {
        self.filename = Some(filename.to_string());
        // Try to read from filesystem
        if let Some(content) = kernel.fs.read_file(filename) {
            self.lines = content.lines().map(|s| s.to_string()).collect();
            if self.lines.is_empty() {
                self.lines.push(String::new());
            }
        } else {
            self.lines = vec![String::new()];
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.scroll_col = 0;
        self.modified = false;
        self.status_msg = format!("Opened: {}", filename);
    }

    pub fn save(&mut self, kernel: &mut Kernel) {
        if let Some(ref filename) = self.filename.clone() {
            let content = self.lines.join("\n") + "\n";
            // Ensure file exists
            if kernel.fs.resolve_path(filename).is_none() {
                kernel.dispatch(Syscall::Create { path: filename.clone() });
            }
            kernel.dispatch(Syscall::Write { path: filename.clone(), content });
            self.modified = false;
            self.status_msg = format!("Saved: {}", filename);
        } else {
            self.status_msg = "No filename to save to".to_string();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        if self.cursor_row < self.lines.len() {
            let col = self.cursor_col.min(self.lines[self.cursor_row].len());
            self.lines[self.cursor_row].insert(col, ch);
            self.cursor_col += 1;
            self.modified = true;
        }
    }

    pub fn handle_enter(&mut self) {
        if self.cursor_row < self.lines.len() {
            let col = self.cursor_col.min(self.lines[self.cursor_row].len());
            let rest = self.lines[self.cursor_row][col..].to_string();
            self.lines[self.cursor_row].truncate(col);
            self.cursor_row += 1;
            self.lines.insert(self.cursor_row, rest);
            self.cursor_col = 0;
            self.modified = true;
        }
    }

    pub fn handle_backspace(&mut self) {
        if self.cursor_col > 0 {
            let col = self.cursor_col.min(self.lines[self.cursor_row].len());
            if col > 0 {
                self.lines[self.cursor_row].remove(col - 1);
                self.cursor_col -= 1;
                self.modified = true;
            }
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.modified = true;
        }
    }

    pub fn handle_delete(&mut self) {
        if self.cursor_row < self.lines.len() {
            let col = self.cursor_col.min(self.lines[self.cursor_row].len());
            if col < self.lines[self.cursor_row].len() {
                self.lines[self.cursor_row].remove(col);
                self.modified = true;
            } else if self.cursor_row + 1 < self.lines.len() {
                let next_line = self.lines.remove(self.cursor_row + 1);
                self.lines[self.cursor_row].push_str(&next_line);
                self.modified = true;
            }
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_row < self.lines.len() {
            if self.cursor_col < self.lines[self.cursor_row].len() {
                self.cursor_col += 1;
            } else if self.cursor_row + 1 < self.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
        }
    }

    pub fn adjust_scroll(&mut self, visible_rows: usize, visible_cols: usize) {
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.cursor_row - visible_rows + 1;
        }
        if self.cursor_col < self.scroll_col {
            self.scroll_col = self.cursor_col;
        } else if visible_cols > 0 && self.cursor_col >= self.scroll_col + visible_cols {
            self.scroll_col = self.cursor_col - visible_cols + 1;
        }
    }

    pub fn status_bar_text(&self) -> String {
        let fname = self.filename.as_deref().unwrap_or("[No Name]");
        let modified = if self.modified { " [modified]" } else { "" };
        format!("{}{} | Row:{} Col:{} | Ctrl+S save | Ctrl+Q quit | {}",
            fname, modified, self.cursor_row + 1, self.cursor_col + 1, self.status_msg)
    }
}
