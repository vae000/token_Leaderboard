# 开发说明

## Rust

```bash
cargo fmt
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Web

```bash
cd apps/web
corepack pnpm install
corepack pnpm lint
corepack pnpm build
```

## 环境变量

- API 读取 `API__*`
- CLI 读取 `CLI_*`
- Web 读取 `NEXT_PUBLIC_API_BASE_URL`

## 日志目录优先级

CLI 各 adapter 统一按以下优先级查找日志目录：

1. `--log-dir` 参数
2. 对应环境变量（`CLI_CODEX_LOG_DIR` / `CLI_DEEPSEEK_TUI_LOG_DIR`）
3. `~/.deepseek/sessions/`（DeepSeek-TUI 默认数据目录，仅 dev 模式回退）
4. `./sample-data/<tool-name>`（仅 dev 模式）

## Codex 日志格式

```json
{
  "timestamp": "2026-05-13T10:00:00Z",
  "model": "gpt-5.5",
  "input_tokens": 1200,
  "output_tokens": 430,
  "cached_tokens": 30,
  "session_id": "sess_123"
}
```

## DeepSeek-TUI 日志格式

DeepSeek TUI 将会话数据保存在 `~/.deepseek/sessions/` 目录下，每个会话一个 `.json` 文件。

```json
{
  "schema_version": 1,
  "metadata": {
    "id": "0951c6db-7da4-4b69-91b7-de55de22c957",
    "title": "New Session",
    "created_at": "2026-05-14T08:57:52.591430573Z",
    "updated_at": "2026-05-14T09:21:21.684888990Z",
    "message_count": 238,
    "total_tokens": 6591962,
    "model": "deepseek-v4-pro",
    "workspace": "/home/vip/gp/workspace",
    "mode": "yolo",
    "cost": {
      "session_cost_usd": 0.0,
      "session_cost_cny": 0.0
    }
  },
  "messages": [...],
  "system_prompt": "..."
}
```

CLI 读取 `metadata` 中的 `total_tokens`、`model`、`created_at`、`id` 生成使用事件。
