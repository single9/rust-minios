# Shell 腳本語言

rust-minios 的 Shell 支援一個輕量腳本語言，可以將多個指令寫成腳本檔案儲存在 VFS 中，透過 `run` 指令執行。

## 快速開始

```bash
# 用編輯器建立腳本
edit /home/hello.sh
```

在編輯器中輸入：

```sh
# 我的第一個腳本
NAME=rust-minios
echo Hello from $NAME!
```

儲存後（`Ctrl+S`）回到 Shell 執行：

```bash
run /home/hello.sh
```

也可以直接試試內建的示範腳本：

```bash
run /home/demo.sh
```

---

## 運作原理

### 整體流程

```
run /home/script.sh
     │
     ▼  Syscall::Read
從 VFS 讀取腳本文字
     │
     ▼  run_script(text, kernel)
┌─────────────────────────────────────────┐
│  直譯器主迴圈 (Interpreter Loop)         │
│                                         │
│  ip = 0  ←─────────────────────┐        │
│  while ip < lines.len()        │        │
│    line = lines[ip]            │        │
│    ip += 1  ──────────────────-┘        │
│    expand_vars(line)                    │
│    match 第一個詞 {                      │
│      VAR=  → 存入 vars HashMap          │
│      if    → 收集區塊 → 遞迴呼叫        │
│      for   → 收集區塊 → 迴圈遞迴呼叫   │
│      其他  → execute_command()          │
│    }                                    │
└─────────────────────────────────────────┘
     │
     ▼  execute_command(line, kernel)
kernel.dispatch(Syscall::...)
```

這是一個**直譯器（Interpreter）**，沒有編譯步驟。腳本文字被逐行讀取、即時執行，不產生任何中間表示（IR）或位元組碼。

---

### 步驟一：變數展開 (`expand_vars`)

**在每一行執行前**，直譯器先對整行文字做變數替換。

實作使用一個字元迭代器逐字掃描：

```
輸入: "exec $NAME-worker at $HOST"
       e x e c   $ N A M E - w o r k e r ...

掃到普通字元 → 直接加入結果
掃到 '$'    → 繼續讀取後面的 [a-zA-Z0-9_] 字元組成變數名
              → 查 vars HashMap 取得值並替換

假設 vars = { "NAME": "web", "HOST": "prod" }
結果: "exec web-worker at prod"
```

未定義的變數展開為空字串（不報錯）：

```
$UNDEFINED  →  ""（靜默替換）
```

---

### 步驟二：直譯器主迴圈 (`run_script`)

主迴圈用一個整數 `ip`（Instruction Pointer，指令指標）記錄目前執行到第幾行：

```rust
let lines: Vec<&str> = script.lines().collect();
let mut ip = 0;

while ip < lines.len() {
    let line = lines[ip].trim();
    ip += 1;   // 每次先遞增，再處理當前行
    // ...
}
```

這模仿了真實 CPU 的 **PC（Program Counter）** 概念：每執行一條指令，PC 自動指向下一條。遇到 `if`/`for` 時，`ip` 會被推進到對應 `end` 之後，跳過整個區塊的原始行（因為區塊已被收集起來遞迴處理）。

每行依序做以下判斷：

```
1. 空行或 # 開頭 → 跳過
2. expand_vars()
3. 符合 KEY=value 格式 → 存入 vars，continue
4. 第一個詞為 "if"    → 進入 if 處理流程
5. 第一個詞為 "for"   → 進入 for 處理流程
6. 其他              → execute_command()
```

---

### 步驟三：`if` 的解析與執行

`if` **不使用跳轉（jump）**，而是「先收集、再遞迴」：

```
if EXISTS == true      ← 1. 立刻評估條件
  echo yes              ←┐
  exec worker           ← ┤ 2. ip 繼續往下掃，
else                   ← ┤    把這些行收進 if_body
  echo no              ←┤ 3. 遇到 else → 切換到 else_body
end                    ← 4. depth==0 → 停止收集

5. 根據條件結果，選擇 if_body 或 else_body
6. 用 run_script() 遞迴執行選中的那組行
```

**depth 計數器**處理巢狀結構：

```
if A          depth=1
  if B        depth=2  ← 遇到新的 if/for，depth+1
    echo x
  end         depth=1  ← 遇到 end，depth-1，但 depth≠0，繼續收集
end           depth=0  ← depth 歸零，停止收集
```

沒有 depth 計數器的話，內層 `end` 會被誤認為外層 `if` 的結束。

---

### 步驟四：`for` 的解析與執行

`for` 同樣先收集 body，然後對每個值**重複遞迴呼叫** `run_script`：

```
for W in alpha beta gamma
  exec $W            ← body 收集到 loop_body
  echo Started $W
end

執行過程：
  iter 1: vars["W"] = "alpha" → run_script("exec $W\necho Started $W", kernel)
  iter 2: vars["W"] = "beta"  → run_script("exec $W\necho Started $W", kernel)
  iter 3: vars["W"] = "gamma" → run_script("exec $W\necho Started $W", kernel)
```

每次迭代前，`W` 被寫入 `vars` HashMap，下一次 `expand_vars` 就會取到新值。

---

### 步驟五：條件評估 (`eval_condition`)

`eval_condition` 用**模式匹配**判斷條件類型，不建構 AST：

```
條件字串
  │
  ├─ 含 "==" → 分割左右，各自 expand_vars，字串比較
  ├─ 含 "!=" → 同上，取反
  ├─ 以 "exists " 開頭 → 解析路徑 → kernel.dispatch(Syscall::Open)
  │                       成功 → true，錯誤 → false
  └─ 其他 → expand_vars 後，非空且非 "0" 非 "false" → true
```

`exists` 條件是唯一會呼叫 kernel 系統呼叫的條件，透過 `Syscall::Open` 查詢 VFS inode 是否存在。

---

### 遞迴結構總覽

```
run_script("for ... / if ... / end")
  │
  ├─ if: run_script(if_body)
  │        └─ if: run_script(nested_if_body)  ← 可無限巢狀
  │
  └─ for: run_script(loop_body) × N 次
             └─ if: run_script(if_body_inside_loop)
```

每層遞迴共享同一個 `Shell` 結構（包含 `vars` HashMap），
所以內層腳本修改的變數，外層可以繼續使用。

---

### 與 Kernel 的關係

腳本引擎本身不操作任何 OS 資源，全部透過 `execute_command` → `kernel.dispatch(Syscall::...)` 間接完成：

```
腳本行 "exec worker"
  → execute_command("exec worker", kernel)
  → kernel.dispatch(Syscall::Fork { name: "worker", priority: 5 })
  → ProcessTable 建立新 PCB
  → Scheduler 加入 ready_queue
  → 回傳 SyscallResult::Value(pid)
  → 輸出 "Started process 'worker' with PID=3"
```

腳本語言完全不知道 PCB、記憶體分頁或 VFS inode 的存在，這是**分層抽象**的體現。

---

## 語法參考

### 註解

以 `#` 開頭的行會被忽略：

```sh
# 這是一行注解
echo Hello  # 行尾不支援行內註解
```

---

### 變數

#### 賦值

```sh
VAR=value
```

- 變數名稱只能包含英文字母、數字、底線
- `=` 兩側不能有空格
- 值會原樣儲存為字串

```sh
NAME=Alice
COUNT=42
PATH_PREFIX=/tmp/data
```

#### 展開

在任何位置使用 `$VAR` 展開變數：

```sh
NAME=world
echo Hello $NAME       # 輸出：Hello world
mkdir /tmp/$NAME       # 建立 /tmp/world
exec $NAME-worker      # 啟動 world-worker 程序
```

#### 在 Shell 互動模式中設定變數

```bash
set NAME=value         # 設定變數
set                    # 列出所有變數
unset NAME             # 刪除變數
NAME=value             # 也可直接在 Shell 輸入賦值語句
```

變數在 Shell 的整個生命週期中持續存在，腳本執行完畢後變數仍會保留。

---

### if / else / end

```sh
if <條件>
  # 條件成立時執行
else
  # 條件不成立時執行（可省略）
end
```

#### 條件類型

**字串相等 `==`**

```sh
MODE=release
if $MODE == release
  echo Building in release mode
end
```

**字串不等 `!=`**

```sh
ENV=prod
if $ENV != test
  echo Not in test environment
end
```

**檔案/目錄是否存在 `exists`**

```sh
if exists /tmp/output.txt
  cat /tmp/output.txt
else
  echo File not found
end
```

**變數是否非空（直接寫變數名稱）**

非空且不為 `0`/`false` 時視為 true：

```sh
RESULT=ok
if $RESULT
  echo Result is set
end
```

#### 巢狀 if

```sh
if exists /tmp
  if exists /tmp/data.txt
    echo Both exist!
  else
    echo tmp exists but data.txt does not
  end
end
```

---

### for / end

```sh
for VAR in 值1 值2 值3
  # 每次迭代執行
end
```

- `VAR` 會依序被設為每個值
- 每個值以空白分隔

```sh
# 啟動三個 worker 程序
for W in alpha beta gamma
  exec $W
  echo Started: $W
end

# 建立多個目錄
for DIR in logs cache tmp/data
  mkdir /home/$DIR
end

# 配置多次記憶體
for I in 1 2 3
  malloc 4096
  echo Allocated block $I
end
```

#### 巢狀 for

```sh
for HOST in web db cache
  for ENV in staging prod
    echo Deploy $HOST to $ENV
  end
end
```

---

### 可用指令

腳本中可以使用所有 Shell 的內建指令：

| 指令 | 說明 |
|------|------|
| `echo <text>` | 輸出文字（支援 `$VAR`）|
| `ls [path]` | 列出目錄 |
| `cat <file>` | 讀取檔案內容 |
| `mkdir <dir>` | 建立目錄 |
| `touch <file>` | 建立空檔案 |
| `rm <path>` | 刪除檔案或目錄 |
| `exec <name>` | 啟動新程序 |
| `kill <pid>` | 終止程序 |
| `ps` | 列出所有程序 |
| `free` | 顯示記憶體統計 |
| `malloc <bytes>` | 配置記憶體 |
| `tree` | 顯示目錄樹 |
| `cd <path>` | 切換工作目錄 |
| `set VAR=value` | 設定變數 |
| `run <file>` | 執行另一個腳本（可巢狀）|

---

## 完整範例

### 範例一：系統初始化腳本

```sh
# /home/init.sh
# 系統啟動後的初始化設定

echo === System Init ===

# 建立工作目錄
mkdir /tmp/logs
mkdir /tmp/cache
touch /tmp/logs/system.log

# 啟動背景服務
for SVC in logger monitor cleaner
  exec $SVC
  echo Service started: $SVC
end

# 確認服務已啟動
echo --- Running processes ---
ps

echo === Init complete ===
```

### 範例二：記憶體壓力測試

```sh
# /home/memtest.sh
# 配置多塊記憶體並顯示使用狀況

echo === Memory Stress Test ===
free

COUNT=1
for SIZE in 4096 8192 16384 4096 8192
  echo Allocating block $COUNT (${SIZE} bytes)
  malloc $SIZE
  COUNT=$COUNT
end

echo --- After allocation ---
free
echo === Test done ===
```

### 範例三：條件式部署

```sh
# /home/deploy.sh
# 根據環境條件決定行為

ENV=staging

echo Deploying to: $ENV

if $ENV == prod
  echo WARNING: Production deployment!
  exec prod-health-check
else
  echo Staging deployment, running tests...
  exec test-runner
end

if exists /tmp/deploy.lock
  echo Deploy already in progress!
else
  touch /tmp/deploy.lock
  exec deploy-worker
  echo Deploy started.
end
```

### 範例四：巢狀腳本

腳本可以呼叫其他腳本：

```sh
# /home/main.sh
echo Running setup...
run /home/init.sh

echo Running tests...
run /home/memtest.sh

echo All done!
```

---

## 注意事項

| 限制 | 說明 |
|------|------|
| 無管道 `\|` | 不支援將輸出導向另一個指令 |
| 無重導向 `>` | 不支援將輸出寫入檔案 |
| 無函式 | 不支援自訂函式定義 |
| 無算術 | 不支援數值運算（`$((1+1))` 不可用）|
| 無 while | 僅支援 `for` 迴圈 |
| 變數皆為字串 | 所有變數值均以字串處理 |

這些限制是有意為之，保持腳本引擎的可讀性，方便學習與理解實作原理。
