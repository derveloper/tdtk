use std::fs;
use std::path::Path;

use handlebars::Handlebars;
use inquire::Text;
use serde_json::json;
use crate::core::execute_command;
use crate::resources::TPL_README;

pub fn handle_service() {
    let service_name = Text::new("(Alpha) What is the name of the service?")
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

    execute_command(
        format!("cd {service_name} && \
        git init && \
        git add . && \
        git commit -am 'Initial commit'").to_string(), None);
}
