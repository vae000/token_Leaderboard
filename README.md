# Token Leaderboard

`plan.md` 中的执行计划已经落成一个可运行的 monorepo MVP，包含：

- `apps/cli`: Rust CLI 守护进程，支持 `start/stop/restart/status`
- `apps/api`: Rust + Axum API，提供认证骨架、事件 ingest、聚合查询、管理接口骨架
- `apps/web`: Next.js App Router 前端，包含 Dashboard、排行榜、个人页
- `crates/common`: CLI/API 共用模型
- `db/migrations`: PostgreSQL schema 与初始化数据
- `docs`: 架构与开发说明

## 本地启动

### 1. API

```bash
cargo run -p api
```

默认监听 `http://127.0.0.1:8080`。未配置 PostgreSQL 时会使用内存仓库运行；配置 `API__DATABASE_URL` 后会自动执行 `db/migrations`。

### 2. CLI 守护进程

CLI 以守护进程方式运行，启动后自动扫描所有支持的工具日志并持续监控。

```bash
# 启动守护进程（后台运行，自动扫描 Codex + DeepSeek-TUI）
cargo run -p cli -- start

# 查看状态
cargo run -p cli -- status

# 重启
cargo run -p cli -- restart

# 停止
cargo run -p cli -- stop
```

守护进程默认每 60 秒轮询一次，可通过 `--interval-seconds` 或 `CLI_SYNC_INTERVAL_SECONDS` 调整间隔。

支持自动扫描的工具：

- **Codex** — 扫描本地 `.jsonl` 日志（默认 `sample-data/codex`）
- **DeepSeek-TUI** — 扫描 `~/.deepseek/sessions/*.json`
- Cursor、Claude Code、OpenCode 接口已预留，适配器待实现

PID 文件与状态配置文件均写入用户配置目录下的 `token-leaderboard/leaderboard/`。

### 3. Web

```bash
cd apps/web
corepack pnpm install
corepack pnpm dev
```

默认访问 `http://127.0.0.1:3000`，服务端会优先读取 `NEXT_PUBLIC_API_BASE_URL` 指向的 API。

## 构建

所有组件均在项目根目录下用 Cargo 或 pnpm 构建。

### CLI

```bash
# Debug 构建
cargo build -p cli

# Release 构建（优化，推荐分发）
cargo build -p cli --release
```

产物：`target/debug/cli` 或 `target/release/cli`，单二进制文件，无外部运行时依赖。

### API

```bash
cargo build -p api --release
```

产物：`target/release/api`。

### Web

```bash
cd apps/web
corepack pnpm install
corepack pnpm build
```

产物在 `apps/web/.next` 目录，通过 `corepack pnpm start` 启动生产服务。

## Web 登录配置

当前 Web 侧使用本地账号密码登录，启动前建议补齐以下环境变量：

```env
API__PUBLIC_BASE_URL=https://你的-api-公网域名
API__WEB_BASE_URL=https://你的-web-公网域名
API__AUTO_PASSWORD_SALT=替换为私有随机盐值
API__WEB_SESSION_TTL_HOURS=720
```

说明：

- 默认账号名使用 `user_id`。
- 默认密码由服务端按 `API__AUTO_PASSWORD_SALT + user_id` 自动派生。
- 登录接口为 `/v1/auth/web/login`，成功后 Web 会写入本地会话 cookie。
- 管理员可以通过 `/v1/admin/users/{user_id}/credentials` 查询某个用户当前可用的自动生成账号密码。

## 当前实现范围

- CLI 已接入 `Codex` 和 `DeepSeek-TUI` adapter，以守护进程方式运行，支持 `start/stop/restart/status`；其余工具（Cursor、Claude Code、OpenCode）接口已预留。
- API 已覆盖计划中的主要读写接口，并在内存仓库上提供分钟级聚合逻辑示意。
- Web 已实现仪表盘、排行榜、个人页三大页面和筛选/表格/趋势展示。
- 数据库 migration 已覆盖核心表、聚合表、奖励表和基础 seed 数据。
