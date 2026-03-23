# 整體架構

## 專案結構

```
rust-minios/
├── Cargo.toml
└── src/
    ├── main.rs               # 進入點：初始化終端機、啟動 TUI 主迴圈
    ├── kernel/               # 作業系統核心
    │   ├── mod.rs            # Kernel 結構體、tick 主循環、系統呼叫派發
    │   ├── memory.rs         # 分頁式記憶體管理員
    │   ├── process.rs        # 程序控制區塊 (PCB) 與程序表
    │   ├── scheduler.rs      # Round-robin 排程器
    │   ├── fs.rs             # In-memory inode 虛擬檔案系統
    │   ├── io.rs             # IO 子系統與裝置抽象
    │   └── syscall.rs        # 系統呼叫定義
    └── ui/                   # TUI 使用者介面
        ├── mod.rs            # App 狀態機、主事件迴圈
        ├── dashboard.rs      # 儀表板面板分配
        ├── memory_view.rs    # 記憶體分頁視覺化
        ├── process_view.rs   # 程序排程器視覺化
        ├── fs_view.rs        # 檔案系統瀏覽器
        ├── editor.rs         # 文字編輯器
        └── shell.rs          # Shell 命令介面
```

## 分層架構

```
┌──────────────────────────────────────────────┐
│  Layer 4: UI Layer (ui/)                     │
│  使用者直接操作的介面，不含任何 OS 邏輯       │
├──────────────────────────────────────────────┤
│  Layer 3: Syscall Interface (syscall.rs)     │
│  UI 與 Kernel 之間唯一的溝通管道              │
├──────────────────────────────────────────────┤
│  Layer 2: Kernel Services (kernel/)          │
│  memory / scheduler / fs / io               │
├──────────────────────────────────────────────┤
│  Layer 1: Hardware Abstraction               │
│  crossterm 提供的終端機 I/O 抽象              │
└──────────────────────────────────────────────┘
```

## 主迴圈設計

```
main()
  │
  └─ App::run(terminal)
        │
        ├─ loop {
        │    ├─ terminal.draw(|f| render(f, &app))   ← 每幀重繪 TUI
        │    │
        │    ├─ if poll(100ms) {                      ← 非阻塞等待輸入
        │    │    event = read()
        │    │    app.handle_event(event)             ← 路由鍵盤事件
        │    │  }
        │    │
        │    └─ app.kernel.tick()                     ← 推進模擬時間
        │  }
        │
        └─ cleanup terminal
```

每 **100ms** 執行一次 `kernel.tick()`，推進排程器模擬一個時間單位（tick）。
使用者輸入與 OS 模擬完全非同步，互不阻塞。

## Kernel 結構體

```rust
pub struct Kernel {
    pub memory:       MemoryManager,  // 記憶體管理
    pub processes:    ProcessTable,   // 程序表
    pub scheduler:    Scheduler,      // 排程器
    pub fs:           FileSystem,     // 檔案系統
    pub io:           IoSubsystem,    // IO 子系統
    pub tick:         u64,            // 系統時鐘
    pub boot_messages: Vec<String>,   // 開機訊息
    pub config:       KernelConfig,   // 核心設定
}
```

Kernel 是整個 OS 狀態的**單一擁有者**（Single Source of Truth）。
所有子系統都透過 `&mut Kernel` 存取，保證 Rust 的借用規則始終成立。

## 開機序列

```
1. MemoryManager::new()     → 初始化 256 頁，前 16 頁標為 Kernel
2. ProcessTable::new()      → 空程序表
3. Scheduler::new(10)       → 時間量子 = 10 ticks
4. FileSystem::new()        → 建立目錄樹、初始檔案
5. 建立 kernel 程序 (PID=0) → 直接設為 Running
6. 建立 init 程序   (PID=1) → 加入 Ready Queue
7. 建立 shell 程序  (PID=2) → 加入 Ready Queue
8. 顯示開機訊息
```

## 模組依賴圖

```
main.rs
  └── ui/mod.rs (App)
        ├── ui/dashboard.rs
        │     ├── ui/memory_view.rs   ──→ kernel::memory
        │     ├── ui/process_view.rs  ──→ kernel::{process, scheduler}
        │     ├── ui/fs_view.rs       ──→ kernel::fs
        │     ├── ui/editor.rs        ──→ kernel (via syscall)
        │     └── ui/shell.rs         ──→ kernel (via syscall)
        └── kernel/mod.rs (Kernel)
              ├── kernel/memory.rs
              ├── kernel/process.rs
              ├── kernel/scheduler.rs
              ├── kernel/fs.rs
              ├── kernel/io.rs
              └── kernel/syscall.rs
```
