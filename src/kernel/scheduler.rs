use std::collections::VecDeque;
use super::process::{ProcessTable, ProcessState};

pub struct Scheduler {
    pub ready_queue: VecDeque<u32>,
    pub blocked: Vec<u32>,
    pub current: Option<u32>,
    pub time_quantum: u32,
    pub tick: u64,
    pub current_quantum: u32,
}

impl Scheduler {
    pub fn new(time_quantum: u32) -> Self {
        Scheduler {
            ready_queue: VecDeque::new(),
            blocked: Vec::new(),
            current: None,
            time_quantum,
            tick: 0,
            current_quantum: 0,
        }
    }

    pub fn tick(&mut self, proc_table: &mut ProcessTable) {
        self.tick += 1;

        // Increment cpu_time for running process
        if let Some(pid) = self.current {
            if let Some(p) = proc_table.get_mut(pid) {
                if p.state == ProcessState::Running {
                    p.cpu_time += 1;
                }
            }
            self.current_quantum += 1;

            // Check if quantum expired
            if self.current_quantum >= self.time_quantum {
                // Preempt current process
                if let Some(p) = proc_table.get_mut(pid) {
                    if p.state == ProcessState::Running {
                        p.state = ProcessState::Ready;
                        self.ready_queue.push_back(pid);
                    }
                }
                self.current = None;
                self.current_quantum = 0;
            }
        }

        // Pick next process if no current
        if self.current.is_none() {
            if let Some(next_pid) = self.ready_queue.pop_front() {
                self.current = Some(next_pid);
                self.current_quantum = 0;
                proc_table.set_state(next_pid, ProcessState::Running);
            }
        }
    }

    pub fn add_process(&mut self, pid: u32) {
        self.ready_queue.push_back(pid);
    }

    pub fn block_process(&mut self, pid: u32, proc_table: &mut ProcessTable) {
        if self.current == Some(pid) {
            self.current = None;
            self.current_quantum = 0;
        } else {
            self.ready_queue.retain(|&p| p != pid);
        }
        self.blocked.push(pid);
        proc_table.set_state(pid, ProcessState::Blocked);
    }

    pub fn unblock_process(&mut self, pid: u32, proc_table: &mut ProcessTable) {
        self.blocked.retain(|&p| p != pid);
        self.ready_queue.push_back(pid);
        proc_table.set_state(pid, ProcessState::Ready);
    }

    pub fn kill_process(&mut self, pid: u32, proc_table: &mut ProcessTable) {
        if self.current == Some(pid) {
            self.current = None;
            self.current_quantum = 0;
        }
        self.ready_queue.retain(|&p| p != pid);
        self.blocked.retain(|&p| p != pid);
        proc_table.set_state(pid, ProcessState::Terminated);
    }
}
