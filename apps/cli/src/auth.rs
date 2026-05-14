use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::{config::CliState, http::LeaderboardClient};

/// 从设备标识派生 user_id（基于 MAC 地址，保证不重复）。
fn derive_user_id(device_id: &str) -> String {
    if let Some(hex) = device_id.strip_prefix("fallback_") {
        // fallback UUID → 取前 8 位
        format!("u_fb_{}", &hex[..8])
    } else {
        // MAC 地址 → 去掉冒号
        format!("u_{}", device_id.replace(':', ""))
    }
}

/// 获取本机 MAC 地址（首个非回环物理网卡）。
pub fn get_mac_address() -> Option<String> {
    let net_dir = Path::new("/sys/class/net");
    if !net_dir.exists() {
        return None;
    }

    let entries = fs::read_dir(net_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // 跳过回环和虚拟接口
        if name_str == "lo"
            || name_str.starts_with("docker")
            || name_str.starts_with("veth")
            || name_str.starts_with("br-")
            || name_str.starts_with("virbr")
        {
            continue;
        }

        let addr_path = entry.path().join("address");
        if let Ok(addr) = fs::read_to_string(&addr_path) {
            let addr = addr.trim();
            if !addr.is_empty() && addr != "00:00:00:00:00:00" {
                return Some(addr.to_string());
            }
        }
    }
    None
}

pub async fn login(api_base_url: &str, name: Option<&str>) -> anyhow::Result<()> {
    let mac = get_mac_address().unwrap_or_else(|| format!("fallback_{}", uuid::Uuid::new_v4()));
    let device_id = mac.clone();

    let state = CliState::load()?;
    let client = LeaderboardClient::new(api_base_url);

    // 调用 start_auth，传入 MAC 作为 device_id
    let start = client.start_auth(&device_id).await?;

    // 设备已绑定 → 直接返回
    if let (Some(bound_user), Some(access_token), Some(refresh_token), Some(_expires_at)) = (
        start.bound_user,
        start.access_token,
        start.refresh_token,
        start.expires_at,
    ) {
        let new_state = CliState {
            api_base_url: Some(api_base_url.into()),
            device_id: Some(device_id.clone()),
            user_id: Some(bound_user.user_id.clone()),
            display_name: Some(bound_user.display_name.clone()),
            access_token: Some(access_token),
            refresh_token: Some(refresh_token),
            last_sync_at: Some(Utc::now()),
            upload_cursors: state.upload_cursors,
        };
        new_state.save()?;

        println!("✅ 已登录 (MAC: {})", device_id);
        println!(
            "   用户: {} ({})",
            bound_user.display_name, bound_user.user_id
        );
        println!();

        // 获取 Web 登录凭证
        match client.get_credentials(&bound_user.user_id).await {
            Ok(cred) => {
                let web_url = web_base_url(api_base_url);
                println!("🌐 Web 登录");
                println!("   地址: {}/login", web_url);
                println!("   账号: {}", cred.username);
                println!("   密码: {}", cred.password);
            }
            Err(e) => {
                println!("(获取 Web 登录凭证失败: {e})");
                println!("请确认 API 服务已启动且 API__AUTO_PASSWORD_SALT 已配置。");
            }
        }
        return Ok(());
    }

    // 设备未绑定 → 自动从 MAC 派生 user_id，首次绑定
    let user_id = derive_user_id(&device_id);
    let name = name.unwrap_or(&user_id).to_string();

    let callback = client.complete_auth(&device_id, &user_id, &name).await?;

    let new_state = CliState {
        api_base_url: Some(api_base_url.into()),
        device_id: Some(device_id.clone()),
        user_id: Some(callback.user_id.clone()),
        display_name: Some(callback.display_name.clone()),
        access_token: Some(callback.access_token.clone()),
        refresh_token: Some(callback.refresh_token.clone()),
        last_sync_at: Some(Utc::now()),
        upload_cursors: Default::default(),
    };
    new_state.save()?;

    println!("✅ 终端绑定成功 (MAC: {})", device_id);
    println!("   用户: {} ({})", callback.display_name, callback.user_id);
    println!();

    // 获取 Web 登录凭证
    match client.get_credentials(&user_id).await {
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
