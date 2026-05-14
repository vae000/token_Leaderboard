use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::Client;
use scraper::Html;
use sqlx::PgPool;

const OPENAI_PRICING_URL: &str = "https://developers.openai.com/api/docs/pricing";
const ANTHROPIC_PRICING_URL: &str = "https://platform.claude.com/docs/en/about-claude/pricing";
const DEEPSEEK_PRICING_URL: &str = "https://api-docs.deepseek.com/quick_start/pricing/";
const KIMI_PRICING_URL: &str = "https://platform.kimi.ai/";
const MINIMAX_PRICING_URL: &str =
    "https://platform.minimax.io/docs/api-reference/anthropic-api-compatible-cache";
const QWEN_PRICING_URL: &str = "https://help.aliyun.com/zh/model-studio/model-pricing";
const SAFE_FX_URL: &str = "https://www.safe.gov.cn/AppStructured/hlw/RMBQuery.do";

#[derive(Debug, Clone)]
pub struct CatalogPrice {
    pub model: &'static str,
    pub vendor: &'static str,
    pub input_price_per_1k_usd: f64,
    pub output_price_per_1k_usd: f64,
    pub cache_price_per_1k_usd: f64,
    pub max_input_tokens: i64,
    pub source_url: &'static str,
}

impl CatalogPrice {
    fn from_per_million(
        model: &'static str,
        vendor: &'static str,
        input_price_per_million_usd: f64,
        output_price_per_million_usd: f64,
        cache_price_per_million_usd: f64,
        max_input_tokens: i64,
        source_url: &'static str,
    ) -> Self {
        Self {
            model,
            vendor,
            input_price_per_1k_usd: input_price_per_million_usd / 1000.0,
            output_price_per_1k_usd: output_price_per_million_usd / 1000.0,
            cache_price_per_1k_usd: cache_price_per_million_usd / 1000.0,
            max_input_tokens,
            source_url,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TokenPricePerMillion {
    input: f64,
    output: f64,
    cache: f64,
}

pub async fn refresh_model_catalog(pool: &PgPool) -> anyhow::Result<usize> {
    let client = Client::builder()
        .user_agent("token-leaderboard/0.1")
        .build()
        .context("failed to build pricing http client")?;

    let checked_at = Utc::now();
    let effective_from = checked_at
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight should be valid")
        .and_utc();
    let prices = fetch_official_prices(&client).await?;

    let mut tx = pool.begin().await?;
    for price in &prices {
        sqlx::query(
            r#"
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (model, effective_from, max_input_tokens) DO UPDATE
            SET vendor = EXCLUDED.vendor,
                input_price_per_1k_usd = EXCLUDED.input_price_per_1k_usd,
                output_price_per_1k_usd = EXCLUDED.output_price_per_1k_usd,
                cache_price_per_1k_usd = EXCLUDED.cache_price_per_1k_usd,
                source_url = EXCLUDED.source_url,
                source_checked_at = EXCLUDED.source_checked_at
            "#,
        )
        .bind(price.model)
        .bind(price.vendor)
        .bind(price.input_price_per_1k_usd)
        .bind(price.output_price_per_1k_usd)
        .bind(price.cache_price_per_1k_usd)
        .bind(price.max_input_tokens)
        .bind(effective_from)
        .bind(price.source_url)
        .bind(checked_at)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(prices.len())
}

async fn fetch_official_prices(client: &Client) -> anyhow::Result<Vec<CatalogPrice>> {
    let openai = fetch_text(client, OPENAI_PRICING_URL).await?;
    let anthropic = fetch_text(client, ANTHROPIC_PRICING_URL).await?;
    let deepseek = fetch_text(client, DEEPSEEK_PRICING_URL).await?;
    let kimi = fetch_text(client, KIMI_PRICING_URL).await?;
    let minimax = fetch_text(client, MINIMAX_PRICING_URL).await?;
    let qwen = fetch_text(client, QWEN_PRICING_URL).await?;
    let rmb_fx = fetch_text(client, SAFE_FX_URL).await?;
    let usd_cny_rate = parse_safe_usd_cny_rate(&rmb_fx)?;

    let mut prices = Vec::new();
    prices.extend(parse_openai_prices(&openai)?);
    prices.extend(parse_anthropic_prices(&anthropic)?);
    prices.extend(parse_deepseek_prices(&deepseek)?);
    prices.extend(parse_kimi_prices(&kimi)?);
    prices.extend(parse_minimax_prices(&minimax)?);
    prices.extend(parse_qwen_prices(&qwen, usd_cny_rate)?);
    Ok(prices)
}

async fn fetch_text(client: &Client, url: &str) -> anyhow::Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to fetch pricing page: {url}"))?
        .error_for_status()
        .with_context(|| format!("pricing page returned non-success status: {url}"))?;
    let body = response.text().await?;
    Ok(normalize_text(&body))
}

fn normalize_text(html: &str) -> String {
    let document = Html::parse_document(html);
    document
        .root_element()
        .text()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_openai_prices(text: &str) -> anyhow::Result<Vec<CatalogPrice>> {
    let flagship = extract_section(text, "Flagship models", "Multimodal models")?;
    let specialized = extract_section(text, "Specialized models", "Finetuning")?;

    let gpt_55 = capture_openai_row(&flagship, "gpt-5.5")?;
    let gpt_54 = capture_openai_row(&flagship, "gpt-5.4")?;
    let gpt_53_codex = capture_openai_row(&specialized, "gpt-5.3-codex")?;

    Ok(vec![
        openai_entry("gpt-5.5", gpt_55),
        openai_entry("openai/gpt-5.5", gpt_55),
        openai_entry("gpt-5.4", gpt_54),
        openai_entry("openai/gpt-5.4", gpt_54),
        openai_entry("gpt-5.3-codex", gpt_53_codex),
        openai_entry("openai/gpt-5.3-codex", gpt_53_codex),
    ])
}

fn parse_anthropic_prices(text: &str) -> anyhow::Result<Vec<CatalogPrice>> {
    let section = extract_section(text, "Model pricing", "Cloud platform pricing")?;
    let opus_47 = capture_anthropic_row(&section, "Claude Opus 4.7")?;
    let opus_46 = capture_anthropic_row(&section, "Claude Opus 4.6")?;
    let opus_41 = capture_anthropic_row(&section, "Claude Opus 4.1")?;

    Ok(vec![
        anthropic_entry("claude-opus-4.7", opus_47),
        anthropic_entry("anthropic/claude-opus-4.7", opus_47),
        anthropic_entry("claude-opus-4.6", opus_46),
        anthropic_entry("anthropic/claude-opus-4.6", opus_46),
        anthropic_entry("claude-opus-4.1", opus_41),
        anthropic_entry("anthropic/claude-opus-4.1", opus_41),
    ])
}

fn parse_deepseek_prices(text: &str) -> anyhow::Result<Vec<CatalogPrice>> {
    let section = extract_section(text, "Model Details", "Deduction Rules")?;
    let flash = capture_deepseek_flash_row(&section)?;
    Ok(vec![
        deepseek_entry("deepseek-v4-flash", flash),
        deepseek_entry("deepseek-reasoner", flash),
    ])
}

fn parse_kimi_prices(text: &str) -> anyhow::Result<Vec<CatalogPrice>> {
    let k2_6 = capture_kimi_row(text, "kimi-k2.6")?;
    Ok(vec![
        kimi_entry("kimi-k2.6", k2_6),
        kimi_entry("kimi-2.6", k2_6),
    ])
}

fn parse_minimax_prices(text: &str) -> anyhow::Result<Vec<CatalogPrice>> {
    let m2_7 = capture_minimax_row(text, "MiniMax-M2.7")?;
    Ok(vec![
        minimax_entry("MiniMax-M2.7", m2_7),
        minimax_entry("minimax-2.7", m2_7),
    ])
}

fn parse_qwen_prices(text: &str, usd_cny_rate: f64) -> anyhow::Result<Vec<CatalogPrice>> {
    let section = extract_section(text, "qwen3.6-plus", "qwen3.5-plus")?;
    let tier_256k = capture_qwen_row(section, "0<Token≤256K")?;
    let tier_1m = capture_qwen_row(section, "256K<Token≤1M")?;

    Ok(vec![
        qwen_entry("qwen3.6-plus", tier_256k, usd_cny_rate, 256_000),
        qwen_entry("qwen3.6-plus", tier_1m, usd_cny_rate, 1_000_000),
        qwen_entry("qwen3.6-plus-2026-04-02", tier_256k, usd_cny_rate, 256_000),
        qwen_entry("qwen3.6-plus-2026-04-02", tier_1m, usd_cny_rate, 1_000_000),
    ])
}

fn openai_entry(model: &'static str, price: TokenPricePerMillion) -> CatalogPrice {
    CatalogPrice::from_per_million(
        model,
        "OpenAI",
        price.input,
        price.output,
        price.cache,
        0,
        OPENAI_PRICING_URL,
    )
}

fn anthropic_entry(model: &'static str, price: TokenPricePerMillion) -> CatalogPrice {
    CatalogPrice::from_per_million(
        model,
        "Anthropic",
        price.input,
        price.output,
        price.cache,
        0,
        ANTHROPIC_PRICING_URL,
    )
}

fn deepseek_entry(model: &'static str, price: TokenPricePerMillion) -> CatalogPrice {
    CatalogPrice::from_per_million(
        model,
        "DeepSeek",
        price.input,
        price.output,
        price.cache,
        0,
        DEEPSEEK_PRICING_URL,
    )
}

fn kimi_entry(model: &'static str, price: TokenPricePerMillion) -> CatalogPrice {
    CatalogPrice::from_per_million(
        model,
        "Kimi",
        price.input,
        price.output,
        price.cache,
        0,
        KIMI_PRICING_URL,
    )
}

fn minimax_entry(model: &'static str, price: TokenPricePerMillion) -> CatalogPrice {
    CatalogPrice::from_per_million(
        model,
        "MiniMax",
        price.input,
        price.output,
        price.cache,
        0,
        MINIMAX_PRICING_URL,
    )
}

fn qwen_entry(
    model: &'static str,
    price_cny: TokenPricePerMillion,
    usd_cny_rate: f64,
    max_input_tokens: i64,
) -> CatalogPrice {
    CatalogPrice::from_per_million(
        model,
        "Qwen",
        price_cny.input / usd_cny_rate,
        price_cny.output / usd_cny_rate,
        price_cny.cache / usd_cny_rate,
        max_input_tokens,
        QWEN_PRICING_URL,
    )
}

fn extract_section<'a>(text: &'a str, start: &str, end: &str) -> anyhow::Result<&'a str> {
    let start_index = text
        .find(start)
        .with_context(|| format!("failed to locate section start `{start}`"))?;
    let rest = &text[start_index..];
    let end_index = rest
        .find(end)
        .with_context(|| format!("failed to locate section end `{end}`"))?;
    Ok(&rest[..end_index])
}

fn capture_openai_row(text: &str, model: &str) -> anyhow::Result<TokenPricePerMillion> {
    let pattern = format!(
        r"{}\$(?P<input>[0-9.]+)\$(?P<cache>[0-9.]+)\$(?P<output>[0-9.]+)",
        regex::escape(model)
    );
    let regex = Regex::new(&pattern)?;
    let captures = regex
        .captures(text)
        .with_context(|| format!("failed to parse OpenAI pricing row for `{model}`"))?;
    Ok(TokenPricePerMillion {
        input: parse_price(&captures["input"])?,
        cache: parse_price(&captures["cache"])?,
        output: parse_price(&captures["output"])?,
    })
}

fn capture_anthropic_row(text: &str, label: &str) -> anyhow::Result<TokenPricePerMillion> {
    let pattern = format!(
        r"{}\$(?P<input>[0-9.]+) / MTok\$(?P<cache_write_5m>[0-9.]+) / MTok\$(?P<cache_write_1h>[0-9.]+) / MTok\$(?P<cache_hit>[0-9.]+) / MTok\$(?P<output>[0-9.]+) / MTok",
        regex::escape(label)
    );
    let regex = Regex::new(&pattern)?;
    let captures = regex
        .captures(text)
        .with_context(|| format!("failed to parse Anthropic pricing row for `{label}`"))?;
    Ok(TokenPricePerMillion {
        input: parse_price(&captures["input"])?,
        cache: parse_price(&captures["cache_hit"])?,
        output: parse_price(&captures["output"])?,
    })
}

fn capture_deepseek_flash_row(text: &str) -> anyhow::Result<TokenPricePerMillion> {
    let regex = Regex::new(
        r"1M INPUT TOKENS \(CACHE HIT\).*?\$(?P<cache>[0-9.]+)\$[0-9.]+.*?1M INPUT TOKENS \(CACHE MISS\)\$(?P<input>[0-9.]+)\$[0-9.]+.*?1M OUTPUT TOKENS\$(?P<output>[0-9.]+)\$[0-9.]+",
    )?;
    let captures = regex
        .captures(text)
        .context("failed to parse DeepSeek pricing row for `deepseek-v4-flash`")?;
    Ok(TokenPricePerMillion {
        input: parse_price(&captures["input"])?,
        cache: parse_price(&captures["cache"])?,
        output: parse_price(&captures["output"])?,
    })
}

fn capture_kimi_row(text: &str, model: &str) -> anyhow::Result<TokenPricePerMillion> {
    let pattern = format!(
        r"{}.*?Cache Hit\s*\$?(?P<cache>[0-9.]+)\s*/\s*MTok\s*Input\s*\$?(?P<input>[0-9.]+)\s*/\s*MTok\s*Output\s*\$?(?P<output>[0-9.]+)\s*/\s*MTok",
        regex::escape(model)
    );
    let regex = Regex::new(&pattern)?;
    let captures = regex
        .captures(text)
        .with_context(|| format!("failed to parse Kimi pricing row for `{model}`"))?;
    Ok(TokenPricePerMillion {
        input: parse_price(&captures["input"])?,
        cache: parse_price(&captures["cache"])?,
        output: parse_price(&captures["output"])?,
    })
}

fn capture_minimax_row(text: &str, model: &str) -> anyhow::Result<TokenPricePerMillion> {
    let pattern = format!(
        r"{}\$(?P<input>[0-9.]+) / M tokens\$(?P<output>[0-9.]+) / M tokens\$(?P<cache>[0-9.]+) / M tokens\$(?P<cache_write>[0-9.]+) / M tokens",
        regex::escape(model)
    );
    let regex = Regex::new(&pattern)?;
    let captures = regex
        .captures(text)
        .with_context(|| format!("failed to parse MiniMax pricing row for `{model}`"))?;
    Ok(TokenPricePerMillion {
        input: parse_price(&captures["input"])?,
        cache: parse_price(&captures["cache"])?,
        output: parse_price(&captures["output"])?,
    })
}

fn capture_qwen_row(text: &str, token_band: &str) -> anyhow::Result<TokenPricePerMillion> {
    let pattern = format!(
        r"{}\s+(?P<input>[0-9.]+) 元\s+(?P<output>[0-9.]+) 元\s+(?P<thinking_output>[0-9.]+) 元",
        regex::escape(token_band)
    );
    let regex = Regex::new(&pattern)?;
    let captures = regex
        .captures(text)
        .with_context(|| format!("failed to parse Qwen pricing row for `{token_band}`"))?;
    Ok(TokenPricePerMillion {
        input: parse_price(&captures["input"])?,
        cache: 0.0,
        output: parse_price(&captures["output"])?,
    })
}

fn parse_safe_usd_cny_rate(text: &str) -> anyhow::Result<f64> {
    let regex = Regex::new(r"\d{4}-\d{2}-\d{2}\s+(?P<usd>[0-9.]+)\s")?;
    let captures = regex
        .captures(text)
        .context("failed to parse SAFE USD/CNY middle rate")?;
    let cny_per_100_usd = parse_price(&captures["usd"])?;
    Ok(cny_per_100_usd / 100.0)
}

fn parse_price(value: &str) -> anyhow::Result<f64> {
    let price = value
        .parse::<f64>()
        .with_context(|| format!("invalid price value `{value}`"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("invalid non-finite price `{value}`");
    }
    Ok(price)
}

pub fn calculate_cost(
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
    input_price_per_1k_usd: f64,
    output_price_per_1k_usd: f64,
    cache_price_per_1k_usd: f64,
) -> f64 {
    (input_tokens as f64 / 1000.0 * input_price_per_1k_usd)
        + (output_tokens as f64 / 1000.0 * output_price_per_1k_usd)
        + (cached_tokens as f64 / 1000.0 * cache_price_per_1k_usd)
}

#[derive(Debug, Clone, Copy)]
pub struct CatalogPriceRow {
    pub input_price_per_1k_usd: f64,
    pub output_price_per_1k_usd: f64,
    pub cache_price_per_1k_usd: f64,
}

pub async fn find_effective_price(
    pool: &PgPool,
    model: &str,
    occurred_at: DateTime<Utc>,
    input_tokens: u64,
) -> anyhow::Result<Option<CatalogPriceRow>> {
    sqlx::query_as::<_, (f64, f64, f64)>(
        r#"
        SELECT
            input_price_per_1k_usd::double precision,
            output_price_per_1k_usd::double precision,
            cache_price_per_1k_usd::double precision
        FROM model_catalog
        WHERE model = $1
          AND effective_from <= $2
          AND (max_input_tokens = 0 OR $3 <= max_input_tokens)
        ORDER BY
            CASE WHEN max_input_tokens = 0 THEN 1 ELSE 0 END,
            max_input_tokens ASC,
            effective_from DESC
        LIMIT 1
        "#,
    )
    .bind(model)
    .bind(occurred_at)
    .bind(input_tokens as i64)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(
            |(input_price_per_1k_usd, output_price_per_1k_usd, cache_price_per_1k_usd)| {
                CatalogPriceRow {
                    input_price_per_1k_usd,
                    output_price_per_1k_usd,
                    cache_price_per_1k_usd,
                }
            },
        )
    })
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_cost, capture_anthropic_row, capture_deepseek_flash_row, capture_kimi_row,
        capture_minimax_row, capture_openai_row, capture_qwen_row, parse_safe_usd_cny_rate,
    };

    #[test]
    fn parses_openai_row() {
        let text = "Flagship models Standard Model Input Cached input Output gpt-5.5$5.00$0.50$30.00$10.00$1.00$45.00 gpt-5.4$2.50$0.25$15.00";
        let row = capture_openai_row(text, "gpt-5.5").expect("openai row");
        assert_eq!(row.input, 5.0);
        assert_eq!(row.cache, 0.5);
        assert_eq!(row.output, 30.0);
    }

    #[test]
    fn parses_anthropic_row() {
        let text = "Model pricing Claude Opus 4.7$5 / MTok$6.25 / MTok$10 / MTok$0.50 / MTok$25 / MTok Claude Opus 4.1$15 / MTok$18.75 / MTok$30 / MTok$1.50 / MTok$75 / MTok";
        let row = capture_anthropic_row(text, "Claude Opus 4.1").expect("anthropic row");
        assert_eq!(row.input, 15.0);
        assert_eq!(row.cache, 1.5);
        assert_eq!(row.output, 75.0);
    }

    #[test]
    fn parses_deepseek_flash_row() {
        let text = "Model Details PRICING 1M INPUT TOKENS (CACHE HIT)(2)$0.0028$0.003625 1M INPUT TOKENS (CACHE MISS)$0.14$0.435 1M OUTPUT TOKENS$0.28$0.87 Deduction Rules";
        let row = capture_deepseek_flash_row(text).expect("deepseek row");
        assert_eq!(row.input, 0.14);
        assert_eq!(row.cache, 0.0028);
        assert_eq!(row.output, 0.28);
    }

    #[test]
    fn parses_kimi_row() {
        let text = "K2.6 kimi-k2.6 is Kimi's latest and most intelligent model Cache Hit$0.16 / MTok Input$0.95 / MTok Output$4.00 / MTok";
        let row = capture_kimi_row(text, "kimi-k2.6").expect("kimi row");
        assert_eq!(row.input, 0.95);
        assert_eq!(row.cache, 0.16);
        assert_eq!(row.output, 4.0);
    }

    #[test]
    fn parses_minimax_row() {
        let text = "Text Model Input Output Prompt caching Read Prompt caching Write MiniMax-M2.7$0.3 / M tokens$1.2 / M tokens$0.06 / M tokens$0.375 / M tokens";
        let row = capture_minimax_row(text, "MiniMax-M2.7").expect("minimax row");
        assert_eq!(row.input, 0.3);
        assert_eq!(row.cache, 0.06);
        assert_eq!(row.output, 1.2);
    }

    #[test]
    fn parses_qwen_row() {
        let text = "qwen3.6-plus 0<Token≤256K 3.7471 元 22.4826 元 22.4826 元 256K<Token≤1M 14.9884 元 44.965 元 44.965 元 qwen3.5-plus";
        let row = capture_qwen_row(text, "0<Token≤256K").expect("qwen row");
        assert_eq!(row.input, 3.7471);
        assert_eq!(row.output, 22.4826);
        assert_eq!(row.cache, 0.0);
    }

    #[test]
    fn parses_safe_usd_cny_rate() {
        let text = "日期 美元 欧元 2026-05-14 684.01 798.6";
        let rate = parse_safe_usd_cny_rate(text).expect("safe fx");
        assert!((rate - 6.8401).abs() < 1e-9);
    }

    #[test]
    fn calculates_cost_from_per_1k_prices() {
        let cost = calculate_cost(1000, 2000, 500, 0.005, 0.03, 0.0005);
        assert!((cost - 0.06525).abs() < 1e-9);
    }
}
