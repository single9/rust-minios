# 程序與排程器

## 程序的概念

**程序（Process）** 是作業系統資源分配的基本單位。
每個程序有自己的：
- 唯一識別碼（PID）
- 執行狀態
- 優先權
- 記憶體空間

## 程序控制區塊 (PCB)

作業系統用 **PCB（Process Control Block）** 記錄每個程序的一切資訊：

```rust
pub struct Process {
    pub pid:          u32,          // 程序識別碼（唯一）
    pub name:         String,       // 程序名稱（如 "shell", "init"）
    pub state:        ProcessState, // 當前狀態
    pub priority:     u8,           // 優先權 0~10（10 最高）
    pub cpu_time:     u64,          // 累計使用 CPU 的 tick 數
    pub memory_pages: Vec<u32>,     // 擁有的記憶體分頁起始頁碼
    pub created_at:   u64,          // 建立時的系統 tick
}
```

## 程序狀態機

每個程序在生命週期中會在以下狀態之間轉換：

```
                    ┌─────────┐
         建立       │   New   │
         ─────────→ └────┬────┘
                         │ 加入就緒佇列
                         ▼
      時間量子到期   ┌─────────┐   被排程器選中
     ┌─────────────  │  Ready  │ ←──────────────┐
     │              └────┬────┘                 │
     │                   │ 排程器選中            │
     ▼                   ▼                      │
┌─────────┐        ┌─────────┐      等待 I/O    │
│  Ready  │        │ Running │ ────────────────→│
└─────────┘        └────┬────┘                 │
                         │                  ┌──────────┐
                         │ 主動呼叫 exit     │ Blocked  │
                         ▼                  │ (等待中) │
                    ┌──────────┐            └──────────┘
                    │Terminated│
                    └──────────┘
```

| 狀態 | 說明 |
|------|------|
| `New` | 剛建立，尚未加入排程 |
| `Ready` | 等待 CPU，已在就緒佇列中 |
| `Running` | 正在使用 CPU |
| `Blocked` | 等待 I/O 或事件，暫停排程 |
| `Terminated` | 已結束，資源待回收 |

## 程序表

`ProcessTable` 使用 `HashMap<u32, Process>` 儲存所有程序：

```rust
pub struct ProcessTable {
    pub processes: HashMap<u32, Process>,
    next_pid: u32,  // 自動遞增的 PID 計數器
}
```

## 排程器

### Round-Robin 演算法

Round-Robin 是最基本的公平排程策略：
每個程序依序輪流使用 CPU，每次最多使用固定**時間量子（Time Quantum）**。

```
時間量子 = 10 ticks

就緒佇列: [PID=1] [PID=2] [PID=3]

Tick  1-10:  PID=1 Running  (quantum 用完)
              → PID=1 preempt → 回到佇列尾端
Tick 11-20:  PID=2 Running  (quantum 用完)
              → PID=2 preempt → 回到佇列尾端
Tick 21-30:  PID=3 Running  (quantum 用完)
Tick 31-40:  PID=1 Running  (再次輪到)
...
```

### 排程器資料結構

```rust
pub struct Scheduler {
    pub ready_queue:     VecDeque<u32>,  // 就緒佇列（FIFO）
    pub blocked:         Vec<u32>,       // 被阻塞的程序
    pub current:         Option<u32>,    // 目前正在執行的 PID
    pub time_quantum:    u32,            // 時間量子（預設 10）
    pub tick:            u64,            // 排程器時鐘
    pub current_quantum: u32,            // 目前程序已用的 tick 數
}
```

### tick() 的執行流程

每次呼叫 `scheduler.tick()` 模擬一個 CPU 時鐘週期：

```
tick() {
  1. 若有正在執行的程序：
     a. 其 cpu_time += 1
     b. current_quantum += 1
     c. 若 current_quantum >= time_quantum：
        → 將當前程序狀態改為 Ready
        → 推回 ready_queue 尾端
        → current = None（搶佔）

  2. 若 current 為空：
     a. 從 ready_queue 取出第一個 PID
     b. 設為 Running
     c. current_quantum = 0
}
```

### Context Switch（上下文切換）

真實 OS 在切換程序時需要儲存/恢復 CPU 暫存器（registers）。
在模擬器中，我們簡化為只更新 `ProcessState` 與 `Scheduler.current`。

```
搶佔（Preempt）發生時：
  舊程序: Running → Ready  （狀態回到就緒）
  新程序: Ready  → Running  （狀態變為執行中）
  
  只需更新 ProcessState，沒有實際的 CPU 上下文需要儲存。
```

## 開機時的初始程序

| PID | 名稱 | 初始狀態 | 優先權 | 說明 |
|-----|------|---------|--------|------|
| 0 | kernel | Running | 10 | 核心程序，永遠「佔用」CPU |
| 1 | init | Ready | 8 | 系統初始化程序 |
| 2 | shell | Ready | 5 | 使用者 Shell |

## TUI 視覺化

程序視圖（F3）顯示：

```
┌─ 程序排程器 ────────────────────────────────────────┐
│  PID  NAME    STATE    PRI  CPU TIME  MEMORY        │
│  ─────────────────────────────────────────────────  │
│  0    kernel  Running   10   1523     16KB          │  ← 綠色
│  1    init    Ready      8    340      8KB          │  ← 黃色
│  2    shell   Ready      5    218      8KB          │  ← 黃色
│                                                      │
│  就緒佇列: [init] → [shell]                          │
│  Blocked:  (empty)                                   │
│  Scheduler Tick: 1523                                │
└──────────────────────────────────────────────────────┘
```

顏色代碼：
- 🟢 綠色 = Running
- 🟡 黃色 = Ready
- 🔴 紅色 = Blocked
- ⬜ 灰色 = Terminated

## 相關指令

```bash
ps               # 列出所有程序（等同 ListProcesses 系統呼叫）
exec <name>      # 建立並啟動新程序
kill <pid>       # 終止指定程序（回收記憶體）
```

## 與真實 OS 的對比

| 概念 | 真實 OS (Linux) | rust-minios |
|------|----------------|-------------|
| 排程演算法 | CFS (Completely Fair Scheduler) | Round-Robin |
| 時間量子 | 動態（依負載調整）| 固定 10 ticks |
| 優先權 | nice value -20~19 | 0~10 |
| 上下文切換 | 儲存/還原 CPU 暫存器 | 只更新狀態 |
| 多核心 | 有（Per-CPU 執行佇列）| 無（單核模擬）|
| 即時排程 | SCHED_FIFO / SCHED_RR | 無 |
