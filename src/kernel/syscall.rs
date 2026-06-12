pub enum SyscallResult {
    Success,
    Err(String),
    Value(i64),
    Str(String),
    #[expect(dead_code)]
    Bytes(Vec<u8>),
}

pub enum Syscall {
    Fork {
        name: String,
        priority: u8,
    },
    #[expect(dead_code)]
    Exit {
        pid: u32,
    },
    Kill {
        pid: u32,
    },
    #[expect(dead_code)]
    Exec {
        name: String,
    },
    #[expect(dead_code)]
    GetPid,
    ListProcesses,
    Malloc {
        size: usize,
    },
    #[expect(dead_code)]
    Free {
        ptr: usize,
    },
    MemStats,
    Open {
        path: String,
    },
    Read {
        path: String,
    },
    Write {
        path: String,
        content: String,
    },
    Create {
        path: String,
    },
    CreateDir {
        path: String,
    },
    Delete {
        path: String,
    },
    ListDir {
        path: String,
    },
    GetTree,
}
