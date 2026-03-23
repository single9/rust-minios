pub enum SyscallResult {
    Success,
    Err(String),
    Value(i64),
    Str(String),
    Bytes(Vec<u8>),
}

pub enum Syscall {
    Fork { name: String, priority: u8 },
    Exit { pid: u32 },
    Kill { pid: u32 },
    Exec { name: String },
    GetPid,
    ListProcesses,
    Malloc { size: usize },
    Free { ptr: usize },
    MemStats,
    Open { path: String },
    Read { path: String },
    Write { path: String, content: String },
    Create { path: String },
    CreateDir { path: String },
    Delete { path: String },
    ListDir { path: String },
    GetTree,
}
