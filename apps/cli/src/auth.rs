use chrono::Utc;

use crate::{config::CliState, http::LeaderboardClient};

pub async fn login(
    api_base_url: &str,
    user_id: &str,
    name: &str,
) -> anyhow::Result<()> {
    let state = CliState::load()?;

    // 已登录状态：直接查 Web 凭证
    if state.device_id.is_some() && state.user_id.is_some() {
        let client = LeaderboardClient::new(api_base_url);
        match client.get_credentials(user_id).await {
            Ok(cred) => {
                println!("✅ 已登录 (账号: {})", cred.user_id);
                let web_url = web_base_url(api_base_url);
                println!();
                println!("🌐 Web 登录");
                println!("   地址: {}/login", web_url);
                println!("   账号: {}", cred.username);
                println!("   密码: {}", cred.password);
            }
            Err(e) => {
                println!("✅ 已登录 (账号: {user_id})");
                println!("(获取 Web 登录凭证失败: {e})");
                println!("请确认 API 服务已启动且 API__AUTO_PASSWORD_SALT 已配置。");
            }
        }
        return Ok(());
    }

    // 首次登录：注册设备
    let client = LeaderboardClient::new(api_base_url);
    let start = client.start_auth().await?;
    let callback = client
        .complete_auth(&start.device_id, user_id, name)
        .await?;

    let new_state = CliState {
        api_base_url: Some(api_base_url.into()),
        device_id: Some(start.device_id.clone()),
        user_id: Some(callback.user_id.clone()),
        display_name: Some(callback.display_name.clone()),
        access_token: Some(callback.access_token.clone()),
        refresh_token: Some(callback.refresh_token.clone()),
        last_sync_at: Some(Utc::now()),
        upload_cursors: Default::default(),
    };
    new_state.save()?;

    println!("✅ 终端注册成功");
    println!("   用户: {} ({})", callback.display_name, callback.user_id);
    println!();

    // 获取 Web 登录凭证
    match client.get_credentials(user_id).await {
        Ok(cred) => {
            let web_url = web_base_url(api_base_url);
            println!("🌐 Web 登录");
            println!("   地址: {}/login", web_url);
            println!("   账号: {}", cred.username);
            println!("   密码: {}", cred.password);
            println!();
            println!("提示: 打开浏览器访问以上地址，输入账号密码即可查看数据。");
            println!("      守护进程启动后会自动采集数据同步到服务器。");
        }
        Err(e) => {
            println!("(获取 Web 登录凭证失败: {e})");
            println!("请确认 API 服务已启动且 API__AUTO_PASSWORD_SALT 已配置。");
        }
    }

    Ok(())
}

pub fn logout() -> anyhow::Result<()> {
    CliState::clear()?;
    println!("本地登录状态已清除");
    Ok(())
}

fn web_base_url(api_base_url: &str) -> String {
    // 从 API URL 推导 Web URL: 把 8080 端口换成 3000, 127.0.0.1 换成 localhost
    api_base_url
        .replace(":8080", ":3000")
        .replace("127.0.0.1", "localhost")
}
