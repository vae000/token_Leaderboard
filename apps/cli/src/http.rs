use anyhow::Context;
use common::{
    AuthCallbackResponse, AuthStartResponse, BatchIngestRequest, BatchIngestResponse,
    GeneratedCredential, RefreshTokenRequest, RefreshTokenResponse,
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

    /// 发送请求，连接失败时给出友好提示。
    async fn send(
        &self,
        builder: reqwest::RequestBuilder,
        action: &str,
    ) -> anyhow::Result<reqwest::Response> {
        builder.send().await.with_context(|| {
            format!(
                "无法连接到 API 服务 ({})。{} 失败。\n请确认 API 已启动: cargo run -p api",
                self.base_url, action
            )
        })
    }

    pub async fn healthz(&self) -> anyhow::Result<Value> {
        self.send(
            self.client.get(format!("{}/healthz", self.base_url)),
            "健康检查",
        )
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
    }

    pub async fn start_auth(&self, device_id: &str) -> anyhow::Result<AuthStartResponse> {
        self.send(
            self.client
                .post(format!("{}/v1/auth/cli/start", self.base_url))
                .json(&common::AuthStartRequest {
                    device_id: device_id.to_owned(),
                }),
            "认证启动",
        )
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
        self.send(
            self.client
                .get(format!("{}/v1/auth/cli/callback", self.base_url))
                .query(&[
                    ("device_id", device_id),
                    ("user_id", user_id),
                    ("display_name", display_name),
                ]),
            "认证回调",
        )
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
        self.send(
            self.client
                .post(format!("{}/v1/auth/cli/refresh", self.base_url))
                .json(request),
            "Token 刷新",
        )
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
    }

    /// 获取指定用户的 Web 登录凭证（账号 + 密码）。
    pub async fn get_credentials(&self, user_id: &str) -> anyhow::Result<GeneratedCredential> {
        self.send(
            self.client.get(format!(
                "{}/v1/admin/users/{}/credentials",
                self.base_url, user_id
            )),
            "获取 Web 凭证",
        )
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
        self.send(
            self.client
                .post(format!("{}/v1/ingest/events:batch", self.base_url))
                .json(request),
            "事件上传",
        )
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
    }
}
