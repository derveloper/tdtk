use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use derive_more::Display;
use inquire::{Select, Text};
use oauth2::basic::BasicTokenResponse;
use oauth2::TokenResponse;
use octocrab::models::User;
use octocrab::Octocrab;
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

pub async fn handle_service(repo_template: &String, spec_questions_path: Option<&String>) -> Result<()> {
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

    let answers = custom_questions(spec_questions_path)?;

    let token_response = get_github_token().await?;

    let octocrab = make_github_client(token_response)?;

    let user = octocrab.current().user().await.with_context(|| "Failed to get user")?;
    let (repo_owner, repo_name) = split_repo_name(&service_name, user.clone());

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

        delete_repo(&octocrab, &repo_owner, &repo_name).await?;
    }

    let (template_owner, template_name) = split_repo_name(repo_template, user);

    create_repo(service_description, octocrab, &repo_owner, &repo_name, template_owner, template_name).await?;

    execute_command("git", &["clone", &format!("git@github.com:{repo_owner}/{repo_name}")], None)?;
    execute_command("git", &["pull"], repo_name.clone().into())?;

    if !answers.is_empty() {
        add_service_specs(answers, repo_name)?;
    }

    Ok(())
}

fn make_github_client(token_response: BasicTokenResponse) -> Result<Octocrab> {
    Octocrab::builder()
        .personal_token(token_response.access_token().secret().to_string())
        .build()
        .with_context(|| "Failed to build octocrab")
}

async fn create_repo(service_description: String, octocrab: Octocrab, repo_owner: &String, repo_name: &String, template_owner: String, template_name: String) -> Result<()> {
    octocrab
        .repos(template_owner, template_name)
        .generate(repo_name.as_str())
        .owner(repo_owner.as_str())
        .description(service_description.as_str())
        .private(true)
        .send()
        .await
        .with_context(|| "Failed to create repo")
}

async fn delete_repo(octocrab: &Octocrab, repo_owner: &String, repo_name: &String) -> Result<()> {
    octocrab
        .repos(repo_owner.as_str(), repo_name.as_str())
        .delete()
        .await
        .with_context(|| "Failed to delete repo")
}

fn add_service_specs(answers: Vec<Answer>, repo_name: String) -> Result<()> {
    let spec_filename = ".service-specs.yaml";
    let spec_path = format!("{}/{}", repo_name.clone(), spec_filename);
    File::create(spec_path.clone())
        .with_context(|| format!("Failed to create {spec_filename}"))?;
    let specs_file = fs::read_to_string(spec_path.as_str())
        .with_context(|| format!("Failed to read {spec_filename}"))?;
    let mut service_specs: BTreeMap<String, String> = serde_yaml::from_str(specs_file.as_str())
        .with_context(|| format!("Failed to parse {spec_filename}"))?;
    for answer in answers {
        service_specs.insert(answer.name, answer.value);
    }
    let service_specs = serde_yaml::to_string(&service_specs)
        .with_context(|| format!("Failed to serialize {spec_filename}"))?;
    fs::write(spec_path.as_str(), service_specs.as_bytes())
        .with_context(|| format!("Failed to write {spec_filename}"))?;

    execute_command("git", &["add", ".service-specs.yaml"], repo_name.clone().into())?;
    execute_command("git", &["commit", "-m", "add service-specs.yaml"], repo_name.clone().into())?;
    execute_command("git", &["push", "origin", "main"], repo_name.into())?;

    Ok(())
}

fn custom_questions(spec_questions_path: Option<&String>) -> Result<Vec<Answer>> {
    let mut answers: Vec<Answer> = Vec::new();

    if spec_questions_path.is_some() && Path::new(spec_questions_path.clone().unwrap().as_str()).exists() {
        let questions = fs::read_to_string(spec_questions_path.clone().unwrap())
            .with_context(|| format!("Failed to read {spec_questions_path:?}"))?;
        let questions: Root = serde_yaml::from_str(questions.as_str())
            .with_context(|| format!("Failed to parse {spec_questions_path:?}"))?;

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
    }
    Ok(answers)
}

fn split_repo_name(service_name: &String, user: User) -> (String, String) {
    let repo_owner;
    let repo_name;

    if service_name.contains("/") {
        let mut parts = service_name.split("/");
        repo_owner = parts.next().unwrap().to_string();
        repo_name = parts.next().unwrap().to_string();
    } else {
        repo_owner = user.login;
        repo_name = service_name.clone();
    }

    (repo_owner, repo_name)
}
