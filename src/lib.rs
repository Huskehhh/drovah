use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use serde::Deserialize;

pub struct Config {
    command: String,
    url: Option<String>,
    project: Option<String>,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() == 0 {
            return Err("Not enough arguments!");
        }

        let command = args[1].clone();

        return if command.eq_ignore_ascii_case("new") {
            let url = Some(args[2].clone());

            Ok(Config {
                command,
                url,
                project: None,
            })
        } else if command.eq_ignore_ascii_case("delete") || command.eq_ignore_ascii_case("build") {
            let project = Some(args[2].clone());

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
                println!("Creating new project '{}'!", name);
                println!("{:?}", output);
            }
        }
    } else if config.command.eq_ignore_ascii_case("remove") {
        if let Some(project) = config.project {
            let path = Path::new(&project);

            if path.exists() && path.is_dir() {
                delete(path);
            }
        }
    } else if config.command.eq_ignore_ascii_case("build") {
        if let Some(project) = config.project {
            let path = Path::new(&project);

            if path.exists() && path.is_dir() {
                let mut settings_file_path = String::from(&project);
                settings_file_path.push_str("/.drovah");

                let ci_settings_file = Path::new(&settings_file_path);
                let settings_string = fs::read_to_string(ci_settings_file)?;
                let ci_config: CIConfig = toml::from_str(&settings_string)?;

                run_commands(ci_config.build.commands);
            }
        }
    }

    Ok(())
}

fn run_commands(commands: Vec<String>) -> Vec<Child> {
    let mut children = vec![];
    for command in commands {
        let split: Vec<&str> = command.split(" ").collect();

        let program = split
            .first()
            .expect("Error, commands in .drovah formatted wrong!");

        let process = Command::new(program)
            .args(&split[1..])
            .spawn()
            .expect("run_commands failed, are they formatted correctly? is the program installed?");

        children.push(process);
    }

    children
}

fn clone(url: String) -> Child {
    Command::new("git")
        .arg("clone")
        .arg(url)
        .stdout(Stdio::piped())
        .spawn()
        .expect("'git clone' command failed to start - is git installed?")
}

fn delete(path: &Path) {
    if let Err(e) = fs::remove_dir_all(path) {
        eprintln!("Error deleting path {}", e);
    }
}

fn get_name_from_url(url: &str) -> Option<String> {
    let split = url.split("/");

    if let Some(name) = split.last() {
        return Some(name.to_string());
    }

    None
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
}
