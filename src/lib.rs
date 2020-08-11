#![feature(proc_macro_hygiene, decl_macro)]

use std::error::Error;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::{fs, thread};

use rocket::response::status;
use rocket::response::status::NoContent;
use rocket_contrib::json::Json;
use serde::Deserialize;

#[macro_use]
extern crate rocket;

pub struct Config {
    command: String,
    url: Option<String>,
    project: Option<String>,
}

impl Config {
    pub fn new(args: Vec<&str>) -> Result<Config, &'static str> {
        if args.len() == 0 {
            return Err("Not enough arguments!");
        }

        let command = args[0].to_string();

        return if command.eq_ignore_ascii_case("new") {
            let url = Some(args[1].to_string());

            Ok(Config {
                command,
                url,
                project: None,
            })
        } else if command.eq_ignore_ascii_case("delete")
            || command.eq_ignore_ascii_case("remove")
            || command.eq_ignore_ascii_case("build")
        {
            let project = Some(args[1].to_string());

            Ok(Config {
                command,
                url: None,
                project,
            })
        } else {
            Err("Incorrect argument!")
        };
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    if config.command.eq_ignore_ascii_case("new") {
        if let Some(url) = config.url {
            if let Some(name) = get_name_from_url(&url) {
                let output = clone(url).wait_with_output()?;
                println!("{:?}", output);

                if output.status.success() {
                    println!("Added new project '{}'!", name);
                } else {
                    println!("Failed to add new project '{}'!", name);
                }
            }
        }
    } else if config.command.eq_ignore_ascii_case("remove")
        || config.command.eq_ignore_ascii_case("delete")
    {
        if let Some(project) = config.project {
            let path = Path::new(&project);

            if path.exists() && path.is_dir() {
                if delete(path) {
                    println!("Success! '{}' has been removed.", project);
                } else {
                    println!("'{}' failed to delete.", project);
                }
            }
        }
    } else if config.command.eq_ignore_ascii_case("build") {
        if let Some(project) = config.project {
            run_build(project)?;
        }
    }

    Ok(())
}

fn run_build(project: String) -> Result<(), Box<dyn Error>> {
    println!("Building '{}'", project);
    let path = Path::new(&project);

    if path.exists() && path.is_dir() {
        let mut settings_file_path = String::from(&project);
        settings_file_path.push_str("/.drovah");

        let ci_settings_file = Path::new(&settings_file_path);
        let settings_string = fs::read_to_string(ci_settings_file)?;
        let ci_config: CIConfig = toml::from_str(&settings_string)?;

        if run_commands(ci_config.build.commands, &project) {
            println!("Success! '{}' has been built.", project);
        } else {
            println!("'{}' has failed to build.", project);
        }
    }
    Ok(())
}

fn run_commands(commands: Vec<String>, directory: &str) -> bool {
    for command in commands {
        let split: Vec<&str> = command.split(" ").collect();

        let program = split
            .first()
            .expect("Error, commands in .drovah formatted wrong!");

        let process = Command::new(program)
            .current_dir(directory)
            .args(&split[1..])
            .stdout(Stdio::piped())
            .spawn()
            .expect("run_commands failed, are they formatted correctly? is the program installed?");

        let result = process
            .wait_with_output()
            .expect("Unexpectedly died on commands!");

        return result.status.success();
    }

    false
}

fn clone(url: String) -> Child {
    Command::new("git")
        .arg("clone")
        .arg(url)
        .stdout(Stdio::piped())
        .spawn()
        .expect("'git clone' command failed to start - is git installed?")
}

fn delete(path: &Path) -> bool {
    if let Err(e) = fs::remove_dir_all(path) {
        eprintln!("Error deleting path '{}'", e);
        return false;
    }
    true
}

fn get_name_from_url(url: &str) -> Option<String> {
    let split = url.split("/");

    if let Some(name) = split.last() {
        return Some(name.to_string());
    }

    None
}

#[post("/webhook", format = "application/json", data = "<webhookdata>")]
fn github_webhook(webhookdata: Json<WebhookData>) -> NoContent {
    thread::spawn(move || {
        let name = &webhookdata.repository.name;

        let path = Path::new(name);
        if path.exists() {
            // Pull latest changes!
            let commands = vec!["git pull".to_string()];
            run_commands(commands, name);

            // Then run build and only tell us if we hit errors!
            if let Err(e) = run_build(name.to_string()) {
                eprintln!("Error! {}", e);
            }
        }
    });

    status::NoContent
}

pub fn launch_rocket() {
    rocket::ignite()
        .mount("/", routes![github_webhook])
        .launch();
}

#[derive(Debug, Deserialize)]
struct WebhookData {
    repository: RepositoryData,
}

#[derive(Debug, Deserialize)]
struct RepositoryData {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CIConfig {
    build: BuildConfig,
}

#[derive(Debug, Deserialize)]
struct BuildConfig {
    commands: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_name_from_url_test() {
        let url = "https://github.com/Huskehhh/biomebot-rs";
        let result = get_name_from_url(url).unwrap();
        assert_eq!("biomebot-rs", result);
    }

    #[test]
    fn toml_test() {
        let toml_str = r#"[build]
commands = ["cargo test", "cargo build"]
    "#;

        let decoded: CIConfig = toml::from_str(toml_str).unwrap();

        println!("{:#?}", decoded.build.commands);

        assert!(true);
    }

    #[test]
    fn json_parse_test() {
        let path = "example-payload.json";
        let string = fs::read_to_string(Path::new(path)).unwrap();

        let decoded: WebhookData = serde_json::from_str(&string).unwrap();

        println!("{:#?}", decoded.repository.name);

        assert_eq!("FakeBlock", decoded.repository.name);
    }
}
