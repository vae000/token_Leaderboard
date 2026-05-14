# Token Leaderboard

`plan.md` 中的执行计划已经落成一个可运行的 monorepo MVP，包含：

- `apps/cli`: Rust CLI，支持 `login/sync/status/logout/doctor`
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

### 2. CLI

```bash
cargo run -p cli -- status
cargo run -p cli -- login --user-id u_demo --name Demo
cargo run -p cli -- sync --tool codex
```

CLI 默认将状态写入用户配置目录下的 `token-leaderboard/config.json`。

### 3. Web

```bash
cd apps/web
corepack pnpm install
corepack pnpm dev
```

默认访问 `http://127.0.0.1:3000`，服务端会优先读取 `NEXT_PUBLIC_API_BASE_URL` 指向的 API。

## 当前实现范围

- CLI 目前已接入 `Codex` adapter，并为其余工具保留扩展位。
- API 已覆盖计划中的主要读写接口，并在内存仓库上提供分钟级聚合逻辑示意。
- Web 已实现三大页面和筛选/表格/趋势展示。
- 数据库 migration 已覆盖核心表、聚合表、奖励表和基础 seed 数据。
