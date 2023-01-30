use std::process::Command;
use derive_more::Display;
use inquire::error::InquireResult;
use inquire::Select;

#[derive(Display)]
#[display(fmt = "{}", prompt)]
pub struct Choice<T> where T: std::fmt::Display {
    pub(crate) choice: T,
    pub(crate) prompt: &'static str,
}

#[derive(Display)]
pub enum Action {
    Generate,
    Import,
}

#[derive(Display)]
pub enum Chores {
    VaultSecret,
    Service,
}

pub fn select<T>(prompt: &str, choices: Vec<Choice<T>>) -> InquireResult<Choice<T>> where T: std::fmt::Display {
    Select::new(prompt, choices).prompt()
}

pub fn execute_command(vault_command: String, password: Option<String>) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(vault_command.clone())
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = match password.clone() {
            Some(password) => stderr.replace(password.as_str(), "*****"),
            None => stderr.to_string(),
        };

        let vault_command = match password {
            Some(password) => vault_command.replace(password.as_str(), "*****"),
            None => vault_command,
        };

        panic!("Failed to execute command: {}\n{}", vault_command, stderr);
    }

    String::from_utf8_lossy(&output.stdout).to_string()
}