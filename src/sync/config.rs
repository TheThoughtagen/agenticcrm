use std::fs;
use std::path::PathBuf;

const SERVICE_NAME: &str = "acrm-icloud";

/// Sync configuration loaded from the config file.
pub struct SyncConfig {
    pub apple_id: String,
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
}
