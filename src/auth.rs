use crate::keychain;
use dialoguer::{Input, Password, Select};

const DEPLOYMENTS: &[(&str, &str)] = &[
    ("US1", "https://api.sumologic.com/api"),
    ("US2", "https://api.us2.sumologic.com/api"),
    ("EU", "https://api.eu.sumologic.com/api"),
    ("AU", "https://api.au.sumologic.com/api"),
    ("JP", "https://api.jp.sumologic.com/api"),
    ("CA", "https://api.ca.sumologic.com/api"),
    ("IN", "https://api.in.sumologic.com/api"),
];

pub struct Credentials {
    pub access_id: String,
    pub access_key: String,
    pub endpoint: String,
}

pub fn resolve_credentials(project_flag: Option<&str>) -> Result<Credentials, String> {
    // Check env vars first as fallback
    if let (Ok(id), Ok(key), Ok(endpoint)) = (
        std::env::var("SUMO_ACCESS_ID"),
        std::env::var("SUMO_ACCESS_KEY"),
        std::env::var("SUMO_API_ENDPOINT"),
    ) {
        if project_flag.is_none() {
            return Ok(Credentials { access_id: id, access_key: key, endpoint });
        }
    }

    let project = match project_flag {
        Some(p) => p.to_string(),
        None => keychain::get_active_project()?,
    };

    let access_id = keychain::get(&project, "access-id")?;
    let access_key = keychain::get(&project, "access-key")?;
    let endpoint = keychain::get(&project, "endpoint")?;

    match (access_id, access_key, endpoint) {
        (Some(id), Some(key), Some(ep)) => Ok(Credentials {
            access_id: id,
            access_key: key,
            endpoint: ep,
        }),
        _ => {
            if project_flag.is_some() {
                // Check if project exists at all
                let projects = keychain::list_projects()?;
                if !projects.contains(&project) {
                    return Err(format!(
                        "Project '{}' not found. Run 'sumo auth list' to see available projects.",
                        project
                    ));
                }
            }
            Err("Not authenticated. Run 'sumo auth login' to set up credentials.".to_string())
        }
    }
}

pub fn login(
    project: &str,
    endpoint: Option<String>,
    access_id: Option<String>,
    access_key: Option<String>,
) -> Result<(), String> {
    let ep = match endpoint {
        Some(e) => e,
        None => {
            let items: Vec<String> = DEPLOYMENTS
                .iter()
                .map(|(name, url)| format!("{name}  {url}"))
                .collect();

            eprintln!("Sumo Logic API Endpoint");
            let selection = Select::new()
                .items(&items)
                .default(0)
                .interact()
                .map_err(|e| format!("Prompt error: {e}"))?;

            DEPLOYMENTS[selection].1.to_string()
        }
    };

    let id = match access_id {
        Some(i) => i,
        None => Input::new()
            .with_prompt("Access ID")
            .interact_text()
            .map_err(|e| format!("Prompt error: {e}"))?,
    };

    let key = match access_key {
        Some(k) => k,
        None => Password::new()
            .with_prompt("Access Key")
            .interact()
            .map_err(|e| format!("Prompt error: {e}"))?,
    };

    keychain::set(project, "endpoint", &ep)?;
    keychain::set(project, "access-id", &id)?;
    keychain::set(project, "access-key", &key)?;
    keychain::add_project_to_registry(project)?;

    eprintln!("Credentials saved to keychain (project: {project}).");
    Ok(())
}

pub fn logout(project: &str, all: bool) -> Result<(), String> {
    if all {
        let projects = keychain::list_projects()?;
        for p in &projects {
            keychain::delete(p, "access-id")?;
            keychain::delete(p, "access-key")?;
            keychain::delete(p, "endpoint")?;
            keychain::remove_project_from_registry(p)?;
        }
        eprintln!("All credentials removed from keychain.");
    } else {
        keychain::delete(project, "access-id")?;
        keychain::delete(project, "access-key")?;
        keychain::delete(project, "endpoint")?;
        keychain::remove_project_from_registry(project)?;
        eprintln!("Credentials removed from keychain (project: {project}).");
    }
    Ok(())
}

pub fn use_project(name: &str) -> Result<(), String> {
    let projects = keychain::list_projects()?;
    if !projects.contains(&name.to_string()) {
        return Err(format!(
            "Project '{}' not found. Run 'sumo auth list' to see available projects.",
            name
        ));
    }
    keychain::set_active_project(name)?;
    eprintln!("Switched to project: {name}");
    Ok(())
}

pub fn list() -> Result<(), String> {
    let projects = keychain::list_projects()?;
    if projects.is_empty() {
        eprintln!("No projects configured. Run 'sumo auth login' to get started.");
        return Ok(());
    }

    let active = keychain::get_active_project()?;

    for p in &projects {
        let marker = if *p == active { "*" } else { " " };
        let endpoint = keychain::get(p, "endpoint")?.unwrap_or_default();
        let deployment = deployment_label(&endpoint);
        println!("{marker} {:<12} {endpoint} ({deployment})", p);
    }
    Ok(())
}

pub fn status() -> Result<(), String> {
    let active = keychain::get_active_project()?;
    let endpoint = keychain::get(&active, "endpoint")?;
    let access_id = keychain::get(&active, "access-id")?;

    match (endpoint, access_id) {
        (Some(ep), Some(id)) => {
            let deployment = deployment_label(&ep);
            let masked_id = if id.len() > 8 {
                format!("{}***", &id[..8])
            } else {
                id
            };
            println!("Project:    {active}");
            println!("Endpoint:   {ep} ({deployment})");
            println!("Access ID:  {masked_id}");
            println!("Access Key: ****");
            Ok(())
        }
        _ => Err("Not authenticated. Run 'sumo auth login' to set up credentials.".to_string()),
    }
}

fn deployment_label(endpoint: &str) -> &'static str {
    for (name, url) in DEPLOYMENTS {
        if endpoint == *url {
            return name;
        }
    }
    "Custom"
}
