pub mod memory;
pub mod process;
pub mod scheduler;
pub mod fs;
pub mod io;
pub mod syscall;

use memory::{MemoryManager, PageOwner};
use process::{ProcessTable, ProcessState};
use scheduler::Scheduler;
use fs::FileSystem;
use io::IoSubsystem;
pub use syscall::{Syscall, SyscallResult};

pub struct KernelConfig {
    pub tick_rate_ms: u64,
    pub version: String,
}

pub struct Kernel {
    pub memory: MemoryManager,
    pub processes: ProcessTable,
    pub scheduler: Scheduler,
    pub fs: FileSystem,
    pub io: IoSubsystem,
    pub tick: u64,
    pub boot_messages: Vec<String>,
    pub config: KernelConfig,
}

impl Kernel {
    pub fn new() -> Self {
        let mut memory = MemoryManager::new();
        let mut processes = ProcessTable::new();
        let mut scheduler = Scheduler::new(10);
        let fs = FileSystem::new();
        let io = IoSubsystem::new();

        // Create kernel process (pid=0) as Running
        let kernel_pid = processes.create("kernel", 10);
        processes.set_state(kernel_pid, ProcessState::Running);
        scheduler.current = Some(kernel_pid);

        // Allocate memory for kernel process
        if let Some(start) = memory.allocate_pages(PageOwner::Process(kernel_pid), 4) {
            if let Some(p) = processes.get_mut(kernel_pid) {
                p.memory_pages = vec![start as u32];
            }
        }

        // Create init (pid=1)
        let init_pid = processes.create("init", 8);
        processes.set_state(init_pid, ProcessState::Ready);
        scheduler.add_process(init_pid);
        if let Some(start) = memory.allocate_pages(PageOwner::Process(init_pid), 2) {
            if let Some(p) = processes.get_mut(init_pid) {
                p.memory_pages = vec![start as u32];
            }
        }

        // Create shell (pid=2)
        let shell_pid = processes.create("shell", 5);
        processes.set_state(shell_pid, ProcessState::Ready);
        scheduler.add_process(shell_pid);
        if let Some(start) = memory.allocate_pages(PageOwner::Process(shell_pid), 2) {
            if let Some(p) = processes.get_mut(shell_pid) {
                p.memory_pages = vec![start as u32];
            }
        }

        let boot_messages = vec![
            "[BOOT] rust-minios v0.1.0 starting...".to_string(),
            "[MEM]  Initializing memory manager: 256 pages x 4KB = 1MB".to_string(),
            "[MEM]  Kernel reserved: pages 0-15 (64KB)".to_string(),
            "[FS]   Mounting virtual filesystem...".to_string(),
            "[FS]   Created: /kernel /home /tmp /dev".to_string(),
            "[SCHED] Initializing round-robin scheduler (quantum=10)".to_string(),
            "[PROC] Created process: kernel (PID=0)".to_string(),
            "[PROC] Created process: init (PID=1)".to_string(),
            "[PROC] Created process: shell (PID=2)".to_string(),
            "[IO]   IO subsystem ready".to_string(),
            "[BOOT] System ready. Type 'help' for commands.".to_string(),
        ];

        Kernel {
            memory,
            processes,
            scheduler,
            fs,
            io,
            tick: 0,
            boot_messages,
            config: KernelConfig {
                tick_rate_ms: 100,
                version: "0.1.0".to_string(),
            },
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.scheduler.tick(&mut self.processes);
    }

    pub fn dispatch(&mut self, syscall: Syscall) -> SyscallResult {
        match syscall {
            Syscall::Fork { name, priority } => {
                let pid = self.processes.create(&name, priority);
                self.scheduler.add_process(pid);
                self.processes.set_state(pid, ProcessState::Ready);
                if let Some(start) = self.memory.allocate_pages(PageOwner::Process(pid), 4) {
                    if let Some(p) = self.processes.get_mut(pid) {
                        p.memory_pages = vec![start as u32];
                    }
                }
                SyscallResult::Value(pid as i64)
            }
            Syscall::Exit { pid } => {
                self.scheduler.kill_process(pid, &mut self.processes);
                self.memory.free_process_pages(pid);
                SyscallResult::Success
            }
            Syscall::Kill { pid } => {
                self.scheduler.kill_process(pid, &mut self.processes);
                self.memory.free_process_pages(pid);
                SyscallResult::Success
            }
            Syscall::Exec { name } => {
                let pid = self.processes.create(&name, 5);
                self.scheduler.add_process(pid);
                self.processes.set_state(pid, ProcessState::Ready);
                SyscallResult::Value(pid as i64)
            }
            Syscall::GetPid => {
                let pid = self.scheduler.current.unwrap_or(0);
                SyscallResult::Value(pid as i64)
            }
            Syscall::ListProcesses => {
                let procs = self.processes.list();
                let mut output = String::new();
                for p in &procs {
                    output.push_str(&format!("PID={} NAME={} STATE={} PRI={} CPU={}\n",
                        p.pid, p.name, p.state, p.priority, p.cpu_time));
                }
                SyscallResult::Str(output)
            }
            Syscall::Malloc { size } => {
                let pages_needed = (size + 4095) / 4096;
                let pages_needed = pages_needed.max(1);
                match self.memory.allocate_pages(PageOwner::Process(999), pages_needed) {
                    Some(start) => SyscallResult::Value(start as i64),
                    None => SyscallResult::Err("Out of memory".to_string()),
                }
            }
            Syscall::Free { ptr } => {
                self.memory.free_pages(ptr, 1);
                SyscallResult::Success
            }
            Syscall::MemStats => {
                let stats = self.memory.get_stats();
                let output = format!(
                    "Total: {}KB | Kernel: {}KB | Process: {}KB | Free: {}KB | Reserved: {}KB",
                    stats.total * 4,
                    stats.used_kernel * 4,
                    stats.used_process * 4,
                    stats.free * 4,
                    stats.reserved * 4,
                );
                SyscallResult::Str(output)
            }
            Syscall::Open { path } => {
                match self.fs.resolve_path(&path) {
                    Some(_) => SyscallResult::Success,
                    None => SyscallResult::Err(format!("No such file: {}", path)),
                }
            }
            Syscall::Read { path } => {
                match self.fs.read_file(&path) {
                    Some(content) => SyscallResult::Str(content),
                    None => SyscallResult::Err(format!("Cannot read: {}", path)),
                }
            }
            Syscall::Write { path, content } => {
                if self.fs.write_file(&path, &content) {
                    SyscallResult::Success
                } else {
                    SyscallResult::Err(format!("Cannot write: {}", path))
                }
            }
            Syscall::Create { path } => {
                if self.fs.create_file(&path) {
                    SyscallResult::Success
                } else {
                    SyscallResult::Err(format!("Cannot create: {}", path))
                }
            }
            Syscall::CreateDir { path } => {
                if self.fs.create_dir(&path) {
                    SyscallResult::Success
                } else {
                    SyscallResult::Err(format!("Cannot create dir: {}", path))
                }
            }
            Syscall::Delete { path } => {
                if self.fs.delete(&path) {
                    SyscallResult::Success
                } else {
                    SyscallResult::Err(format!("Cannot delete: {}", path))
                }
            }
            Syscall::ListDir { path } => {
                let entries = self.fs.list_dir(&path);
                SyscallResult::Str(entries.join("\n"))
            }
            Syscall::GetTree => {
                SyscallResult::Str(self.fs.get_tree())
            }
        }
    }
}
