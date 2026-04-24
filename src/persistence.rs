use std::collections::HashSet;
use std::path::Path;
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use teloxide::types::UserId;
use crate::{BotError, Result, stt::SttProvider};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AuthorizedUsersData {
    pub users: HashSet<u64>,
}

const USERS_FILE: &str = "data/authorized_users.json";
const RUNTIME_CONFIG_FILE: &str = "data/runtime_config.json";

impl AuthorizedUsersData {
    pub fn from_user_ids(user_ids: &HashSet<UserId>) -> Self {
        Self {
            users: user_ids.iter().map(|id| id.0).collect(),
        }
    }

    pub fn to_user_ids(&self) -> HashSet<UserId> {
        self.users.iter().map(|&id| UserId(id)).collect()
    }
}

pub async fn load_authorized_users() -> Result<HashSet<UserId>> {
    // Create data directory if it doesn't exist
    if let Some(parent) = Path::new(USERS_FILE).parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await.map_err(BotError::Io)?;
            info!("Created data directory: {}", parent.display());
        }
    }

    if !Path::new(USERS_FILE).exists() {
        info!("No authorized users file found, starting with empty list");
        return Ok(HashSet::new());
    }

    match tokio::fs::read_to_string(USERS_FILE).await {
        Ok(contents) => {
            match serde_json::from_str::<AuthorizedUsersData>(&contents) {
                Ok(data) => {
                    let user_ids = data.to_user_ids();
                    info!("Loaded {} authorized users from {}", user_ids.len(), USERS_FILE);
                    Ok(user_ids)
                }
                Err(e) => {
                    warn!("Failed to parse authorized users file: {}, starting with empty list", e);
                    Ok(HashSet::new())
                }
            }
        }
        Err(e) => {
            warn!("Failed to read authorized users file: {}, starting with empty list", e);
            Ok(HashSet::new())
        }
    }
}

pub async fn save_authorized_users(user_ids: &HashSet<UserId>) -> Result<()> {
    // Create data directory if it doesn't exist
    if let Some(parent) = Path::new(USERS_FILE).parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await.map_err(BotError::Io)?;
            info!("Created data directory: {}", parent.display());
        }
    }

    let data = AuthorizedUsersData::from_user_ids(user_ids);

    match serde_json::to_string_pretty(&data) {
        Ok(json_content) => {
            match tokio::fs::write(USERS_FILE, json_content).await {
                Ok(_) => {
                    info!("Saved {} authorized users to {}", user_ids.len(), USERS_FILE);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to write authorized users file: {}", e);
                    Err(BotError::Io(e))
                }
            }
        }
        Err(e) => {
            error!("Failed to serialize authorized users: {}", e);
            Err(BotError::Config(format!("JSON serialization error: {}", e)))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RuntimeConfigData {
    stt_provider: String,
}

pub async fn load_runtime_config() -> Result<Option<SttProvider>> {
    if !Path::new(RUNTIME_CONFIG_FILE).exists() {
        return Ok(None);
    }

    match tokio::fs::read_to_string(RUNTIME_CONFIG_FILE).await {
        Ok(contents) => {
            match serde_json::from_str::<RuntimeConfigData>(&contents) {
                Ok(data) => {
                    match SttProvider::from_str(&data.stt_provider) {
                        Some(provider) => {
                            info!("Loaded runtime config: provider={}", data.stt_provider);
                            Ok(Some(provider))
                        }
                        None => {
                            warn!("Unknown provider '{}' in runtime config, ignoring", data.stt_provider);
                            Ok(None)
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse runtime config: {}, ignoring", e);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            warn!("Failed to read runtime config: {}, ignoring", e);
            Ok(None)
        }
    }
}

pub async fn save_runtime_config(provider: SttProvider) -> Result<()> {
    if let Some(parent) = Path::new(RUNTIME_CONFIG_FILE).parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await.map_err(BotError::Io)?;
        }
    }

    let data = RuntimeConfigData {
        stt_provider: provider.as_str().to_string(),
    };

    match serde_json::to_string_pretty(&data) {
        Ok(json_content) => {
            tokio::fs::write(RUNTIME_CONFIG_FILE, json_content)
                .await
                .map_err(|e| {
                    error!("Failed to write runtime config: {}", e);
                    BotError::Io(e)
                })?;
            info!("Saved runtime config: provider={}", provider.as_str());
            Ok(())
        }
        Err(e) => {
            error!("Failed to serialize runtime config: {}", e);
            Err(BotError::Config(format!("JSON serialization error: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_authorized_users_data_conversion() {
        let mut user_ids = HashSet::new();
        user_ids.insert(UserId(123456789));
        user_ids.insert(UserId(987654321));

        let data = AuthorizedUsersData::from_user_ids(&user_ids);
        let converted_back = data.to_user_ids();

        assert_eq!(user_ids, converted_back);
    }
}
