use crate::kernel::{Kernel, Syscall, SyscallResult};
use std::collections::HashMap;

pub struct Shell {
    pub history: Vec<String>,
    pub current_input: String,
    pub output_lines: Vec<String>,
    pub scroll_offset: usize,
    pub cwd: String,
    pub vars: HashMap<String, String>,
}

impl Shell {
    pub fn new() -> Self {
        let mut shell = Shell {
            history: Vec::new(),
            current_input: String::new(),
            output_lines: Vec::new(),
            scroll_offset: 0,
            cwd: "/home".to_string(),
            vars: HashMap::new(),
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

    /// 展開字串中的 $VAR 變數
    fn expand_vars(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '$' {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if name.is_empty() {
                    result.push('$');
                } else {
                    result.push_str(self.vars.get(&name).map(|s| s.as_str()).unwrap_or(""));
                }
            } else {
                result.push(ch);
            }
        }
        result
    }

    /// 執行一段腳本文字（支援變數、if/else/end、for/end、#註解）
    pub fn run_script(&mut self, script: &str, kernel: &mut Kernel) -> Option<String> {
        let lines: Vec<&str> = script.lines().collect();
        let mut ip = 0; // instruction pointer
        let mut editor_file: Option<String> = None;

        while ip < lines.len() {
            let raw = lines[ip].trim();
            ip += 1;

            // 跳過空行與註解
            if raw.is_empty() || raw.starts_with('#') {
                continue;
            }

            let line = self.expand_vars(raw);
            let line = line.trim();

            // 變數賦值：VAR=value
            if let Some(eq_pos) = line.find('=') {
                let key = &line[..eq_pos];
                if key.chars().all(|c| c.is_alphanumeric() || c == '_') && !key.is_empty() {
                    let value = line[eq_pos + 1..].to_string();
                    self.vars.insert(key.to_string(), value);
                    continue;
                }
            }

            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            match parts[0] {
                // if <condition> ... else ... end
                "if" => {
                    let cond = parts.get(1).copied().unwrap_or("").trim();
                    let cond_result = self.eval_condition(cond, kernel);

                    // 收集 if-body 與 else-body
                    let mut if_body: Vec<String> = Vec::new();
                    let mut else_body: Vec<String> = Vec::new();
                    let mut in_else = false;
                    let mut depth = 1;

                    while ip < lines.len() {
                        let inner = lines[ip].trim();
                        ip += 1;
                        let inner_exp = self.expand_vars(inner);
                        let inner_trimmed = inner_exp.trim().to_string();
                        let first_word = inner_trimmed.split_whitespace().next().unwrap_or("");
                        match first_word {
                            "if" | "for" => {
                                depth += 1;
                                if in_else { else_body.push(inner.to_string()); }
                                else { if_body.push(inner.to_string()); }
                            }
                            "else" if depth == 1 => { in_else = true; }
                            "end" => {
                                depth -= 1;
                                if depth == 0 { break; }
                                if in_else { else_body.push(inner.to_string()); }
                                else { if_body.push(inner.to_string()); }
                            }
                            _ => {
                                if in_else { else_body.push(inner.to_string()); }
                                else { if_body.push(inner.to_string()); }
                            }
                        }
                    }

                    let body = if cond_result { if_body } else { else_body };
                    let block = body.join("\n");
                    if let Some(f) = self.run_script(&block, kernel) {
                        editor_file = Some(f);
                    }
                }

                // for VAR in val1 val2 val3 ... end
                "for" => {
                    let rest = parts.get(1).copied().unwrap_or("");
                    let for_parts: Vec<&str> = rest.splitn(3, ' ').collect();
                    let var_name = for_parts.first().copied().unwrap_or("").to_string();
                    let values: Vec<String> = if for_parts.len() >= 3 && for_parts[1] == "in" {
                        for_parts[2].split_whitespace().map(|s| s.to_string()).collect()
                    } else {
                        Vec::new()
                    };

                    // 收集 loop body
                    let mut loop_body: Vec<String> = Vec::new();
                    let mut depth = 1;
                    while ip < lines.len() {
                        let inner = lines[ip].trim();
                        ip += 1;
                        let inner_exp = self.expand_vars(inner);
                        let first_word = inner_exp.trim().split_whitespace().next().unwrap_or("").to_string();
                        match first_word.as_str() {
                            "if" | "for" => { depth += 1; loop_body.push(inner.to_string()); }
                            "end" => {
                                depth -= 1;
                                if depth == 0 { break; }
                                loop_body.push(inner.to_string());
                            }
                            _ => { loop_body.push(inner.to_string()); }
                        }
                    }

                    let block = loop_body.join("\n");
                    for val in values {
                        self.vars.insert(var_name.clone(), val);
                        if let Some(f) = self.run_script(&block, kernel) {
                            editor_file = Some(f);
                        }
                    }
                }

                "else" | "end" => {
                    // 脫離上下文時忽略
                }

                _ => {
                    if let Some(f) = self.execute_command(line, kernel) {
                        editor_file = Some(f);
                    }
                }
            }
        }
        editor_file
    }

    /// 評估條件表達式，回傳 bool
    /// 支援：`<cmd>` 的輸出非空視為 true、`VAR == value`、`VAR != value`
    fn eval_condition(&mut self, cond: &str, kernel: &mut Kernel) -> bool {
        let cond = cond.trim();

        // VAR == value
        if let Some(pos) = cond.find("==") {
            let lhs = cond[..pos].trim().to_string();
            let rhs = cond[pos + 2..].trim().to_string();
            let lhs_val = self.expand_vars(&lhs);
            let rhs_val = self.expand_vars(&rhs);
            return lhs_val.trim() == rhs_val.trim();
        }

        // VAR != value
        if let Some(pos) = cond.find("!=") {
            let lhs = cond[..pos].trim().to_string();
            let rhs = cond[pos + 2..].trim().to_string();
            let lhs_val = self.expand_vars(&lhs);
            let rhs_val = self.expand_vars(&rhs);
            return lhs_val.trim() != rhs_val.trim();
        }

        // exists <path>：檔案/目錄是否存在
        if let Some(path_part) = cond.strip_prefix("exists ") {
            let path = self.expand_vars(path_part.trim());
            let path = self.resolve_path(path.trim());
            return matches!(
                kernel.dispatch(Syscall::Open { path }),
                SyscallResult::Success
            );
        }

        // 空字串視為 false，非空視為 true（變數展開後）
        let expanded = self.expand_vars(cond);
        !expanded.trim().is_empty() && expanded.trim() != "0" && expanded.trim() != "false"
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
                self.push_output("  echo <text>       - Echo text (supports $VAR)");
                self.push_output("  pwd               - Print working dir");
                self.push_output("  cd <path>         - Change directory");
                self.push_output("  tree              - Show filesystem tree");
                self.push_output("  edit <file>       - Open text editor");
                self.push_output("  run <file>        - Execute a shell script");
                self.push_output("  set [VAR=value]   - Set/list variables");
                self.push_output("  unset <VAR>       - Remove variable");
                self.push_output("Script syntax:");
                self.push_output("  VAR=value         - Assign variable");
                self.push_output("  $VAR              - Expand variable");
                self.push_output("  # comment         - Comment line");
                self.push_output("  if COND/else/end  - Conditional");
                self.push_output("  for V in a b c/end- Loop");
                self.push_output("  if exists <path>  - File exists check");
                self.push_output("  if A == B         - String equality");
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
                let expanded = self.expand_vars(args);
                self.push_output(&expanded);
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
            // 執行腳本檔案
            "run" => {
                if args.is_empty() {
                    self.push_output("Usage: run <script>");
                } else {
                    let path = self.resolve_path(args);
                    match kernel.dispatch(Syscall::Read { path: path.clone() }) {
                        SyscallResult::Str(script) => {
                            self.push_output(&format!("--- Running script: {} ---", path));
                            let script_owned = script.clone();
                            let result = self.run_script(&script_owned, kernel);
                            self.push_output(&format!("--- Script done: {} ---", path));
                            return result;
                        }
                        SyscallResult::Err(e) => self.push_output(&format!("run: {}", e)),
                        _ => {}
                    }
                }
            }
            // 設定/列出變數
            "set" => {
                if args.is_empty() {
                    if self.vars.is_empty() {
                        self.push_output("(no variables set)");
                    } else {
                        let pairs: Vec<String> = self.vars.iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect();
                        for p in pairs {
                            self.push_output(&p);
                        }
                    }
                } else if let Some(eq) = args.find('=') {
                    let key = args[..eq].trim().to_string();
                    let val = args[eq + 1..].trim().to_string();
                    self.vars.insert(key, val);
                } else {
                    self.push_output("Usage: set VAR=value  or  set (list all)");
                }
            }
            // 刪除變數
            "unset" => {
                if args.is_empty() {
                    self.push_output("Usage: unset <VAR>");
                } else {
                    self.vars.remove(args.trim());
                    self.push_output(&format!("unset {}", args.trim()));
                }
            }
            _ => {
                // 嘗試直接執行（VAR=value 賦值語法）
                if let Some(eq_pos) = cmd.find('=') {
                    let key = cmd[..eq_pos].trim();
                    if key.chars().all(|c| c.is_alphanumeric() || c == '_') && !key.is_empty() {
                        let val = cmd[eq_pos + 1..].trim().to_string();
                        let val = self.expand_vars(&val);
                        self.vars.insert(key.to_string(), val.clone());
                        self.push_output(&format!("{}={}", key, val));
                        return None;
                    }
                }
                self.push_output(&format!("Unknown command: {}. Type 'help' for help.", command));
            }
        }
        None
    }
}
