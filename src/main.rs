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
}

#[tokio::main]
async fn main() {
    let home_dir = dirs::home_dir().unwrap();
    let config_path = format!("{}/.config/tdtk.toml", home_dir.to_str().unwrap());
    let config: Option<Config> = fs::read_to_string(config_path).map(|toml_str| {
        toml::from_str(toml_str.as_str()).unwrap()
    }).unwrap_or(None);

    let mut template_repo_arg = arg!([template_repo] "The name of the template repo (e.g. 'java-service', 'org/default-service)")
        .short('t')
        .default_value("tdtk-template-repo");

    if let Some(config) = config {
        if let Some(template_repo) = config.template_repo {
            template_repo_arg = template_repo_arg.default_value(template_repo);
        }
    }
    let matches = command!() // requires `cargo` feature
        .after_help("You can also set defaults in ~/.config/tdtk.toml")
        .arg(template_repo_arg)
        .get_matches();

    match matches.get_one::<String>("template_repo") {
        Some(template_repo) => {
            match select("What do you need to do?", vec![
                Choice { choice: VaultSecret, prompt: "Create a ansible vault secret (password, token, key, ...)" },
                Choice { choice: Service, prompt: "Create a new service" },
            ]) {
                Ok(choice) => {
                    match choice.choice {
                        VaultSecret => match handle_vault_secret() {
                            Ok(_) => {}
                            Err(e) => println!("Failed to handle vault secret {}", e)
                        },
                        Service => {
                            match handle_service(template_repo.to_string()).await {
                                Ok(_) => {}
                                Err(e) => println!("There was an error: {}", e)
                            }
                        }
                    }
                }
                Err(_) => println!("There was an error, please try again"),
            };
        }
        _ => {
            println!("Please provide a template repo name");
        }
    }
}
