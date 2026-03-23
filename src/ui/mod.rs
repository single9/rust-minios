pub mod dashboard;
pub mod memory_view;
pub mod process_view;
pub mod fs_view;
pub mod editor;
pub mod shell;

use std::io::Stdout;
use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};

use crate::kernel::Kernel;
use self::shell::Shell;
use self::editor::Editor;
pub use self::fs_view::FsViewState;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Dashboard,
    Memory,
    Processes,
    FileSystem,
    Editor,
    Shell,
}

pub struct App {
    pub kernel: Kernel,
    pub mode: AppMode,
    pub shell: Shell,
    pub editor: Editor,
    pub fs_state: FsViewState,
    pub should_quit: bool,
}

impl App {
    pub fn new(kernel: Kernel) -> Self {
        App {
            kernel,
            mode: AppMode::Dashboard,
            shell: Shell::new(),
            editor: Editor::new(),
            fs_state: FsViewState::new(),
            should_quit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();

        loop {
        terminal.draw(|f| self.draw(f))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    pub fn draw(&mut self, f: &mut Frame) {
        dashboard::render_dashboard(f, self);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Global: Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // F-key tab switching (except in editor where Ctrl+Q exits)
        match key.code {
            KeyCode::F(1) => { self.mode = AppMode::Dashboard; return; }
            KeyCode::F(2) => { self.mode = AppMode::Memory; return; }
            KeyCode::F(3) => { self.mode = AppMode::Processes; return; }
            KeyCode::F(4) => { self.mode = AppMode::FileSystem; return; }
            KeyCode::F(5) => { self.mode = AppMode::Editor; return; }
            KeyCode::F(6) => { self.mode = AppMode::Shell; return; }
            _ => {}
        }

        match self.mode {
            AppMode::Dashboard => {
                if key.code == KeyCode::Char('q') {
                    self.should_quit = true;
                }
            }
            AppMode::Shell => {
                self.handle_shell_key(key);
            }
            AppMode::Editor => {
                self.handle_editor_key(key);
            }
            AppMode::Memory | AppMode::Processes | AppMode::FileSystem => {
                // Arrow keys for scroll in FS view
                if self.mode == AppMode::FileSystem {
                    match key.code {
                        KeyCode::Up => {
                            if self.fs_state.selected > 0 {
                                self.fs_state.selected -= 1;
                            }
                        }
                        KeyCode::Down => {
                            self.fs_state.selected += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn handle_shell_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                self.shell.handle_input(c);
            }
            KeyCode::Backspace => {
                self.shell.handle_backspace();
            }
            KeyCode::Enter => {
                let edit_file = self.shell.handle_enter(&mut self.kernel);
                if let Some(filename) = edit_file {
                    self.editor.open(&filename, &self.kernel);
                    self.mode = AppMode::Editor;
                }
            }
            KeyCode::Up => {
                self.shell.scroll_up();
            }
            KeyCode::Down => {
                self.shell.scroll_down();
            }
            _ => {}
        }
    }

    fn handle_editor_key(&mut self, key: KeyEvent) {
        // Ctrl+S: save
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => {
                    self.editor.save(&mut self.kernel);
                    return;
                }
                KeyCode::Char('q') => {
                    self.mode = AppMode::Shell;
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char(c) => self.editor.insert_char(c),
            KeyCode::Enter => self.editor.handle_enter(),
            KeyCode::Backspace => self.editor.handle_backspace(),
            KeyCode::Delete => self.editor.handle_delete(),
            KeyCode::Up => self.editor.move_cursor_up(),
            KeyCode::Down => self.editor.move_cursor_down(),
            KeyCode::Left => self.editor.move_cursor_left(),
            KeyCode::Right => self.editor.move_cursor_right(),
            _ => {}
        }
    }

    pub fn on_tick(&mut self) {
        self.kernel.tick();
    }
}
