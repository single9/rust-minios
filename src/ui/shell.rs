use crate::kernel::{Kernel, Syscall, SyscallResult};

pub struct Shell {
    pub history: Vec<String>,
    pub current_input: String,
    pub output_lines: Vec<String>,
    pub scroll_offset: usize,
    pub cwd: String,
}

impl Shell {
    pub fn new() -> Self {
        let mut shell = Shell {
            history: Vec::new(),
            current_input: String::new(),
            output_lines: Vec::new(),
            scroll_offset: 0,
            cwd: "/home".to_string(),
        };
        shell.push_output("rust-minios shell v0.1.0");
        shell.push_output("Type 'help' for available commands.");
        shell.push_output("");
        shell
    }

    fn push_output(&mut self, line: &str) {
        self.output_lines.push(line.to_string());
        if self.output_lines.len() > 200 {
            self.output_lines.remove(0);
        }
        // Auto-scroll to bottom
        self.scroll_offset = self.output_lines.len().saturating_sub(1);
    }

    pub fn handle_input(&mut self, ch: char) {
        self.current_input.push(ch);
    }

    pub fn handle_backspace(&mut self) {
        self.current_input.pop();
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_offset + 1 < self.output_lines.len() {
            self.scroll_offset += 1;
        }
    }

    pub fn handle_enter(&mut self, kernel: &mut Kernel) -> Option<String> {
        let input = self.current_input.trim().to_string();
        self.current_input.clear();

        if input.is_empty() {
            self.push_output(&format!("{}$ ", self.cwd));
            return None;
        }

        self.history.push(input.clone());
        self.push_output(&format!("{}$ {}", self.cwd, input));

        let result = self.execute_command(&input, kernel);
        self.push_output("");
        result
    }

    fn resolve_path(&self, path: &str) -> String {
        if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.cwd.trim_end_matches('/'), path)
        }
    }

    pub fn execute_command(&mut self, cmd: &str, kernel: &mut Kernel) -> Option<String> {
        let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
        let command = parts[0];
        let args = parts.get(1).copied().unwrap_or("");

        match command {
            "help" => {
                self.push_output("Available commands:");
                self.push_output("  help              - Show this help");
                self.push_output("  ls [path]         - List directory");
                self.push_output("  cat <file>        - Show file contents");
                self.push_output("  mkdir <dir>       - Create directory");
                self.push_output("  touch <file>      - Create empty file");
                self.push_output("  rm <file>         - Delete file/dir");
                self.push_output("  ps                - List processes");
                self.push_output("  kill <pid>        - Kill process");
                self.push_output("  exec <name>       - Run new process");
                self.push_output("  free              - Show memory stats");
                self.push_output("  malloc <n>        - Allocate n bytes");
                self.push_output("  echo <text>       - Echo text");
                self.push_output("  pwd               - Print working dir");
                self.push_output("  cd <path>         - Change directory");
                self.push_output("  tree              - Show filesystem tree");
                self.push_output("  edit <file>       - Open text editor");
            }
            "ls" => {
                let path = if args.is_empty() {
                    self.cwd.clone()
                } else {
                    self.resolve_path(args)
                };
                match kernel.dispatch(Syscall::ListDir { path }) {
                    SyscallResult::Str(s) => {
                        if s.is_empty() {
                            self.push_output("(empty)");
                        } else {
                            for line in s.lines() {
                                self.push_output(line);
                            }
                        }
                    }
                    SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                    _ => {}
                }
            }
            "cat" => {
                if args.is_empty() {
                    self.push_output("Usage: cat <file>");
                } else {
                    let path = self.resolve_path(args);
                    match kernel.dispatch(Syscall::Read { path }) {
                        SyscallResult::Str(s) => {
                            for line in s.lines() {
                                self.push_output(line);
                            }
                        }
                        SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                        _ => {}
                    }
                }
            }
            "mkdir" => {
                if args.is_empty() {
                    self.push_output("Usage: mkdir <dir>");
                } else {
                    let path = self.resolve_path(args);
                    match kernel.dispatch(Syscall::CreateDir { path: path.clone() }) {
                        SyscallResult::Success => self.push_output(&format!("Created directory: {}", path)),
                        SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                        _ => {}
                    }
                }
            }
            "touch" => {
                if args.is_empty() {
                    self.push_output("Usage: touch <file>");
                } else {
                    let path = self.resolve_path(args);
                    match kernel.dispatch(Syscall::Create { path: path.clone() }) {
                        SyscallResult::Success => self.push_output(&format!("Created file: {}", path)),
                        SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                        _ => {}
                    }
                }
            }
            "rm" => {
                if args.is_empty() {
                    self.push_output("Usage: rm <file>");
                } else {
                    let path = self.resolve_path(args);
                    match kernel.dispatch(Syscall::Delete { path: path.clone() }) {
                        SyscallResult::Success => self.push_output(&format!("Deleted: {}", path)),
                        SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                        _ => {}
                    }
                }
            }
            "ps" => {
                match kernel.dispatch(Syscall::ListProcesses) {
                    SyscallResult::Str(s) => {
                        self.push_output("PID  NAME             STATE       PRI  CPU");
                        self.push_output("───  ───────────────  ──────────  ───  ───");
                        for line in s.lines() {
                            self.push_output(line);
                        }
                    }
                    _ => {}
                }
            }
            "kill" => {
                if args.is_empty() {
                    self.push_output("Usage: kill <pid>");
                } else if let Ok(pid) = args.trim().parse::<u32>() {
                    match kernel.dispatch(Syscall::Kill { pid }) {
                        SyscallResult::Success => self.push_output(&format!("Killed process {}", pid)),
                        SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                        _ => {}
                    }
                } else {
                    self.push_output("Invalid PID");
                }
            }
            "exec" => {
                if args.is_empty() {
                    self.push_output("Usage: exec <name>");
                } else {
                    match kernel.dispatch(Syscall::Fork { name: args.to_string(), priority: 5 }) {
                        SyscallResult::Value(pid) => self.push_output(&format!("Started process '{}' with PID={}", args, pid)),
                        SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                        _ => {}
                    }
                }
            }
            "free" => {
                match kernel.dispatch(Syscall::MemStats) {
                    SyscallResult::Str(s) => self.push_output(&s),
                    _ => {}
                }
            }
            "malloc" => {
                if args.is_empty() {
                    self.push_output("Usage: malloc <bytes>");
                } else if let Ok(size) = args.trim().parse::<usize>() {
                    match kernel.dispatch(Syscall::Malloc { size }) {
                        SyscallResult::Value(ptr) => self.push_output(&format!("Allocated {} bytes at page {}", size, ptr)),
                        SyscallResult::Err(e) => self.push_output(&format!("Error: {}", e)),
                        _ => {}
                    }
                } else {
                    self.push_output("Invalid size");
                }
            }
            "echo" => {
                self.push_output(args);
            }
            "pwd" => {
                let cwd = self.cwd.clone();
                self.push_output(&cwd);
            }
            "cd" => {
                if args.is_empty() {
                    self.cwd = "/home".to_string();
                } else {
                    let new_path = self.resolve_path(args);
                    // Verify directory exists
                    match kernel.dispatch(Syscall::ListDir { path: new_path.clone() }) {
                        SyscallResult::Str(_) => {
                            self.cwd = new_path;
                        }
                        _ => {
                            let msg = format!("cd: no such directory: {}", args);
                            self.push_output(&msg);
                        }
                    }
                }
            }
            "tree" => {
                match kernel.dispatch(Syscall::GetTree) {
                    SyscallResult::Str(s) => {
                        for line in s.lines() {
                            self.push_output(line);
                        }
                    }
                    _ => {}
                }
            }
            "edit" => {
                if args.is_empty() {
                    self.push_output("Usage: edit <file>");
                } else {
                    let path = self.resolve_path(args);
                    self.push_output(&format!("Opening editor: {}", path));
                    return Some(path);
                }
            }
            _ => {
                self.push_output(&format!("Unknown command: {}. Type 'help' for help.", command));
            }
        }
        None
    }
}
