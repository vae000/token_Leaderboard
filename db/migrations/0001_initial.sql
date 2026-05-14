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
    user_id TEXT REFERENCES users (id),
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
    effective_from TIMESTAMPTZ NOT NULL,
    UNIQUE (model, effective_from)
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
