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