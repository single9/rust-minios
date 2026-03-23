# 虛擬檔案系統 (VFS)

## 概念：inode 檔案系統

**inode（Index Node）** 是 Unix 檔案系統的核心資料結構。
每個檔案或目錄都對應一個 inode，儲存其元資料（metadata）與資料。

```
傳統磁碟上的 inode 系統：

磁碟
├── inode table (固定區域)
│     inode#5: {type=file, size=1024, blocks=[12,13,14]}
│     inode#7: {type=dir,  entries=[(readme, 5), (src, 8)]}
│
└── data blocks (資料區)
      block#12: "Hello Wor"
      block#13: "ld!\n     "
      ...
```

rust-minios 使用 **in-memory HashMap** 模擬相同概念，
省去真實磁碟的區塊尋址，保留 inode 的邏輯結構。

## 資料結構

```rust
pub enum InodeType {
    File,
    Directory,
}

pub struct Inode {
    pub id:         u32,          // inode 編號（唯一）
    pub name:       String,       // 名稱（不含路徑）
    pub inode_type: InodeType,    // 檔案 or 目錄
    pub content:    Vec<u8>,      // 檔案內容（目錄此欄為空）
    pub children:   Vec<u32>,     // 子項目的 inode ID（目錄才有）
    pub parent:     Option<u32>,  // 父目錄 inode ID
    pub size:       usize,        // 內容大小（bytes）
    pub created_at: u64,          // 建立時的系統 tick
}

pub struct FileSystem {
    pub inodes:     HashMap<u32, Inode>,  // inode 表
    pub next_inode: u32,                  // 下一個可用 inode 編號
}
```

## 目錄樹結構

開機後的預設目錄樹：

```
/ (inode=0, Directory)
├── kernel/  (inode=1)      ← 核心相關檔案
├── home/    (inode=2)      ← 使用者家目錄
│     ├── readme.txt (inode=5)  "Welcome to rust-minios!\n..."
│     └── hello.txt  (inode=6)  "Hello, World!\n"
├── tmp/     (inode=3)      ← 暫存檔案
└── dev/     (inode=4)      ← 裝置檔案
```

## 路徑解析 (Path Resolution)

`resolve_path("/home/readme.txt")` 的執行過程：

```
1. 從根目錄 inode=0 開始
2. 分割路徑: ["home", "readme.txt"]
3. 在 inode=0 的 children 中找名為 "home" 的項目
   → 找到 inode=2
4. 在 inode=2 的 children 中找名為 "readme.txt" 的項目
   → 找到 inode=5
5. 回傳 Some(5)
```

```rust
pub fn resolve_path(&self, path: &str) -> Option<u32> {
    // 從 "/" (inode=0) 開始，逐段解析
    let parts = path.trim_start_matches('/').split('/');
    let mut current = 0u32;  // 從根目錄開始
    for part in parts {
        // 在當前目錄的 children 中找 part
        current = find_child(current, part)?;
    }
    Some(current)
}
```

時間複雜度：O(depth × children_per_dir)

## 檔案操作

### 讀取檔案 (read_file)

```
resolve_path(path) → inode_id
→ inodes[inode_id].content
→ Vec<u8> → String::from_utf8_lossy
```

### 寫入檔案 (write_file)

```
resolve_path(path) → inode_id
→ inodes[inode_id].content = new_content.as_bytes()
→ inodes[inode_id].size = content.len()
```

### 建立檔案 (create_file)

```
split_path("/home/notes.txt") → ("/home", "notes.txt")
→ resolve_path("/home") → parent_id=2
→ alloc_inode("notes.txt", File, parent=2) → new_id
→ inodes[2].children.push(new_id)
```

### 刪除 (delete)

```
resolve_path(path) → target_id
→ inodes[target_id].parent → parent_id
→ inodes[parent_id].children.retain(|&c| c != target_id)
→ inodes.remove(target_id)
```

> ⚠️ 注意：本模擬器的刪除不遞迴刪除子目錄。

## 目錄樹顯示 (tree)

`get_tree()` 使用深度優先搜尋（DFS）遍歷整個 inode 樹：

```
/
├── kernel/
├── home/
│   ├── readme.txt
│   └── hello.txt
├── tmp/
└── dev/
```

## 與真實 FS 的對比

| 概念 | 真實 ext4 | rust-minios |
|------|-----------|-------------|
| inode 儲存 | 磁碟固定區域 | HashMap (記憶體) |
| 資料儲存 | 磁碟 blocks | Vec<u8> (記憶體) |
| 路徑快取 | dcache | 無 |
| 硬連結 | 多個 dentry 指向同 inode | 無 |
| 符號連結 | 特殊 inode 含路徑字串 | 無 |
| 持久化 | 是（斷電不遺失）| 否（程式關閉即消失）|
| 日誌 (Journal) | 有 | 無 |
| 權限 | rwxrwxrwx | 無 |

## TUI 視覺化

檔案系統視圖（F4）：

```
┌─ 檔案系統瀏覽器 ──────────────────────────────────┐
│  /                                                 │
│  ├── kernel/                                       │
│  ├── home/                                        │
│  │   ├── readme.txt    [文字, 60B]                │  ← 選中（高亮）
│  │   └── hello.txt     [文字, 14B]                │
│  ├── tmp/                                         │
│  └── dev/                                         │
└───────────────────────────────────────────────────┘
```

## 相關指令

```bash
ls [路徑]         # 列出目錄內容
cat <檔案>        # 顯示檔案內容
tree              # 顯示完整目錄樹
mkdir <目錄>      # 建立目錄
touch <檔案>      # 建立空檔案
rm <路徑>         # 刪除檔案或目錄
edit <檔案>       # 在編輯器中開啟（見 tui-visualization.md）
```
