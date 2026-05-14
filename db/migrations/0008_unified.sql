-- ── 全量迁移合集（统一版）────────────────────────────────
-- 合并了 0001～0007 的全部逻辑。移除演示种子数据，
-- 所有操作均为幂等（IF NOT EXISTS / ON CONFLICT / IF EXISTS）。

-- ============================================================
-- 0001: 核心表
-- ============================================================

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    email TEXT,
    wechat_openid TEXT UNIQUE,
    wechat_unionid TEXT UNIQUE,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS teams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_team_id TEXT REFERENCES teams (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_team_memberships (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    team_id TEXT NOT NULL REFERENCES teams (id),
    effective_from TIMESTAMPTZ NOT NULL,
    effective_to TIMESTAMPTZ,
    UNIQUE (user_id, team_id, effective_from)
);

CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users (id) ON DELETE SET NULL,
    refresh_token TEXT,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tool_types (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS model_catalog (
    id BIGSERIAL PRIMARY KEY,
    model TEXT NOT NULL,
    vendor TEXT NOT NULL,
    input_price_per_1k_usd NUMERIC(12, 6) NOT NULL,
    output_price_per_1k_usd NUMERIC(12, 6) NOT NULL,
    cache_price_per_1k_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    max_input_tokens BIGINT NOT NULL DEFAULT 0,
    effective_from TIMESTAMPTZ NOT NULL,
    UNIQUE (model, effective_from, max_input_tokens)
);

CREATE TABLE IF NOT EXISTS usage_events (
    id UUID PRIMARY KEY,
    event_id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users (id),
    tool TEXT NOT NULL,
    model TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens BIGINT NOT NULL DEFAULT 0,
    estimated_cost_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    session_id TEXT,
    source_file TEXT NOT NULL,
    source_offset BIGINT NOT NULL DEFAULT 0,
    raw_hash TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_usage_events_occurred_at ON usage_events (occurred_at);
CREATE INDEX IF NOT EXISTS idx_usage_events_user_id ON usage_events (user_id);
CREATE INDEX IF NOT EXISTS idx_usage_events_tool ON usage_events (tool);
CREATE INDEX IF NOT EXISTS idx_usage_events_model ON usage_events (model);
CREATE INDEX IF NOT EXISTS idx_usage_events_raw_hash ON usage_events (raw_hash);

-- ============================================================
-- 0002: 聚合表
-- ============================================================

CREATE TABLE IF NOT EXISTS usage_agg_user_day (
    day DATE NOT NULL,
    user_id TEXT NOT NULL REFERENCES users (id),
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens BIGINT NOT NULL DEFAULT 0,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    estimated_cost_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (day, user_id)
);

CREATE TABLE IF NOT EXISTS usage_agg_team_day (
    day DATE NOT NULL,
    team_id TEXT NOT NULL REFERENCES teams (id),
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens BIGINT NOT NULL DEFAULT 0,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    estimated_cost_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (day, team_id)
);

CREATE TABLE IF NOT EXISTS usage_agg_tool_day (
    day DATE NOT NULL,
    tool TEXT NOT NULL,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens BIGINT NOT NULL DEFAULT 0,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    estimated_cost_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (day, tool)
);

CREATE TABLE IF NOT EXISTS usage_agg_model_day (
    day DATE NOT NULL,
    model TEXT NOT NULL,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens BIGINT NOT NULL DEFAULT 0,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    estimated_cost_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (day, model)
);

CREATE INDEX IF NOT EXISTS idx_usage_agg_user_day_user ON usage_agg_user_day (user_id, day);
CREATE INDEX IF NOT EXISTS idx_usage_agg_team_day_team ON usage_agg_team_day (team_id, day);
CREATE INDEX IF NOT EXISTS idx_usage_agg_tool_day_tool ON usage_agg_tool_day (tool, day);
CREATE INDEX IF NOT EXISTS idx_usage_agg_model_day_model ON usage_agg_model_day (model, day);

-- ============================================================
-- 0003: 奖励表与基础数据
-- ============================================================

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

INSERT INTO reward_rules (id, name, description)
VALUES
    ('monthly-top', '月度 Token TOP', '按月总 token 排名展示前列用户'),
    ('coverage', '工具覆盖度奖励', '使用工具种类最多的用户奖励'),
    ('growth', '增长最快奖励', '相较上周期增长最快的用户')
ON CONFLICT (id) DO NOTHING;

-- ============================================================
-- 0004: 模型价格表扩展
-- ============================================================

ALTER TABLE model_catalog
ADD COLUMN IF NOT EXISTS source_url TEXT NOT NULL DEFAULT '';
ALTER TABLE model_catalog
ADD COLUMN IF NOT EXISTS source_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

INSERT INTO model_catalog (
    model, vendor, input_price_per_1k_usd, output_price_per_1k_usd,
    cache_price_per_1k_usd, effective_from, source_url, source_checked_at
)
VALUES
    ('gpt-5.5', 'OpenAI', 0.005000, 0.030000, 0.000500, '2026-05-14T00:00:00Z',
     'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('openai/gpt-5.5', 'OpenAI', 0.005000, 0.030000, 0.000500, '2026-05-14T00:00:00Z',
     'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('gpt-5.4', 'OpenAI', 0.002500, 0.015000, 0.000250, '2026-05-14T00:00:00Z',
     'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('openai/gpt-5.4', 'OpenAI', 0.002500, 0.015000, 0.000250, '2026-05-14T00:00:00Z',
     'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('gpt-5.3-codex', 'OpenAI', 0.001750, 0.014000, 0.000175, '2026-05-14T00:00:00Z',
     'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('openai/gpt-5.3-codex', 'OpenAI', 0.001750, 0.014000, 0.000175, '2026-05-14T00:00:00Z',
     'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('claude-opus-4.1', 'Anthropic', 0.015000, 0.075000, 0.001500, '2026-05-14T00:00:00Z',
     'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('anthropic/claude-opus-4.1', 'Anthropic', 0.015000, 0.075000, 0.001500, '2026-05-14T00:00:00Z',
     'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('claude-opus-4.6', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z',
     'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('anthropic/claude-opus-4.6', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z',
     'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('claude-opus-4.7', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z',
     'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('anthropic/claude-opus-4.7', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z',
     'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('deepseek-reasoner', 'DeepSeek', 0.000140, 0.000280, 0.000003, '2026-05-14T00:00:00Z',
     'https://api-docs.deepseek.com/quick_start/pricing/', '2026-05-14T00:00:00Z'),
    ('deepseek-v4-flash', 'DeepSeek', 0.000140, 0.000280, 0.000003, '2026-05-14T00:00:00Z',
     'https://api-docs.deepseek.com/quick_start/pricing/', '2026-05-14T00:00:00Z')
ON CONFLICT (model, effective_from, max_input_tokens) DO UPDATE
SET vendor = EXCLUDED.vendor,
    input_price_per_1k_usd = EXCLUDED.input_price_per_1k_usd,
    output_price_per_1k_usd = EXCLUDED.output_price_per_1k_usd,
    cache_price_per_1k_usd = EXCLUDED.cache_price_per_1k_usd,
    source_url = EXCLUDED.source_url,
    source_checked_at = EXCLUDED.source_checked_at;

-- ============================================================
-- 0005: 扩展模型 tier
-- ============================================================

INSERT INTO model_catalog (
    model, vendor, input_price_per_1k_usd, output_price_per_1k_usd,
    cache_price_per_1k_usd, max_input_tokens, effective_from,
    source_url, source_checked_at
)
VALUES
    ('kimi-k2.6', 'Kimi', 0.000950, 0.004000, 0.000160, 0,
     '2026-05-14T00:00:00Z', 'https://platform.kimi.ai/docs/pricing/chat-k26', '2026-05-14T00:00:00Z'),
    ('kimi-2.6', 'Kimi', 0.000950, 0.004000, 0.000160, 0,
     '2026-05-14T00:00:00Z', 'https://platform.kimi.ai/docs/pricing/chat-k26', '2026-05-14T00:00:00Z'),
    ('MiniMax-M2.7', 'MiniMax', 0.000300, 0.001200, 0.000060, 0,
     '2026-05-14T00:00:00Z', 'https://platform.minimax.io/docs/guides/pricing-paygo', '2026-05-14T00:00:00Z'),
    ('minimax-2.7', 'MiniMax', 0.000300, 0.001200, 0.000060, 0,
     '2026-05-14T00:00:00Z', 'https://platform.minimax.io/docs/guides/pricing-paygo', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus', 'Qwen', 0.000548, 0.003287, 0.000000, 256000,
     '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus', 'Qwen', 0.002191, 0.006574, 0.000000, 1000000,
     '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus-2026-04-02', 'Qwen', 0.000548, 0.003287, 0.000000, 256000,
     '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus-2026-04-02', 'Qwen', 0.002191, 0.006574, 0.000000, 1000000,
     '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z')
ON CONFLICT (model, effective_from, max_input_tokens) DO UPDATE
SET vendor = EXCLUDED.vendor,
    input_price_per_1k_usd = EXCLUDED.input_price_per_1k_usd,
    output_price_per_1k_usd = EXCLUDED.output_price_per_1k_usd,
    cache_price_per_1k_usd = EXCLUDED.cache_price_per_1k_usd,
    source_url = EXCLUDED.source_url,
    source_checked_at = EXCLUDED.source_checked_at;

-- ============================================================
-- 0006: Web 会话表
-- ============================================================

CREATE TABLE IF NOT EXISTS web_login_states (
    state TEXT PRIMARY KEY,
    return_to TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS web_sessions (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_web_login_states_expires_at
ON web_login_states (expires_at);

CREATE INDEX IF NOT EXISTS idx_web_sessions_user_id
ON web_sessions (user_id);

CREATE INDEX IF NOT EXISTS idx_web_sessions_expires_at
ON web_sessions (expires_at);

-- ============================================================
-- 0007: 外键级联删除
-- ============================================================

-- 已在 devices 表定义中直接使用 ON DELETE SET NULL（见上方），
-- 无需重复修改。保留此注释以确保版本连续性。
