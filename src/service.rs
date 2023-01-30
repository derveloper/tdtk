use std::fs;
use std::path::Path;

use git2::{Repository};
use handlebars::Handlebars;
use inquire::Text;
use serde_json::json;
use crate::resources::TPL_README;

pub fn handle_service() {
    let service_name = Text::new("What is the name of the service?")
        .prompt()
        .expect("Failed to get service name");

    let service_name = service_name.trim().to_string();
    let service_name = service_name.replace(" ", "-");
    if service_name.trim().is_empty() {
        println!("Service name cannot be empty");
        return;
    }

    if Path::new(service_name.clone().as_str()).exists() {
        println!("Service already exists");
        return;
    }

    let service_description = Text::new("What is the description of the service?")
        .prompt()
        .expect("Failed to get service description");

    fs::create_dir(service_name.clone()).expect("Failed to create service directory");
    let repo = Repository::init(service_name.clone()).expect("Failed to initialize git repository");

    let reg = Handlebars::new();
    reg.render_template_to_write(
        TPL_README,
        &json!({
            "service_name": service_name,
            "service_description": service_description,
        }),
        &mut fs::File::create(format!("{}/README.md", service_name))
            .expect("Failed to create README.md"),
    ).expect("Failed to render README.md");

    let mut index = repo.index()
        .expect("Failed to get git index");
    index
        .add_all(&["*"], git2::IndexAddOption::DEFAULT, None)
        .expect("Failed to add files to git index");

    let oid = index.write_tree().expect("Failed to write git tree");
    let tree = repo.find_tree(oid).expect("Failed to find git tree");
    let signature = &repo.signature().expect("Failed to get git signature");

    repo.commit(
        Some("HEAD"),
        signature,
        signature,
        "Initial commit",
        &tree,
        &[],
    ).expect("Failed to commit files to git repository");
}
