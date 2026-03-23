# IO 子系統與系統呼叫

## IO 子系統

### 概念

**IO 子系統（I/O Subsystem）** 負責管理程序與周邊裝置（鍵盤、螢幕、磁碟等）之間的資料流。
核心提供統一介面，讓程序不需要直接操作硬體。

### 資料結構

```rust
// IO 事件類型
pub enum IoEvent {
    KeyPress(char),         // 一般字元按鍵
    KeySpecial(SpecialKey), // 特殊按鍵
    DeviceReady(u32),       // 裝置 ID 就緒通知
}

pub enum SpecialKey {
    Enter, Backspace, Up, Down, Left, Right,
    Tab, Escape, F(u8),
}

pub struct IoSubsystem {
    pub event_queue:   VecDeque<IoEvent>,  // 輸入事件佇列
    pub output_buffer: Vec<String>,        // 輸出緩衝區
}
```

### 事件佇列模型

```
使用者按鍵
    │
    ▼  (crossterm)
IoSubsystem::push_event(IoEvent::KeyPress('a'))
    │
    ▼
event_queue: [KeyPress('a'), KeyPress('b'), Enter, ...]
    │
    ▼  app.handle_event() 每幀處理
pop_event() → 取出事件 → 路由到對應 UI 元件
```

### 輸出緩衝

```
程序呼叫 write() 系統呼叫
    │
    ▼
IoSubsystem::write_output("Hello, World!")
    │
    ▼
output_buffer: ["Hello, World!", "Another line", ...]
    │
    ▼  TUI 渲染時讀取
drain_output() → 清空並回傳所有待顯示文字
```

---

## 系統呼叫 (Syscall)

### 概念

**系統呼叫（System Call）** 是使用者程式進入核心模式的唯一合法途徑。
程式透過系統呼叫請求核心提供服務（記憶體、檔案、程序管理等）。

```
使用者程式
    │
    │  int 0x80 / syscall 指令（真實 x86）
    │  kernel.dispatch(Syscall::...)（模擬器）
    │
    ▼
核心模式
    │
    ├─ 驗證參數
    ├─ 執行操作
    └─ 回傳結果
```

### 系統呼叫列表

```rust
pub enum Syscall {
    // ── 程序管理 ──────────────────────────────
    Fork { name: String, priority: u8 }, // 建立子程序
    Exec { name: String },               // 執行新程式（簡化版）
    Exit { pid: u32 },                   // 程序主動結束
    Kill { pid: u32 },                   // 強制終止程序
    GetPid,                              // 取得當前程序 PID
    ListProcesses,                       // 列出所有程序

    // ── 記憶體管理 ────────────────────────────
    Malloc { size: usize },  // 配置記憶體（回傳起始頁碼）
    Free   { ptr: usize },   // 釋放記憶體
    MemStats,                // 取得記憶體統計

    // ── 檔案系統 ──────────────────────────────
    Open      { path: String },                   // 開啟檔案
    Read      { path: String },                   // 讀取檔案內容
    Write     { path: String, content: String },  // 寫入檔案
    Create    { path: String },                   // 建立檔案
    CreateDir { path: String },                   // 建立目錄
    Delete    { path: String },                   // 刪除
    ListDir   { path: String },                   // 列出目錄
    GetTree,                                      // 取得目錄樹
}
```

### 回傳值

```rust
pub enum SyscallResult {
    Success,           // 操作成功，無需回傳值
    Err(String),       // 錯誤訊息
    Value(i64),        // 整數值（如 PID、頁碼）
    Str(String),       // 字串（如檔案內容、程序列表）
    Bytes(Vec<u8>),    // 原始位元組
}
```

### 派發流程

```
Shell 輸入: "cat /home/readme.txt"
    │
    ▼  shell.execute_command()
Syscall::Read { path: "/home/readme.txt" }
    │
    ▼  kernel.dispatch(syscall)
match Syscall::Read { path } {
    Some(content) → SyscallResult::Str(content)
    None          → SyscallResult::Err("Cannot read: ...")
}
    │
    ▼  shell 顯示結果
"Welcome to rust-minios!\nA micro OS simulator..."
```

### Fork vs Exec（Unix 哲學）

真實 Unix 中，程序建立分兩步：
1. `fork()` — 複製當前程序（包含記憶體）
2. `exec()` — 用新程式覆蓋當前程序映像

模擬器的簡化版：
- `Fork` — 建立新 PCB + 配置記憶體 + 加入就緒佇列
- `Exec` — 直接建立新程序（跳過 fork 的複製步驟）

```
真實 Unix:                    rust-minios:
shell ──fork()──→ shell複本   ─────→ 新 Process 物件
                    │                   │
                 exec("ls")          設為 Ready
                    │                   │
                  ls 程式            加入 ready_queue
```

### malloc 的實作

```
malloc(size=12288)  ← 要求 12KB
    │
    ▼  計算需要幾頁
pages_needed = ceil(12288 / 4096) = 3 頁
    │
    ▼  呼叫記憶體管理員
memory.allocate_pages(owner=Process(999), count=3)
    │
    ├─ 成功 → SyscallResult::Value(start_page)
    └─ 失敗 → SyscallResult::Err("Out of memory")
```

## 系統呼叫對照表（Shell 指令 → Syscall）

| Shell 指令 | 對應 Syscall | 說明 |
|-----------|-------------|------|
| `exec <name>` | `Fork { name, priority: 5 }` | 建立程序 |
| `kill <pid>` | `Kill { pid }` | 終止程序 |
| `ps` | `ListProcesses` | 程序列表 |
| `free` | `MemStats` | 記憶體統計 |
| `malloc <n>` | `Malloc { size: n*4096 }` | 配置記憶體 |
| `ls [path]` | `ListDir { path }` | 列出目錄 |
| `cat <file>` | `Read { path }` | 讀取檔案 |
| `touch <file>` | `Create { path }` | 建立檔案 |
| `mkdir <dir>` | `CreateDir { path }` | 建立目錄 |
| `rm <path>` | `Delete { path }` | 刪除 |
| `tree` | `GetTree` | 目錄樹 |
| `edit <file>` (儲存) | `Write { path, content }` | 寫入檔案 |

## 與真實 OS 的對比

| 概念 | 真實 Linux | rust-minios |
|------|-----------|-------------|
| 呼叫方式 | `syscall` 指令（x86-64）| Rust enum dispatch |
| 系統呼叫號 | 整數（如 read=0, write=1）| Enum variant |
| 權限切換 | Ring 3 → Ring 0 | 無（純模擬）|
| 阻塞 I/O | 程序進入 Blocked 狀態 | 直接同步完成 |
| 信號（Signal）| 有（SIGKILL, SIGTERM...）| 無 |
| 管道（Pipe）| 有 | 無 |
