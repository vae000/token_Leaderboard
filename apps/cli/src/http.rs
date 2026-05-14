use anyhow::Context;
use common::{
    AuthCallbackResponse, AuthStartResponse, BatchIngestRequest, BatchIngestResponse,
    RefreshTokenRequest, RefreshTokenResponse,
};
use reqwest::Client;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LeaderboardClient {
    base_url: String,
    client: Client,
}

impl LeaderboardClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            client: Client::new(),
        }
    }

    pub async fn healthz(&self) -> anyhow::Result<Value> {
        self.client
            .get(format!("{}/healthz", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(Into::into)
    }

    pub async fn start_auth(&self) -> anyhow::Result<AuthStartResponse> {
        self.client
            .post(format!("{}/v1/auth/cli/start", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(Into::into)
    }

    pub async fn complete_auth(
        &self,
        device_id: &str,
        user_id: &str,
        display_name: &str,
    ) -> anyhow::Result<AuthCallbackResponse> {
        self.client
            .get(format!("{}/v1/auth/cli/callback", self.base_url))
            .query(&[
                ("device_id", device_id),
                ("user_id", user_id),
                ("display_name", display_name),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(Into::into)
    }

    pub async fn refresh_token(
        &self,
        request: &RefreshTokenRequest,
    ) -> anyhow::Result<RefreshTokenResponse> {
        self.client
            .post(format!("{}/v1/auth/cli/refresh", self.base_url))
            .json(request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(Into::into)
    }

    pub async fn ingest_batch(
        &self,
        request: &BatchIngestRequest,
    ) -> anyhow::Result<BatchIngestResponse> {
        self.client
            .post(format!("{}/v1/ingest/events:batch", self.base_url))
            .json(request)
            .send()
            .await
            .with_context(|| "failed to send ingest request")?
            .error_for_status()?
            .json()
            .await
            .map_err(Into::into)
    }
}
