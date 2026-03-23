# 記憶體管理

## 概念：分頁式記憶體 (Paged Memory)

真實作業系統將實體記憶體切割成固定大小的**分頁（Page）**，
以分頁為單位進行配置與回收，避免外部碎片化。

rust-minios 模擬 **1MB 實體記憶體**：

```
總記憶體 = 256 頁 × 4KB = 1,048,576 bytes = 1MB

Page 0   Page 1   Page 2   ...   Page 255
┌──────┐ ┌──────┐ ┌──────┐       ┌──────┐
│ 4KB  │ │ 4KB  │ │ 4KB  │  ...  │ 4KB  │
└──────┘ └──────┘ └──────┘       └──────┘
```

## 資料結構

```rust
// 每個分頁的擁有者
pub enum PageOwner {
    Free,           // 可供配置
    Kernel,         // 核心保留
    Process(u32),   // 被某個程序使用（u32 = PID）
    Reserved,       // 硬體保留
}

// 每個分頁的元資料
pub struct PageInfo {
    pub owner: PageOwner,
}

// 記憶體管理員
pub struct MemoryManager {
    pub pages: Vec<PageInfo>,  // 長度固定為 256
}
```

## 配置演算法：連續首次適配 (Contiguous First-Fit)

```rust
pub fn allocate_pages(&mut self, owner: PageOwner, count: usize) -> Option<usize>
```

搜尋足夠大的**連續空閒分頁區塊**，回傳起始頁碼：

```
要求 3 頁，當前狀態：

Page:  0  1  2  3  4  5  6  7  8  9
Owner: K  K  K  P0 P0 .  .  .  P1 P1
                        ↑
                   找到 page 5 開始有 3 個連續空閒頁

配置後：
Page:  0  1  2  3  4  5  6  7  8  9
Owner: K  K  K  P0 P0 N  N  N  P1 P1
                       ───────
                       回傳 start=5
```

若找不到足夠連續空間，回傳 `None`（Out of Memory）。

## 記憶體釋放

```rust
// 釋放從 start 開始的 count 個分頁
pub fn free_pages(&mut self, start: usize, count: usize)

// 釋放某個 PID 擁有的所有分頁（程序終止時呼叫）
pub fn free_process_pages(&mut self, pid: u32)
```

程序終止時，kernel 自動呼叫 `free_process_pages`，
將所有屬於該程序的分頁歸還為 `Free`。

## 初始化狀態

```
開機後的記憶體佈局：

Page 00-15:  KKKKKKKKKKKKKKKK  (Kernel 保留 = 64KB)
Page 16-19:  PPPP              (kernel 程序 PID=0, 4頁)
Page 20-21:  PP                (init   程序 PID=1, 2頁)
Page 22-23:  PP                (shell  程序 PID=2, 2頁)
Page 24-255: ............      (Free = 928KB)
```

## 記憶體統計

```rust
pub struct MemStats {
    pub total:        usize,  // 256 頁
    pub used_kernel:  usize,  // Kernel 使用頁數
    pub used_process: usize,  // 所有程序使用頁數
    pub free:         usize,  // 空閒頁數
    pub reserved:     usize,  // 保留頁數
}
```

Shell 指令 `free` 會顯示：
```
Total: 1024KB | Kernel: 64KB | Process: 32KB | Free: 928KB | Reserved: 0KB
```

## TUI 視覺化

記憶體視圖（F2）將 256 個分頁顯示為 16×16 彩色方塊矩陣：

```
     00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
00:  ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  ← Kernel (藍色)
10:  ██ ██ ██ ██ ▓▓ ▓▓ ▓▓ ▓▓ ▓▓ ▓▓ ▓▓ ▓▓ ░░ ░░ ░░ ░░  ← Process (綠)/Free (灰)
20:  ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░
...

Legend: ██ Kernel  ▓▓ Process  ░░ Free
```

## 與真實 OS 的對比

| 概念 | 真實 OS | rust-minios |
|------|---------|-------------|
| 分頁大小 | 4KB（x86）| 4KB（模擬）|
| 總記憶體 | GB 級 | 1MB（256頁）|
| 配置演算法 | Buddy System / Slab | First-Fit |
| 虛擬記憶體 | 有（分頁表、TLB）| 無（直接實體）|
| 保護機制 | Ring 0/3、NX bit | 無（純教育模擬）|

## 相關指令

```bash
free              # 顯示記憶體使用統計
malloc <頁數>     # 手動配置指定頁數的記憶體
memfree <起始頁>  # 釋放從起始頁開始的記憶體
```
