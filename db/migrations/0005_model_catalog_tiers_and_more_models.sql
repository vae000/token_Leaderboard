INSERT INTO model_catalog (
    model,
    vendor,
    input_price_per_1k_usd,
    output_price_per_1k_usd,
    cache_price_per_1k_usd,
    max_input_tokens,
    effective_from,
    source_url,
    source_checked_at
)
VALUES
    ('kimi-k2.6', 'Kimi', 0.000950, 0.004000, 0.000160, 0, '2026-05-14T00:00:00Z', 'https://platform.kimi.ai/docs/pricing/chat-k26', '2026-05-14T00:00:00Z'),
    ('kimi-2.6', 'Kimi', 0.000950, 0.004000, 0.000160, 0, '2026-05-14T00:00:00Z', 'https://platform.kimi.ai/docs/pricing/chat-k26', '2026-05-14T00:00:00Z'),
    ('MiniMax-M2.7', 'MiniMax', 0.000300, 0.001200, 0.000060, 0, '2026-05-14T00:00:00Z', 'https://platform.minimax.io/docs/guides/pricing-paygo', '2026-05-14T00:00:00Z'),
    ('minimax-2.7', 'MiniMax', 0.000300, 0.001200, 0.000060, 0, '2026-05-14T00:00:00Z', 'https://platform.minimax.io/docs/guides/pricing-paygo', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus', 'Qwen', 0.000548, 0.003287, 0.000000, 256000, '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus', 'Qwen', 0.002191, 0.006574, 0.000000, 1000000, '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus-2026-04-02', 'Qwen', 0.000548, 0.003287, 0.000000, 256000, '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z'),
    ('qwen3.6-plus-2026-04-02', 'Qwen', 0.002191, 0.006574, 0.000000, 1000000, '2026-05-14T00:00:00Z', 'https://help.aliyun.com/zh/model-studio/model-pricing', '2026-05-14T00:00:00Z')
ON CONFLICT (model, effective_from, max_input_tokens) DO UPDATE
SET vendor = EXCLUDED.vendor,
    input_price_per_1k_usd = EXCLUDED.input_price_per_1k_usd,
    output_price_per_1k_usd = EXCLUDED.output_price_per_1k_usd,
    cache_price_per_1k_usd = EXCLUDED.cache_price_per_1k_usd,
    source_url = EXCLUDED.source_url,
    source_checked_at = EXCLUDED.source_checked_at;
