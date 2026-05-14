# Token Leaderboard

一个用于汇总 AI 工具 Token 使用量的 monorepo：

- `apps/api`: Rust + Axum API
- `apps/cli`: Rust 本地采集守护进程
- `apps/web`: Next.js Web 面板
- `crates/common`: 共用模型
- `db/migrations`: PostgreSQL 迁移

当前以**真实数据采集**为主，不使用种子数据。

## 快速启动

推荐直接用 Docker：

```bash
docker compose up -d --build
```

启动后访问：

- Web: `http://localhost:3000`
- API: `http://localhost:8080`

说明：

- API 镜像编译时会内嵌 `db/migrations`，构建上下文必须是仓库根目录。
- Web 容器内部通过 `API_INTERNAL_BASE_URL=http://api:8080` 访问 API。
- 浏览器访问 API 使用 `NEXT_PUBLIC_API_BASE_URL=http://localhost:8080`。

## 本地开发

### API

```bash
cargo run -p api
```

行为：

- 读取 `API__DATABASE_URL`，或用 `API__DB_*` 自动拼接连接串
- 若目标库不存在，会先连接 `postgres` 库执行 `CREATE DATABASE`
- 然后自动建 schema 并执行 `db/migrations`

### CLI

首次登录并启动守护进程：

```bash
cargo run -p cli -- login
cargo run -p cli -- start
```

常用命令：

```bash
cargo run -p cli -- status
cargo run -p cli -- restart
cargo run -p cli -- stop
```

如果是本机直接运行二进制，API 地址写 `localhost` 没问题：

```bash
./target/release/leaderboard --api-base-url http://localhost:8080 restart
```

已接入的真实日志源：

- `Codex`: `~/.codex/sessions/`
- `Claude Code`: `~/.claude/projects/**/*.jsonl`
- `DeepSeek-TUI`: `~/.deepseek/sessions/*.json`

`Cursor` 和 `OpenCode` 适配器仍在完善中。

### Web

```bash
cd apps/web
corepack pnpm dev
```

## 关键环境变量

只列常用项：

| 变量 | 说明 |
|------|------|
| `API__DB_HOST` / `API__DB_PORT` / `API__DB_USER` / `API__DB_PASSWORD` / `API__DB_NAME` | PostgreSQL 连接参数 |
| `API__DATABASE_URL` | 完整数据库连接串，优先级高于拆分变量 |
| `API__DB_SCHEMA` | schema，默认使用 `token` |
| `API__AUTO_PASSWORD_SALT` | Web 登录密码盐值 |
| `API__WEB_BASE_URL` | Web 外部地址 |
| `API_INTERNAL_BASE_URL` | Web 容器内部访问 API 的地址，Compose 下用 `http://api:8080` |
| `NEXT_PUBLIC_API_BASE_URL` | 浏览器访问 API 的地址，Compose 下用 `http://localhost:8080` |
| `CLI_SYNC_INTERVAL_SECONDS` | CLI 轮询间隔 |
| `CLI_SYNC_BATCH_SIZE` | CLI 批量上传条数 |

## Web 登录

Web 使用本地账号密码登录：

- 用户名就是 `user_id`
- 密码由服务端按 `SHA256(API__AUTO_PASSWORD_SALT:user_id)` 自动生成

管理员可查询凭据：

```bash
curl http://localhost:8080/v1/admin/users/<user_id>/credentials
```

## 构建

```bash
cargo build -p api --release
cargo build -p cli --release
cd apps/web && corepack pnpm build
```

## 当前限制

- 当前 Web 能正常展示，但是否有数据取决于 CLI 是否已经上传真实事件
- `Cursor`、`OpenCode` 适配器仍未完全稳定
- 真实微信 OAuth、完整权限模型、更多生产化能力仍未完成
