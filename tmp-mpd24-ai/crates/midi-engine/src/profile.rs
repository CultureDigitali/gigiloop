use std::{fs, path::Path};

use mpd24_ai_common::ControllerProfile;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("failed to read or write controller profile: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid controller profile JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid controller profile id: {0}")]
    InvalidId(String),
}

pub fn matches_device(profile: &ControllerProfile, device_name: &str) -> bool {
    let device_name = device_name.to_lowercase();
    profile
        .name_matches
        .iter()
        .any(|needle| device_name.contains(&needle.to_lowercase()))
}

pub fn load_profile(path: &Path) -> Result<ControllerProfile, ProfileError> {
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn save_profile(path: &Path, profile: &ControllerProfile) -> Result<(), ProfileError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(profile)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

pub fn profile_file_name(profile_id: &str) -> Result<String, ProfileError> {
    let valid = !profile_id.is_empty()
        && profile_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));

    if !valid {
        return Err(ProfileError::InvalidId(profile_id.to_owned()));
    }

    Ok(format!("{profile_id}.json"))
}
