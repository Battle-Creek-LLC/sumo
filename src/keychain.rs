//! OS keychain access for Sumo Logic API credentials.
//!
//! Backed by the `keyring` crate so the same code works against macOS
//! Keychain, Linux Secret Service / kernel keyutils, and Windows
//! Credential Manager. Service names are kept compatible with the
//! original `security-framework`-based layout so existing macOS
//! entries continue to resolve.

const ACCOUNT: &str = "sumo-cli";
const ACTIVE_PROJECT_SERVICE: &str = "com.sumologic.cli.active-project";
const PROJECT_REGISTRY_SERVICE: &str = "com.sumologic.cli.projects";

fn service_name(project: &str, key: &str) -> String {
    format!("com.sumologic.cli.{project}.{key}")
}

fn entry(service: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, ACCOUNT)
        .map_err(|e| format!("Unable to access OS keychain. ({e})"))
}

fn get_password(service: &str) -> Result<Option<String>, String> {
    match entry(service)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Unable to access OS keychain. ({e})")),
    }
}

fn set_password(service: &str, value: &str) -> Result<(), String> {
    entry(service)?
        .set_password(value)
        .map_err(|e| format!("Unable to access OS keychain. ({e})"))
}

fn delete_password(service: &str) -> Result<(), String> {
    match entry(service)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Unable to access OS keychain. ({e})")),
    }
}

pub fn set(project: &str, key: &str, value: &str) -> Result<(), String> {
    set_password(&service_name(project, key), value)
}

pub fn get(project: &str, key: &str) -> Result<Option<String>, String> {
    get_password(&service_name(project, key))
}

pub fn delete(project: &str, key: &str) -> Result<(), String> {
    delete_password(&service_name(project, key))
}

pub fn get_active_project() -> Result<String, String> {
    Ok(get_password(ACTIVE_PROJECT_SERVICE)?.unwrap_or_else(|| "default".to_string()))
}

pub fn set_active_project(name: &str) -> Result<(), String> {
    set_password(ACTIVE_PROJECT_SERVICE, name)
}

/// List all projects from the registry stored as a newline-separated list.
pub fn list_projects() -> Result<Vec<String>, String> {
    match get_password(PROJECT_REGISTRY_SERVICE)? {
        Some(data) => Ok(data.lines().filter(|l| !l.is_empty()).map(String::from).collect()),
        None => Ok(vec![]),
    }
}

pub fn add_project_to_registry(name: &str) -> Result<(), String> {
    let mut projects = list_projects()?;
    if !projects.contains(&name.to_string()) {
        projects.push(name.to_string());
    }
    save_project_registry(&projects)
}

pub fn remove_project_from_registry(name: &str) -> Result<(), String> {
    let mut projects = list_projects()?;
    projects.retain(|p| p != name);
    save_project_registry(&projects)
}

fn save_project_registry(projects: &[String]) -> Result<(), String> {
    if projects.is_empty() {
        delete_password(PROJECT_REGISTRY_SERVICE)
    } else {
        set_password(PROJECT_REGISTRY_SERVICE, &projects.join("\n"))
    }
}
