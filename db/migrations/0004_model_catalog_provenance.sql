ALTER TABLE model_catalog
ADD COLUMN IF NOT EXISTS source_url TEXT NOT NULL DEFAULT '',
ADD COLUMN IF NOT EXISTS source_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

INSERT INTO model_catalog (
    model,
    vendor,
    input_price_per_1k_usd,
    output_price_per_1k_usd,
    cache_price_per_1k_usd,
    effective_from,
    source_url,
    source_checked_at
)
VALUES
    ('gpt-5.5', 'OpenAI', 0.005000, 0.030000, 0.000500, '2026-05-14T00:00:00Z', 'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('openai/gpt-5.5', 'OpenAI', 0.005000, 0.030000, 0.000500, '2026-05-14T00:00:00Z', 'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('gpt-5.4', 'OpenAI', 0.002500, 0.015000, 0.000250, '2026-05-14T00:00:00Z', 'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('openai/gpt-5.4', 'OpenAI', 0.002500, 0.015000, 0.000250, '2026-05-14T00:00:00Z', 'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('gpt-5.3-codex', 'OpenAI', 0.001750, 0.014000, 0.000175, '2026-05-14T00:00:00Z', 'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('openai/gpt-5.3-codex', 'OpenAI', 0.001750, 0.014000, 0.000175, '2026-05-14T00:00:00Z', 'https://developers.openai.com/api/docs/pricing', '2026-05-14T00:00:00Z'),
    ('claude-opus-4.1', 'Anthropic', 0.015000, 0.075000, 0.001500, '2026-05-14T00:00:00Z', 'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('anthropic/claude-opus-4.1', 'Anthropic', 0.015000, 0.075000, 0.001500, '2026-05-14T00:00:00Z', 'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('claude-opus-4.6', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z', 'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('anthropic/claude-opus-4.6', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z', 'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('claude-opus-4.7', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z', 'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('anthropic/claude-opus-4.7', 'Anthropic', 0.005000, 0.025000, 0.000500, '2026-05-14T00:00:00Z', 'https://platform.claude.com/docs/en/about-claude/pricing', '2026-05-14T00:00:00Z'),
    ('deepseek-reasoner', 'DeepSeek', 0.000140, 0.000280, 0.000003, '2026-05-14T00:00:00Z', 'https://api-docs.deepseek.com/quick_start/pricing/', '2026-05-14T00:00:00Z'),
    ('deepseek-v4-flash', 'DeepSeek', 0.000140, 0.000280, 0.000003, '2026-05-14T00:00:00Z', 'https://api-docs.deepseek.com/quick_start/pricing/', '2026-05-14T00:00:00Z')
ON CONFLICT (model, effective_from) DO UPDATE
SET vendor = EXCLUDED.vendor,
    input_price_per_1k_usd = EXCLUDED.input_price_per_1k_usd,
    output_price_per_1k_usd = EXCLUDED.output_price_per_1k_usd,
    cache_price_per_1k_usd = EXCLUDED.cache_price_per_1k_usd,
    source_url = EXCLUDED.source_url,
    source_checked_at = EXCLUDED.source_checked_at;
