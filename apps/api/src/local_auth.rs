use sha2::{Digest, Sha256};

pub fn username_for_user_id(user_id: &str) -> String {
    user_id.trim().to_owned()
}

pub fn password_for_user_id(user_id: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(user_id.trim().as_bytes());
    let digest = hasher.finalize();
    let suffix = digest[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .to_uppercase();
    format!("TL-{suffix}")
}
