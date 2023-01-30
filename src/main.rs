mod vault;
mod core;
mod completer;

use crate::core::{Choice, prompt};
use crate::core::Chores::{Service, VaultSecret};
use crate::vault::handle_vault_secret;

fn main() {
    match prompt("Which chores do you need to do?", vec![
        Choice {choice: VaultSecret, prompt: "New ansible vault secret (password, token, key, ...)"},
        Choice {choice: Service, prompt: "Create a new service"},
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

fn handle_service() {
    println!("Handling a service");
}
