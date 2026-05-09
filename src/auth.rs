use crate::config::{self, FileConfig, Overrides, Project, ResolvedConfig};
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
    let resolved = config::resolve(Overrides {
        project: project_flag.map(String::from),
        ..Overrides::default()
    })?;
    Ok(Credentials {
        access_id: resolved.access_id,
        access_key: resolved.access_key,
        endpoint: resolved.endpoint,
    })
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

    let path = config::config_path(None)?;
    let mut file = config::load_or_default(&path)?;
    file.projects.insert(
        project.to_string(),
        Project {
            access_id: Some(id),
            access_key: Some(key),
            endpoint: Some(ep),
        },
    );
    if file.default_project.is_none() {
        file.default_project = Some(project.to_string());
    }
    config::save_file(&path, &file)?;

    eprintln!(
        "Credentials saved to {} (project: {project}).",
        path.display()
    );
    Ok(())
}

pub fn logout(project: &str, all: bool) -> Result<(), String> {
    let path = config::config_path(None)?;
    let mut file = config::load_or_default(&path)?;

    if all {
        file.projects.clear();
        file.default_project = None;
        config::save_file(&path, &file)?;
        eprintln!("All credentials removed from {}.", path.display());
        return Ok(());
    }

    if file.projects.remove(project).is_none() {
        return Err(format!(
            "Project '{project}' not found in {}.",
            path.display()
        ));
    }
    if file.default_project.as_deref() == Some(project) {
        file.default_project = file.projects.keys().next().cloned();
    }
    config::save_file(&path, &file)?;
    eprintln!(
        "Credentials removed from {} (project: {project}).",
        path.display()
    );
    Ok(())
}

pub fn use_project(name: &str) -> Result<(), String> {
    let path = config::config_path(None)?;
    let mut file = config::load_or_default(&path)?;
    if !file.projects.contains_key(name) {
        return Err(format!(
            "Project '{name}' not found. Run 'sumo auth list' to see available projects."
        ));
    }
    file.default_project = Some(name.to_string());
    config::save_file(&path, &file)?;
    eprintln!("Switched to project: {name}");
    Ok(())
}

pub fn list() -> Result<(), String> {
    let path = config::config_path(None)?;
    let file = config::load_or_default(&path)?;

    if file.projects.is_empty() {
        eprintln!("No projects configured. Run 'sumo auth login' to get started.");
        return Ok(());
    }

    let active = active_project(&file);

    for (name, project) in &file.projects {
        let marker = if Some(name.as_str()) == active.as_deref() {
            "*"
        } else {
            " "
        };
        let endpoint = project.endpoint.clone().unwrap_or_default();
        let deployment = deployment_label(&endpoint);
        println!("{marker} {:<12} {endpoint} ({deployment})", name);
    }
    Ok(())
}

pub fn status() -> Result<(), String> {
    let resolved: ResolvedConfig = config::resolve(Overrides::default())?;
    let deployment = deployment_label(&resolved.endpoint);
    let masked_id = if resolved.access_id.len() > 8 {
        format!("{}***", &resolved.access_id[..8])
    } else {
        resolved.access_id
    };
    println!("Project:    {}", resolved.project);
    println!("Endpoint:   {} ({deployment})", resolved.endpoint);
    println!("Access ID:  {masked_id}");
    println!("Access Key: ****");
    Ok(())
}

fn active_project(file: &FileConfig) -> Option<String> {
    file.default_project
        .clone()
        .or_else(|| file.projects.keys().next().cloned())
}

fn deployment_label(endpoint: &str) -> &'static str {
    for (name, url) in DEPLOYMENTS {
        if endpoint == *url {
            return name;
        }
    }
    "Custom"
}
