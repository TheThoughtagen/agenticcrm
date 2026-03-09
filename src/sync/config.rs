use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

const SERVICE_NAME: &str = "acrm-icloud";

/// Sync configuration loaded from the config file (sync.toml).
#[derive(Debug, Deserialize)]
pub struct SyncConfig {
    /// Required by TOML deserialization; credentials are loaded separately via keyring.
    #[allow(dead_code)]
    pub apple_id: String,
    #[serde(default)]
    pub push_filters: FilterConfig,
    #[serde(default)]
    pub pull_filters: FilterConfig,
}

/// Filter configuration for push or pull direction.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct FilterConfig {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub statuses: Vec<String>,
}

/// Returns the acrm config directory (~/.config/acrm/), creating it if needed.
pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("acrm");
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
    dir
}

/// Store credentials: apple_id in config file, app_password in macOS Keychain.
pub fn store_credentials(apple_id: &str, app_password: &str) -> anyhow::Result<()> {
    // Store apple_id in config file
    let config_path = config_dir().join("sync.toml");
    let content = format!("apple_id = \"{}\"\n", apple_id);
    fs::write(&config_path, content)?;

    // Store app_password in macOS Keychain via keyring
    let entry = keyring::Entry::new(SERVICE_NAME, apple_id)?;
    entry.set_password(app_password)?;

    Ok(())
}

/// Load credentials: apple_id from config file, app_password from macOS Keychain.
/// Returns (apple_id, app_password).
pub fn load_credentials() -> anyhow::Result<(String, String)> {
    let config_path = config_dir().join("sync.toml");

    if !config_path.exists() {
        anyhow::bail!("No sync configuration found. Run `acrm sync setup` first.");
    }

    let content = fs::read_to_string(&config_path)?;
    let apple_id = parse_apple_id(&content).ok_or_else(|| {
        anyhow::anyhow!("Invalid sync config file. Run `acrm sync setup` to reconfigure.")
    })?;

    let entry = keyring::Entry::new(SERVICE_NAME, &apple_id)?;
    let app_password = entry.get_password().map_err(|_| {
        anyhow::anyhow!(
            "Could not retrieve app password from keychain. Run `acrm sync setup` to reconfigure."
        )
    })?;

    Ok((apple_id, app_password))
}

/// Load full sync configuration from sync.toml, including filter sections.
/// Returns a SyncConfig with push_filters and pull_filters (defaulting to empty).
pub fn load_sync_config() -> anyhow::Result<SyncConfig> {
    let config_path = config_dir().join("sync.toml");

    if !config_path.exists() {
        anyhow::bail!("No sync configuration found. Run `acrm sync setup` first.");
    }

    let content = fs::read_to_string(&config_path)?;
    let config: SyncConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Parse apple_id from a minimal TOML config (just `apple_id = "..."` line).
fn parse_apple_id(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("apple_id") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim();
                // Strip surrounding quotes
                if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
                    return Some(rest[1..rest.len() - 1].to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apple_id() {
        assert_eq!(
            parse_apple_id("apple_id = \"user@icloud.com\"\n"),
            Some("user@icloud.com".to_string())
        );
    }

    #[test]
    fn test_parse_apple_id_with_spaces() {
        assert_eq!(
            parse_apple_id("  apple_id  =  \"test@example.com\"  \n"),
            Some("test@example.com".to_string())
        );
    }

    #[test]
    fn test_parse_apple_id_missing() {
        assert_eq!(parse_apple_id("other_key = \"value\"\n"), None);
    }

    #[test]
    fn test_config_dir() {
        let dir = config_dir();
        assert!(dir.ends_with("acrm"));
    }

    #[test]
    fn test_sync_config_deserialize_with_filters() {
        let toml_str = r#"
apple_id = "user@icloud.com"

[push_filters]
tags = ["work", "vip"]
statuses = ["active"]

[pull_filters]
tags = ["personal"]
statuses = ["active", "dormant"]
"#;
        let config: SyncConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.apple_id, "user@icloud.com");
        assert_eq!(config.push_filters.tags, vec!["work", "vip"]);
        assert_eq!(config.push_filters.statuses, vec!["active"]);
        assert_eq!(config.pull_filters.tags, vec!["personal"]);
        assert_eq!(
            config.pull_filters.statuses,
            vec!["active", "dormant"]
        );
    }

    #[test]
    fn test_sync_config_deserialize_legacy_no_filters() {
        let toml_str = "apple_id = \"user@icloud.com\"\n";
        let config: SyncConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.apple_id, "user@icloud.com");
        assert!(config.push_filters.tags.is_empty());
        assert!(config.push_filters.statuses.is_empty());
        assert!(config.pull_filters.tags.is_empty());
        assert!(config.pull_filters.statuses.is_empty());
    }

    #[test]
    fn test_store_credentials_writes_parseable_toml() {
        // Verify the format string used in store_credentials is valid TOML
        let apple_id = "test@example.com";
        let content = format!("apple_id = \"{}\"\n", apple_id);
        let config: SyncConfig = toml::from_str(&content).unwrap();
        assert_eq!(config.apple_id, apple_id);
    }
}
