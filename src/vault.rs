use std::{fs, io, str};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use ansible_vault::decrypt_vault_from_file;
use anyhow::{Context, Result};
use base64::Engine;
use inquire::{Password, PasswordDisplayMode, Text};
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use regex::Regex;

use crate::completer::FilePathCompleter;
use crate::core::{Choice, execute_command, select};
use crate::core::Action::{Generate, Import};

pub fn handle_vault_secret() -> Result<()> {
    let vault_password = if std::env::var("ANSIBLE_VAULT_PASSWORD_FILE").is_err() {
        Password::new("Ansible vault password:")
            .with_display_mode(PasswordDisplayMode::Masked)
            .without_confirmation()
            .prompt()
            .with_context(|| "Failed to get vault password")?
    } else {
        println!("Using ANSIBLE_VAULT_PASSWORD_FILE environment variable");
        get_vault_password()?
    };

    let min_length = 4;
    if vault_password.len() < min_length {
        println!("The vault password must be at least {min_length} characters long");
        handle_vault_secret()?
    } else {
        match select("Do you want to generate a new secret?", vec![
            Choice { choice: Generate, prompt: "Generate a new secret".to_string() },
            Choice { choice: Import, prompt: "Import a secret".to_string() },
        ]) {
            Ok(choice) => {
                match choice.choice {
                    Generate => handle_vault_secret_generate(&vault_password)?,
                    Import => handle_vault_secret_import(&vault_password)?,
                }
            }
            Err(_) => println!("There was an error, please try again"),
        };
    }

    Ok(())
}

fn prompt_secret_name() -> Result<String> {
    let name = Text::new("What is the name of the secret?").prompt();

    name.map(|name| {
        let re = Regex::new(r"[^A-Za-z0-9]").unwrap();
        let name = re.replace_all(&name.trim().to_string(), "_").to_string();
        if name.starts_with("vault_") { name } else { format!("vault_{}", name) }
    }).with_context(|| "Failed to get secret name")
}

fn handle_vault_secret_import(vault_password: &String) -> Result<()> {
    let secret_name = prompt_secret_name()?;
    let secret = Password::new("The secret text")
        .with_display_mode(PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()
        .with_context(|| "Failed to get secret text")?;

    add_vault_secret(vault_password, &secret_name, Some(&secret))
}

fn handle_vault_secret_generate(vault_password: &String) -> Result<()> {
    let secret_name = prompt_secret_name()?;
    add_vault_secret(vault_password, &secret_name, None)
}

fn add_vault_secret(vault_password: &String, secret_name: &String, secret: Option<&String>) -> Result<()> {
    let vault_file_path = prompt_vault_file_path()?;
    let path = Path::new(vault_file_path.as_str());

    if !path.exists() {
        io::stdout().flush().unwrap();
        create_vault_file(vault_file_path.as_str(), vault_password)?;
    }

    if !path.is_file() {
        add_vault_secret(
            vault_password,
            secret_name,
            secret,
        )?;
    } else {
        let absolute_vault_file_path = fs::canonicalize(vault_file_path)
            .with_context(|| "Failed to get absolute vault file path")?;
        let absolute_vault_file_path = absolute_vault_file_path.to_str().unwrap();

        let new_secret = generate_secret();
        let secret = match secret {
            Some(secret) => secret,
            None => &new_secret,
        };

        add_vault_secret_to_file(&secret_name, &secret, absolute_vault_file_path, &vault_password)?
    }

    Ok(())
}

fn generate_secret() -> String {
    let mut rng = ChaCha20Rng::from_entropy();
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    let secret = base64::engine::general_purpose::STANDARD.encode(&bytes);
    secret
}

fn prompt_vault_file_path() -> Result<String> {
    let vault_file_path = Text::new("Where is the vault file located? (tab to autocomplete)")
        .with_autocomplete(FilePathCompleter::default())
        .prompt()
        .with_context(|| "Failed to get vault file path")?;
    if vault_file_path.starts_with("./") || vault_file_path.starts_with("/") {
        Ok(vault_file_path)
    } else {
        Ok("./".to_string() + &*vault_file_path)
    }
}

fn add_vault_secret_to_file(secret_name: &String, secret: &String, vault_file_path: &str, password: &String) -> Result<()> {
    let mut vault_file = decrypt_vault_file(vault_file_path, password)?;
    vault_file.insert(secret_name.clone(), secret.clone());

    let vault_file_string = serde_yaml::to_string(&vault_file).unwrap();
    let encrypted = ansible_vault::encrypt_vault(vault_file_string.as_bytes(), password.as_str())
        .with_context(|| "Failed to encrypt vault")?;

    fs::write(vault_file_path, encrypted).with_context(|| "Failed to write vault file")
}

fn decrypt_vault_file(file: &str, password: &String) -> Result<BTreeMap<String, String>> {
    let decrypted = decrypt_vault_from_file(file, password.as_str())
        .with_context(|| "Failed to decrypt vault file")?;
    serde_yaml::from_str(str::from_utf8(&decrypted).with_context(|| "UTF-8 content expected")?)
        .with_context(|| "Failed to parse decrypted vault file")
}

fn create_vault_file(file_path: &str, password: &String) -> Result<usize> {
    let mut file = File::create(file_path).with_context(|| "Failed to create vault file")?;
    let vault = ansible_vault::encrypt_vault("---".as_bytes(), password.as_str())
        .with_context(|| "Failed to encrypt vault")?;
    file.write(vault.as_bytes()).with_context(|| "Failed to write vault file")
}

fn get_vault_password() -> Result<String> {
    let vault_password_file = std::env::var("ANSIBLE_VAULT_PASSWORD_FILE")
        .with_context(|| "ANSIBLE_VAULT_PASSWORD_FILE is not set")?;
    execute_command(vault_password_file.as_str(), &[], None)
        .map(|stdout| stdout.trim().to_string())
}
