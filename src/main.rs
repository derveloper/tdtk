mod vault;
mod core;
mod completer;
mod service;
mod resources;

use crate::core::{Choice, select};
use crate::core::Chores::{Service, VaultSecret};
use crate::service::handle_service;
use crate::vault::handle_vault_secret;

fn main() {
    match select("Which chores do you need to do?", vec![
        Choice {choice: VaultSecret, prompt: "New ansible vault secret (password, token, key, ...)"},
        Choice {choice: Service, prompt: "Create a new service.rs"},
    ]) {
        Ok(choice) => {
            match choice.choice {
                VaultSecret => handle_vault_secret(),
                Service => handle_service(),
            }
        },
        Err(_) => println!("There was an error, please try again"),
    };
}
