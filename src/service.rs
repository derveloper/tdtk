use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use inquire::{Select, Text};
use oauth2::TokenResponse;
use octocrab::models::User;

use crate::core::execute_command;
use crate::github::get_github_token;

pub async fn handle_service(repo_template: String) -> Result<()> {
    let service_name = Text::new("(Alpha) What is the name of the service?")
        .prompt()
        .with_context(|| "Failed to get service name")?;

    let service_name = service_name.trim().to_string();
    let service_name = service_name.replace(" ", "-");
    if service_name.trim().is_empty() {
        println!("Service name cannot be empty");
        return Ok(());
    }

    if Path::new(service_name.clone().as_str()).exists() {
        let ans = Select::new(
            "Directory exists, do you want to delete it?",
            vec!["No", "Yes"],
        )
            .prompt()
            .with_context(|| "Failed to get input")?;
        if ans == "No" {
            print!("Nothing to do, exiting");
            return Ok(());
        }

        fs::remove_dir_all(service_name.clone()).with_context(|| "Failed to remove service directory")?;
    }

    let service_description = Text::new("What is the description of the service?")
        .prompt()
        .with_context(|| "Failed to get service description")?;

    let token_response = get_github_token().await?;

    let octocrab = octocrab::Octocrab::builder()
        .personal_token(token_response.access_token().secret().to_string())
        .build()
        .with_context(|| "Failed to build octocrab")?;

    let user = octocrab.current().user().await.with_context(|| "Failed to get user")?;
    let (repo_owner, repo_name) = split_repo_name(service_name, user.clone());

    let repo = octocrab
        .repos(repo_owner.clone(), repo_name.clone())
        .get()
        .await;
    if repo.is_ok() {
        let ans = Select::new("Repo exists, do you want to delete it?", vec!["No", "Yes"])
            .prompt()
            .with_context(|| "Failed to get input")?;
        if ans == "No" {
            print!("Nothing to do, exiting");
            return Ok(());
        }

        octocrab
            .repos(repo_owner.as_str(), repo_name.as_str())
            .delete()
            .await
            .with_context(|| "Failed to delete repo")?;
    }

    let (template_owner, template_name) = split_repo_name(repo_template, user);

    octocrab
        .repos(template_owner, template_name)
        .generate(repo_name.as_str())
        .owner(repo_owner.as_str())
        .description(service_description.as_str())
        .private(true)
        .send()
        .await
        .with_context(|| "Failed to create repo")?;

    execute_command("git", &["clone", &format!("git@github.com:{repo_owner}/{repo_name}")])?;

    Ok(())
}

fn split_repo_name(service_name: String, user: User) -> (String, String) {
    let repo_owner;
    let repo_name;

    if service_name.contains("/") {
        let mut parts = service_name.split("/");
        repo_owner = parts.next().unwrap().to_string();
        repo_name = parts.next().unwrap().to_string();
    } else {
        repo_owner = user.login;
        repo_name = service_name;
    }

    (repo_owner, repo_name)
}
