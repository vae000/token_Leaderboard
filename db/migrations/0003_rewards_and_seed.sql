CREATE TABLE IF NOT EXISTS reward_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS reward_results (
    id BIGSERIAL PRIMARY KEY,
    rule_id TEXT NOT NULL REFERENCES reward_rules (id),
    user_id TEXT NOT NULL REFERENCES users (id),
    snapshot_date DATE NOT NULL,
    label TEXT NOT NULL,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (rule_id, user_id, snapshot_date)
);

INSERT INTO tool_types (id, display_name)
VALUES
    ('codex', 'Codex'),
    ('cursor', 'Cursor'),
    ('claude_code', 'Claude Code'),
    ('opencode', 'OpenCode'),
    ('deepseek_tui', 'DeepSeek-TUI')
ON CONFLICT (id) DO NOTHING;

INSERT INTO model_catalog (model, vendor, input_price_per_1k_usd, output_price_per_1k_usd, cache_price_per_1k_usd, effective_from)
VALUES
    ('gpt-5.5', 'OpenAI', 0.010000, 0.030000, 0.002000, '2026-01-01T00:00:00Z'),
    ('claude-opus-4.1', 'Anthropic', 0.015000, 0.075000, 0.000000, '2026-01-01T00:00:00Z'),
    ('deepseek-reasoner', 'DeepSeek', 0.002000, 0.008000, 0.000000, '2026-01-01T00:00:00Z')
ON CONFLICT (model, effective_from) DO NOTHING;

INSERT INTO reward_rules (id, name, description)
VALUES
    ('monthly-top', '月度 Token TOP', '按月总 token 排名展示前列用户'),
    ('coverage', '工具覆盖度奖励', '使用工具种类最多的用户奖励'),
    ('growth', '增长最快奖励', '相较上周期增长最快的用户')
ON CONFLICT (id) DO NOTHING;
