use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

const ACCOUNT: &str = "sumo-cli";

fn service_name(project: &str, key: &str) -> String {
    format!("com.sumologic.cli.{project}.{key}")
}

pub fn set(project: &str, key: &str, value: &str) -> Result<(), String> {
    let service = service_name(project, key);
    // Delete first to avoid "duplicate item" errors on update
    let _ = delete_generic_password(&service, ACCOUNT);
    set_generic_password(&service, ACCOUNT, value.as_bytes())
        .map_err(|e| format!("Unable to access macOS Keychain. Check system permissions. ({e})"))
}

pub fn get(project: &str, key: &str) -> Result<Option<String>, String> {
    let service = service_name(project, key);
    match get_generic_password(&service, ACCOUNT) {
        Ok(bytes) => {
            let val = String::from_utf8(bytes)
                .map_err(|_| "Invalid UTF-8 in keychain entry".to_string())?;
            Ok(Some(val))
        }
        Err(e) => {
            let code = e.code();
            // errSecItemNotFound = -25300
            if code == -25300 {
                Ok(None)
            } else {
                Err(format!("Unable to access macOS Keychain. Check system permissions. ({e})"))
            }
        }
    }
}

pub fn delete(project: &str, key: &str) -> Result<(), String> {
    let service = service_name(project, key);
    match delete_generic_password(&service, ACCOUNT) {
        Ok(()) => Ok(()),
        Err(e) => {
            let code = e.code();
            if code == -25300 {
                Ok(()) // already gone
            } else {
                Err(format!("Unable to access macOS Keychain. Check system permissions. ({e})"))
            }
        }
    }
}

pub fn get_active_project() -> Result<String, String> {
    let service = "com.sumologic.cli.active-project";
    match get_generic_password(service, ACCOUNT) {
        Ok(bytes) => {
            String::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in keychain entry".to_string())
        }
        Err(e) => {
            let code = e.code();
            if code == -25300 {
                Ok("default".to_string())
            } else {
                Err(format!("Unable to access macOS Keychain. Check system permissions. ({e})"))
            }
        }
    }
}

pub fn set_active_project(name: &str) -> Result<(), String> {
    let service = "com.sumologic.cli.active-project";
    let _ = delete_generic_password(service, ACCOUNT);
    set_generic_password(service, ACCOUNT, name.as_bytes())
        .map_err(|e| format!("Unable to access macOS Keychain. Check system permissions. ({e})"))
}

/// List all projects by scanning known keychain entries.
/// We store a project registry at com.sumologic.cli.projects as a newline-separated list.
pub fn list_projects() -> Result<Vec<String>, String> {
    let service = "com.sumologic.cli.projects";
    match get_generic_password(service, ACCOUNT) {
        Ok(bytes) => {
            let data = String::from_utf8(bytes)
                .map_err(|_| "Invalid UTF-8 in keychain entry".to_string())?;
            Ok(data.lines().filter(|l| !l.is_empty()).map(String::from).collect())
        }
        Err(e) => {
            let code = e.code();
            if code == -25300 {
                Ok(vec![])
            } else {
                Err(format!("Unable to access macOS Keychain. Check system permissions. ({e})"))
            }
        }
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
    let service = "com.sumologic.cli.projects";
    let _ = delete_generic_password(service, ACCOUNT);
    if !projects.is_empty() {
        let data = projects.join("\n");
        set_generic_password(service, ACCOUNT, data.as_bytes())
            .map_err(|e| format!("Unable to access macOS Keychain. Check system permissions. ({e})"))?;
    }
    Ok(())
}
