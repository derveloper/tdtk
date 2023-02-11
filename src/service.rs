use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use derive_more::Display;
use inquire::{Select, Text};
use oauth2::TokenResponse;
use octocrab::models::User;
use serde::{Deserialize, Serialize};

use crate::core::{Choice, execute_command, select};
use crate::github::get_github_token;

#[derive(Debug, Display, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[display(fmt = "name: {}, question: {}, required: {}, default: {:?}, options: {:?}, condition: {:?}", name, question, required, default, options, condition)]
struct Question {
    name: String,
    question: String,
    required: bool,
    default: Option<String>,
    options: Option<Vec<OptionElement>>,
    condition: Option<Condition>,
}

#[derive(Debug, Display, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[display(fmt = "display: {}, value: {:?}", display, value)]
struct OptionElement {
    display: String,
    value: String,
}

#[derive(Debug, Display, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[display(fmt = "question: {}, values: {:?}", question, values)]
struct Condition {
    question: String,
    values: Vec<String>,
}

#[derive(Debug, Display, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[display(fmt = "questions: {:?}", questions)]
struct Root {
    questions: Vec<Question>,
}

#[derive(Debug, Display, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[display(fmt = "name: {}, value: {}", name, value)]
struct Answer {
    name: String,
    value: String,
}

pub async fn handle_service(repo_template: String) -> Result<()> {
    let service_name = Text::new("What is the name of the service?")
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

    let questions = fs::read_to_string("service-spec.yaml")
        .with_context(|| "Failed to read service-spec.yaml")?;
    let questions: Root = serde_yaml::from_str(questions.as_str())
        .with_context(|| "Failed to parse service-spec.yaml")?;

    let mut answers: Vec<Answer> = Vec::new();
    for question in questions.questions {
        if question.condition.is_some() {
            let condition = question.clone().condition.unwrap();
            let ans = answers.iter().find(|a| a.name == condition.question);
            if ans.is_none() {
                continue;
            }

            let ans = ans.unwrap();
            if !condition.values.contains(&ans.value) {
                continue;
            }
        }
        if question.options.is_some() && question.clone().options.unwrap().len() > 0 {
            let options = question.options.unwrap();
            let options: Vec<Choice<String>> = options.iter()
                .map(|o| Choice { prompt: o.display.clone(), choice: o.value.clone() })
                .collect();
            let ans = select(question.question.as_str(), options)
                .with_context(|| "Failed to get input")?;
            answers.push(Answer {
                name: question.name,
                value: ans.choice,
            });
        } else {
            let ans = Text::new(question.question.as_str())
                .prompt()
                .with_context(|| "Failed to get input")?;
            answers.push(Answer {
                name: question.name,
                value: ans,
            });
        }
    }

    panic!("{:?}", answers);

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
