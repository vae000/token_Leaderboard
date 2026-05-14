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
