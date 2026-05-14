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
