# AI Change Guidance

**使用台灣正體中文**

- 使用 Rust 1.94 實作的教育用作業系統模擬器，透過 TUI 圖形化展示 OS 核心運作原理。

## Rust

- Version: v1.94+

## Docs

- README: README.md
- 文件位置：`docs/`
- 新增、修改或更新功能時，請同步更新文件
- 語言使用台灣正體中文，不要使用中國用語

## Commit Convention

請遵循 Conventional Commits 規範，git-cliff 會據此自動生成 changelog 與版本號。

```
<type>(<scope>): <簡短描述>

<選擇性詳細說明>
```

**types**（依 git-cliff 分組）：
- `feat` — 新功能 → Features
- `fix` — 錯誤修正 → Bug Fixes
- `docs` — 文件異動 → Documentation
- `perf` — 效能改善 → Performance
- `refactor` — 重構 → Refactor
- `style` — 程式碼格式 → Styling
- `test` — 測試相關 → Testing
- `chore` — 雜項（建置、依賴等）→ Miscellaneous
- `ci` — CI/CD 異動 → CI/CD

**scope**（選擇性）：例如 `kernel`、`ui`、`scheduler`、`memory` 等模組名稱。

範例：
```
feat(scheduler): 加入多層回饋佇列排程演算法
fix(memory): 修正分頁錯誤處理中的邊界條件
docs: 更新記憶體管理文件中的圖表
ci: 加入 macOS Intel 建置矩陣
```
