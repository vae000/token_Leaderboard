use anyhow::{Context, bail};
use reqwest::Client;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone)]
pub struct WechatConfig {
    pub app_id: String,
    pub app_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone)]
pub struct WechatUserInfo {
    pub openid: String,
    pub unionid: Option<String>,
    pub nickname: String,
}

#[derive(Debug, Deserialize)]
struct WechatTokenResponse {
    access_token: Option<String>,
    openid: Option<String>,
    unionid: Option<String>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WechatUserInfoResponse {
    openid: Option<String>,
    unionid: Option<String>,
    nickname: Option<String>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

pub fn build_qr_connect_url(config: &WechatConfig, state: &str) -> anyhow::Result<String> {
    let mut url = Url::parse("https://open.weixin.qq.com/connect/qrconnect")
        .context("invalid wechat qrconnect base url")?;
    url.query_pairs_mut()
        .append_pair("appid", &config.app_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "snsapi_login")
        .append_pair("state", state);
    Ok(format!("{url}#wechat_redirect"))
}

pub async fn fetch_user_info(
    client: &Client,
    config: &WechatConfig,
    code: &str,
) -> anyhow::Result<WechatUserInfo> {
    let token = exchange_code(client, config, code).await?;
    let access_token = token
        .access_token
        .context("wechat token response missing access_token")?;
    let openid = token
        .openid
        .context("wechat token response missing openid")?;
    let mut url = Url::parse("https://api.weixin.qq.com/sns/userinfo")
        .context("invalid wechat userinfo base url")?;
    url.query_pairs_mut()
        .append_pair("access_token", &access_token)
        .append_pair("openid", &openid);

    let response = client
        .get(url)
        .send()
        .await
        .context("failed to fetch wechat user info")?
        .error_for_status()
        .context("wechat user info request failed")?;
    let payload: WechatUserInfoResponse = response
        .json()
        .await
        .context("failed to parse wechat user info response")?;

    if let Some(code) = payload.errcode {
        bail!(
            "wechat userinfo failed with errcode {}: {}",
            code,
            payload.errmsg.unwrap_or_else(|| "unknown error".into())
        );
    }

    Ok(WechatUserInfo {
        openid: payload.openid.unwrap_or(openid),
        unionid: payload.unionid.or(token.unionid),
        nickname: payload
            .nickname
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "微信用户".into()),
    })
}

async fn exchange_code(
    client: &Client,
    config: &WechatConfig,
    code: &str,
) -> anyhow::Result<WechatTokenResponse> {
    let mut url = Url::parse("https://api.weixin.qq.com/sns/oauth2/access_token")
        .context("invalid wechat token base url")?;
    url.query_pairs_mut()
        .append_pair("appid", &config.app_id)
        .append_pair("secret", &config.app_secret)
        .append_pair("code", code)
        .append_pair("grant_type", "authorization_code");

    let response = client
        .get(url)
        .send()
        .await
        .context("failed to exchange wechat oauth code")?
        .error_for_status()
        .context("wechat oauth token request failed")?;
    let payload: WechatTokenResponse = response
        .json()
        .await
        .context("failed to parse wechat oauth token response")?;

    if let Some(code) = payload.errcode {
        bail!(
            "wechat oauth token exchange failed with errcode {}: {}",
            code,
            payload.errmsg.unwrap_or_else(|| "unknown error".into())
        );
    }

    Ok(payload)
}
