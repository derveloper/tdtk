use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use derive_more::Display;
use oauth2::basic::BasicTokenResponse;
use oauth2::TokenResponse;
use octocrab::models::User;
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

use crate::core::{Choice, execute_command, select, text};
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
    let service_name = text("What is the name of the service?")?;

    let service_name = service_name.trim().to_string();
    let service_name = service_name.replace(" ", "-");
    if service_name.trim().is_empty() {
        println!("Service name cannot be empty");
        return Ok(());
    }

    if Path::new(service_name.as_str()).exists() {
        let ans = select(
            "Directory exists, do you want to delete it?",
            vec!["No", "Yes"],
        )?;
        if ans == "No" {
            print!("Nothing to do, exiting");
            return Ok(());
        }

        fs::remove_dir_all(&service_name).context("Failed to remove service directory")?;
    }

    let service_description = text("What is the description of the service?")?;

    let answers = custom_questions(spec_questions_path)?;

    let token_response = get_github_token().await?;

    let octocrab = make_github_client(token_response)?;

    let user = octocrab.current().user().await.context("Failed to get user")?;
    let (repo_owner, repo_name) = split_repo_name(&service_name, &user)?;

    let repo = octocrab
        .repos(&repo_owner, &repo_name)
        .get()
        .await;

    if repo.is_ok() {
        let ans = select("Repo exists, do you want to delete it?", vec!["No", "Yes"])?;
        if ans == "No" {
            print!("Nothing to do, exiting");
            return Ok(());
        }

        delete_repo(&octocrab, &repo_owner, &repo_name).await?;
    }

    let (template_owner, template_name) = split_repo_name(repo_template, &user)?;

    create_repo(service_description, octocrab, &repo_owner, &repo_name, template_owner, template_name).await?;

    execute_command("git", &["clone", &format!("git@github.com:{repo_owner}/{repo_name}")], None)?;
    execute_command("git", &["pull"], Some(&repo_name))?;

    if !answers.is_empty() {
        add_service_specs(answers, repo_name)?;
    }

    Ok(())
}

fn make_github_client(token_response: BasicTokenResponse) -> Result<Octocrab> {
    Octocrab::builder()
        .personal_token(token_response.access_token().secret().to_string())
        .build()
        .context("Failed to build octocrab")
}

async fn create_repo(
    service_description: String,
    octocrab: Octocrab,
    repo_owner: &String,
    repo_name: &String,
    template_owner: String,
    template_name: String,
) -> Result<()> {
    octocrab
        .repos(template_owner, template_name)
        .generate(repo_name.as_str())
        .owner(repo_owner.as_str())
        .description(service_description.as_str())
        .private(true)
        .send()
        .await
        .context("Failed to create repo")
}

async fn delete_repo(
    octocrab: &Octocrab,
    repo_owner: &String,
    repo_name: &String,
) -> Result<()> {
    octocrab
        .repos(repo_owner.as_str(), repo_name.as_str())
        .delete()
        .await
        .context("Failed to delete repo")
}

fn add_service_specs(
    answers: Vec<Answer>,
    repo_name: String,
) -> Result<()> {
    let spec_filename = ".service-specs.yaml";
    let spec_path = format!("{}/{}", repo_name, spec_filename);

    File::create(&spec_path)
        .context(format!("Failed to create {spec_filename}"))?;

    let specs_file = fs::read_to_string(spec_path.as_str())
        .context(format!("Failed to read {spec_filename}"))?;

    let mut service_specs: BTreeMap<String, String> = serde_yaml::from_str(specs_file.as_str())
        .context(format!("Failed to parse {spec_filename}"))?;

    for answer in answers {
        service_specs.insert(answer.name, answer.value);
    }

    let service_specs = serde_yaml::to_string(&service_specs)
        .context(format!("Failed to serialize {spec_filename}"))?;

    fs::write(spec_path.as_str(), service_specs.as_bytes())
        .context(format!("Failed to write {spec_filename}"))?;

    let repo_name = Some(&repo_name);
    execute_command("git", &["add", ".service-specs.yaml"], repo_name)?;
    execute_command("git", &["commit", "-m", "add service-specs.yaml"], repo_name)?;
    execute_command("git", &["push", "origin", "main"], repo_name)?;

    Ok(())
}

macro_rules! check_condition {
    ($condition:expr, $answers:expr) => {
        let ans = $answers.iter().find(|a| a.name == $condition.question);
        if ans.is_none() {
            continue;
        }

        let ans = ans.unwrap();
        if !$condition.values.contains(&ans.value) {
            continue;
        }
    };
}

fn custom_questions(spec_questions_path: Option<&String>) -> Result<Vec<Answer>> {
    let mut answers: Vec<Answer> = Vec::new();

    if let Some(spec_questions_path) = spec_questions_path {
        let questions = fs::read_to_string(spec_questions_path)
            .context(format!("Failed to read {spec_questions_path:?}"))?;
        let questions: Root = serde_yaml::from_str(questions.as_str())
            .context(format!("Failed to parse {spec_questions_path:?}"))?;

        for question in &questions.questions {
            if let Some(condition) = &question.condition {
                check_condition!(condition, answers);
            }
            if let Some(options) = &question.options {
                answers.push(get_answer(&question.name, &question.question, options)?);
            } else {
                let ans = text(&question.question)?;
                answers.push(Answer {
                    name: question.clone().name,
                    value: ans,
                });
            }
        }
    }

    Ok(answers)
}

fn get_answer(name: &String, question: &String, options: &Vec<OptionElement>) -> Result<Answer> {
    let options: Vec<Choice<String>> = options.iter()
        .map(|o| Choice { prompt: o.display.clone(), choice: o.value.clone() })
        .collect();

    let ans = select(question, options)?;
    let answer = Answer {
        name: name.clone(),
        value: ans.choice,
    };

    Ok(answer)
}

fn split_repo_name(service_name: &String, user: &User) -> Result<(String, String)> {
    let repo_owner;
    let repo_name;

    if service_name.contains("/") {
        let mut parts = service_name.split("/");
        repo_owner = parts.next().unwrap().to_string();
        repo_name = parts.next().unwrap().to_string();
    } else {
        repo_owner = user.clone().login;
        repo_name = service_name.clone();
    }

    Ok((repo_owner, repo_name))
}
