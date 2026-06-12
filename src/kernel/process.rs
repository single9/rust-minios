use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    New,
    Ready,
    Running,
    Blocked,
    Terminated,
}

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessState::New => write!(f, "New"),
            ProcessState::Ready => write!(f, "Ready"),
            ProcessState::Running => write!(f, "Running"),
            ProcessState::Blocked => write!(f, "Blocked"),
            ProcessState::Terminated => write!(f, "Terminated"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    pub priority: u8,
    pub cpu_time: u64,
    pub memory_pages: Vec<u32>,
    #[expect(dead_code)]
    pub created_at: u64,
    pub state_history: VecDeque<ProcessState>,
}

impl Process {
    pub fn record_state(&mut self) {
        self.state_history.push_back(self.state.clone());
        if self.state_history.len() > 40 {
            self.state_history.pop_front();
        }
    }
}

pub struct ProcessTable {
    pub processes: HashMap<u32, Process>,
    next_pid: u32,
}

impl ProcessTable {
    pub fn new() -> Self {
        ProcessTable {
            processes: HashMap::new(),
            next_pid: 0,
        }
    }

    pub fn create(&mut self, name: &str, priority: u8) -> u32 {
        let pid = self.next_pid;
        self.next_pid += 1;
        let mut state_history = VecDeque::new();
        state_history.push_back(ProcessState::New);
        self.processes.insert(
            pid,
            Process {
                pid,
                name: name.to_string(),
                state: ProcessState::New,
                priority,
                cpu_time: 0,
                memory_pages: Vec::new(),
                created_at: 0,
                state_history,
            },
        );
        pid
    }

    pub fn get(&self, pid: u32) -> Option<&Process> {
        self.processes.get(&pid)
    }

    pub fn get_mut(&mut self, pid: u32) -> Option<&mut Process> {
        self.processes.get_mut(&pid)
    }

    pub fn set_state(&mut self, pid: u32, state: ProcessState) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.state = state;
        }
    }

    pub fn record_all_states(&mut self) {
        for p in self.processes.values_mut() {
            p.record_state();
        }
    }

    pub fn list(&self) -> Vec<Process> {
        let mut procs: Vec<Process> = self.processes.values().cloned().collect();
        procs.sort_by_key(|p| p.pid);
        procs
    }

    #[expect(dead_code)]
    pub fn remove(&mut self, pid: u32) {
        self.processes.remove(&pid);
    }
}
