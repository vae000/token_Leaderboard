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

## Codex 日志样例

CLI 的 `codex` adapter 读取 JSONL，支持字段：

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

默认日志目录优先级：

1. `--log-dir`
2. `CLI_CODEX_LOG_DIR`
3. `./sample-data/codex`
