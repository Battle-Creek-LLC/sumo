//! Sumo Logic CLI configuration.
//!
//! Credentials and project definitions live in a TOML file (default
//! `~/.config/sumo/config.toml` on Linux, equivalent on macOS / Windows
//! via the `directories` crate). Resolution precedence for any single
//! command is:
//!
//!     CLI flag  >  environment variable  >  named project in file  >  file defaults

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_PROJECT: &str = "default";

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub access_id: String,
    pub access_key: String,
    pub endpoint: String,
    pub project: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct FileConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_project: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub projects: BTreeMap<String, Project>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Project {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Overrides {
    pub access_id: Option<String>,
    pub access_key: Option<String>,
    pub endpoint: Option<String>,
    pub project: Option<String>,
    pub config_path: Option<PathBuf>,
}

pub fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "battlecreek", "sumo")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

pub fn config_path(override_path: Option<PathBuf>) -> Result<PathBuf, String> {
    override_path
        .or_else(|| std::env::var("SUMO_CONFIG").ok().map(PathBuf::from))
        .or_else(default_config_path)
        .ok_or_else(|| "Could not determine config file path".to_string())
}

pub fn load_or_default(path: &Path) -> Result<FileConfig, String> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    load_file(path)
}

pub fn load_file(path: &Path) -> Result<FileConfig, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("reading config file {}: {e}", path.display()))?;
    toml::from_str(&raw)
        .map_err(|e| format!("parsing config file {}: {e}", path.display()))
}

pub fn save_file(path: &Path, config: &FileConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating config dir {}: {e}", parent.display()))?;
    }
    let serialized = toml::to_string_pretty(config)
        .map_err(|e| format!("serializing config: {e}"))?;
    std::fs::write(path, serialized)
        .map_err(|e| format!("writing config file {}: {e}", path.display()))?;
    restrict_permissions(path);
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

pub fn resolve(overrides: Overrides) -> Result<ResolvedConfig, String> {
    let env_id = std::env::var("SUMO_ACCESS_ID").ok();
    let env_key = std::env::var("SUMO_ACCESS_KEY").ok();
    let env_endpoint = std::env::var("SUMO_API_ENDPOINT").ok();
    let env_project = std::env::var("SUMO_PROJECT").ok();

    let path = config_path(overrides.config_path.clone())?;
    let file = load_or_default(&path)?;

    let project_name = overrides
        .project
        .clone()
        .or(env_project)
        .or_else(|| file.default_project.clone())
        .unwrap_or_else(|| DEFAULT_PROJECT.to_string());

    // If an explicit project was requested (via flag, env, or file default),
    // ensure it exists in the config file. Otherwise treat as not-found.
    let project_explicit = overrides.project.is_some()
        || std::env::var("SUMO_PROJECT").is_ok()
        || file.default_project.is_some();

    let project = file.projects.get(&project_name);

    if project_explicit && project.is_none() && !file.projects.is_empty() {
        return Err(format!(
            "Project '{project_name}' not found in {}. Run 'sumo auth list' to see available projects.",
            path.display()
        ));
    }

    let access_id = overrides
        .access_id
        .or(env_id)
        .or_else(|| project.and_then(|p| p.access_id.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Not authenticated. Run 'sumo auth login' to set up credentials.".to_string()
        })?;
    let access_key = overrides
        .access_key
        .or(env_key)
        .or_else(|| project.and_then(|p| p.access_key.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Not authenticated. Run 'sumo auth login' to set up credentials.".to_string()
        })?;
    let endpoint = overrides
        .endpoint
        .or(env_endpoint)
        .or_else(|| project.and_then(|p| p.endpoint.clone()))
        .or(file.default_endpoint)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Not authenticated. Run 'sumo auth login' to set up credentials.".to_string()
        })?;

    Ok(ResolvedConfig {
        access_id,
        access_key,
        endpoint,
        project: project_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_projects_toml() {
        let toml_str = r#"
            default_endpoint = "https://api.eu.sumologic.com/api"
            default_project = "prod"

            [projects.prod]
            access_id = "id1"
            access_key = "key1"

            [projects.staging]
            access_id = "id2"
            access_key = "key2"
            endpoint = "https://api.us2.sumologic.com/api"
        "#;
        let parsed: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            parsed.default_endpoint.as_deref(),
            Some("https://api.eu.sumologic.com/api")
        );
        assert_eq!(parsed.default_project.as_deref(), Some("prod"));
        assert_eq!(parsed.projects.len(), 2);
        assert_eq!(
            parsed.projects["staging"].endpoint.as_deref(),
            Some("https://api.us2.sumologic.com/api")
        );
    }

    #[test]
    fn round_trips_minimal_config() {
        let mut cfg = FileConfig::default();
        cfg.default_project = Some("default".to_string());
        cfg.projects.insert(
            "default".to_string(),
            Project {
                access_id: Some("id".to_string()),
                access_key: Some("key".to_string()),
                endpoint: Some("https://api.sumologic.com/api".to_string()),
            },
        );
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let reparsed: FileConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed.projects["default"].access_id.as_deref(), Some("id"));
    }
}
