use chrono::Utc;

use crate::{config::CliState, http::LeaderboardClient};

pub async fn login(api_base_url: &str, user_id: &str, name: &str) -> anyhow::Result<()> {
    let client = LeaderboardClient::new(api_base_url);
    let start = client.start_auth().await?;
    let callback = client
        .complete_auth(&start.device_id, user_id, name)
        .await?;

    let state = CliState {
        api_base_url: Some(api_base_url.into()),
        device_id: Some(start.device_id.clone()),
        user_id: Some(callback.user_id.clone()),
        display_name: Some(callback.display_name.clone()),
        access_token: Some(callback.access_token.clone()),
        refresh_token: Some(callback.refresh_token.clone()),
        last_sync_at: Some(Utc::now()),
        upload_cursors: Default::default(),
    };
    state.save()?;

    println!("login_url: {}", start.login_url);
    println!(
        "logged_in_as: {} ({})",
        callback.display_name, callback.user_id
    );
    Ok(())
}

pub fn logout() -> anyhow::Result<()> {
    CliState::clear()?;
    println!("local auth state cleared");
    Ok(())
}
