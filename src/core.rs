use std::fmt;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use derive_more::Display;
use inquire::{Select, Text};

#[derive(Display)]
#[display(fmt = "{}", prompt)]
pub struct Choice<T> where T: fmt::Display {
    pub(crate) choice: T,
    pub(crate) prompt: String,
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

pub fn select<T>(prompt: &str, choices: Vec<T>) -> Result<T> where T: fmt::Display {
    Select::new(prompt, choices).prompt().context(format!("Failed to select `{}`", prompt))
}

pub fn text<T>(prompt: T) -> Result<String> where T: Into<String> {
    Text::new(prompt.into().as_str())
        .prompt()
        .context("Failed to get input")
}

pub fn execute_command(command: &str, args: &[&str], wd: Option<&String>) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(wd.unwrap_or(&".".to_string()))
        .output()
        .context(format!("Failed to execute process `{}`", command))?;

    println!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to execute command `{} {}`, stderr:\n{}",
                                    command, args.join(" "), stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}