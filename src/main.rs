use std::fs;

use clap::{arg, command};
use serde::Deserialize;

use crate::core::{Choice, select};
use crate::core::Chores::{Service, VaultSecret};
use crate::service::handle_service;
use crate::vault::handle_vault_secret;

mod vault;
mod core;
mod completer;
mod service;
mod github;

#[derive(Debug, Deserialize)]
struct Config {
    template_repo: Option<String>,
    spec_questions_path: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let home_dir = dirs::home_dir().unwrap();
    let cwd = std::env::current_dir()?;
    let config_path_home = format!("{}/.config/tdtk.toml", home_dir.to_str().unwrap());
    let config_path_cwd = format!("{}/.tdtk.toml", cwd.to_str().unwrap());
    let config: Option<Config> = fs::read_to_string(config_path_cwd)
        .or(fs::read_to_string(config_path_home))
        .map(|toml_str| {
            toml::from_str(toml_str.as_str()).unwrap()
        }).unwrap_or(None);

    let mut template_repo_arg = arg!([template_repo] "The name of the template repo (e.g. 'java-service', 'org/default-service)")
        .short('t')
        .default_value("derveloper/tdtk-template-repo");

    let mut spec_questions_path_arg = arg!([spec_questions_path] "Path to the spec questions file")
        .short('q')
        .required(false);

    if let Some(config) = config {
        if let Some(template_repo) = config.template_repo {
            template_repo_arg = template_repo_arg.default_value(template_repo);
        }

        if let Some(spec_questions_path) = config.spec_questions_path {
            spec_questions_path_arg = spec_questions_path_arg.default_value(spec_questions_path);
        }
    }
    let matches = command!() // requires `cargo` feature
        .after_help("You can also set defaults in ~/.config/tdtk.toml or ./.tdtk.toml")
        .arg(template_repo_arg)
        .arg(spec_questions_path_arg)
        .get_matches();

    match matches.get_one::<String>("template_repo") {
        Some(template_repo) => {
            match select("What do you need to do?", vec![
                Choice { choice: VaultSecret, prompt: "Create a ansible vault secret (password, token, key, ...)".to_string() },
                Choice { choice: Service, prompt: "Create a new service".to_string() },
            ]) {
                Ok(choice) => {
                    match choice.choice {
                        VaultSecret => handle_vault_secret()?,
                        Service => {
                            let spec_questions_path_arg = matches.get_one::<String>("spec_questions_path");
                            handle_service(template_repo, spec_questions_path_arg).await?
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to get input: {}", e);
                    return Ok(());
                }
            }
            Ok(())
        }
        _ => {
            println!("Please provide a template repo name");
            Ok(())
        }
    }
}
