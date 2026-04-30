# 周报工具 (Daily Paper Tool)

一个基于 Tauri + Vue 3 的桌面应用，用于从**本地 Git 仓库**提取工作记录，结合 **OpenAI 兼容接口**进行润色，并导出固定格式的 **Word 周报**。

当前版本已经收缩成两个核心能力：

- 读取本地 Git 提交，整理为当天工作内容
- 生成并导出固定格式的 Word 周报

## 功能特性

- **本地 Git 获取**：按天读取本地仓库当前分支的提交记录，按作者名或邮箱过滤
- **AI 润色**：支持自定义 OpenAI 兼容 API，将原始提交信息整理成更适合周报的中文工作项
- **每日最多 4 条**：AI 润色和手动编辑都控制在每天最多 4 条内容
- **Word 周报导出**：导出接近固定模板样式的 `.docx` 周报
- **本地数据存储**：基于 SQLite 保存工作记录和周总结
- **Windows 在线打包**：已配置 GitHub Actions，只打 Windows NSIS 安装包

## 技术栈

- **前端**：Vue 3 + TypeScript + Ant Design Vue + Vite
- **后端**：Tauri v2 + Rust
- **数据库**：SQLite (`tauri-plugin-sql`)
- **文档生成**：`docx-rs`

## 当前使用方式

1. 在「配置」页填写：
   - 本地 Git 仓库路径
   - Git 作者名或作者邮箱
   - AI 接口 `base_url / api_key / model`
   - 员工姓名、默认日工时、默认完成度、总结备注
2. 在首页点击「读取本周 Git」
3. 按天点击「AI 润色」
4. 需要时点击「AI 总结」
5. 点击「导出周报」，生成 `.docx`

## 配置说明

首次使用需要在「配置」页面完成以下设置。

### 本地 Git 配置

| 字段 | 说明 |
|------|------|
| 仓库路径 | 每行一个本地 Git 仓库路径 |
| Git 作者名 | 用于过滤提交作者 |
| Git 作者邮箱 | 用于过滤提交作者 |

说明：

- 作者名和作者邮箱至少填写一个
- 当前默认只读取每个仓库**当前分支**的提交

### 模型配置

支持任意 OpenAI 兼容接口。

| 字段 | 说明 | 示例 |
|------|------|------|
| Base URL | API 基础地址 | `https://api.openai.com` |
| API Key | API 密钥 | `sk-...` |
| Model | 模型名称 | `gpt-4o-mini` |

### 周报导出配置

| 字段 | 说明 |
|------|------|
| 员工姓名 | 导出 Word 时写入员工栏 |
| 默认日工时 | 用于自动分摊当天时长 |
| 默认完成度 | 默认填入“任务完成度和困难”列 |
| 总结备注 | 导出周报最后一列备注内容 |

### 提示词配置

可选自定义：

- AI 润色 System Prompt
- AI 润色 Few-shot 示例
- 周总结 System Prompt

## 页面说明

### 本周工作

- 按周展示每天的工作内容卡片
- 支持读取本周 Git、按天读取 Git、AI 润色、手动编辑、AI 总结、导出周报

### 工作记录

- 按天聚合查看历史工作内容
- 支持关键字搜索和日期范围筛选

### 配置

- 管理本地 Git、模型、周报导出参数和提示词

## 导出格式

当前导出为 `.docx`，整体结构贴近固定周报模板：

- 标题：`工作周报（YYYY.MM.DD-MM.DD）`
- 员工：`员工：姓名`
- 表格列：
  - 日期
  - 工作内容
  - 花费时长
  - 任务完成度和困难
- 每天固定保留 4 行
- 总结行包含：
  - 总结
  - 周总结内容
  - 总时长
  - 备注

## 从源码运行

### 环境要求

- Node.js 18+
- pnpm 9+
- Rust 1.70+
- Windows 打包时需要 Visual Studio C++ Build Tools

### 开发命令

```bash
pnpm install
pnpm tauri dev
```

### 前端静态检查

```bash
pnpm exec vue-tsc --noEmit
pnpm build
```

## 在线打包

仓库已配置 GitHub Actions 在线打包，只生成 **Windows NSIS 安装包**。

### 触发方式 1：手动运行

1. 打开 GitHub 仓库的 `Actions`
2. 选择 `Release`
3. 点击 `Run workflow`
4. 输入一个新的 `release_tag`，例如 `v1.0.3`
5. 等待构建完成
6. 到 `Releases` 页面下载 Windows 安装包

### 触发方式 2：推送 tag

```bash
git tag v1.0.3
git push origin v1.0.3
```

## 项目结构

```text
daily-paper-tool/
├── src/
│   ├── db/                  # SQLite 访问
│   ├── pages/
│   │   ├── Home.vue         # 本周工作
│   │   ├── Records.vue      # 工作记录
│   │   └── Settings.vue     # 配置页面
│   ├── router/              # 路由
│   └── utils/               # 前端工具
├── src-tauri/
│   └── src/
│       ├── config.rs        # 配置模型
│       ├── local_git.rs     # 本地 Git 读取
│       ├── fetch.rs         # 数据获取与 AI 润色入口
│       ├── llm.rs           # OpenAI 兼容接口调用
│       ├── report.rs        # Word 周报导出
│       ├── lib.rs           # Tauri commands
│       └── utils.rs         # 通用文本处理
└── .github/workflows/
    └── release.yml          # Windows 在线打包
```

## 数据库

当前主要使用两张表：

```sql
CREATE TABLE work_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    work_date TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT DEFAULT 'manual',
    created_at TEXT NOT NULL
);

CREATE TABLE week_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_start TEXT NOT NULL UNIQUE,
    summary TEXT NOT NULL,
    key_tasks TEXT DEFAULT '',
    completion_status TEXT DEFAULT '',
    updated_at TEXT NOT NULL
);
```

说明：

- 目前 `week_summaries` 里保留了历史字段 `key_tasks` / `completion_status`
- 当前页面主流程已经不再依赖它们

## 仓库地址

- GitHub 仓库：<https://github.com/EvansLeeSin/daily-paper-tool>

## 许可证

MIT License
