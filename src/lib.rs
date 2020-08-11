use std::error::Error;
use std::process::{Command, Stdio, Child};
use std::path::Path;
use std::fs;

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

            Ok(Config { command, url, project: None })
        } else if command.eq_ignore_ascii_case("delete") {
            let project = Some(args[2].clone());

            Ok(Config { command, url: None, project })
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

            if path.exists() {
                delete(path);
            }
        }
    }

    Ok(())
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
    fn new_test() {
        let config = Config {
            command: "new".to_string(),
            url: Some("https://github.com/Huskehhh/biomebot-rs".to_string()),
            project: None,
        };

        let result = run(config);

        assert_eq!(result.unwrap(), ());

        delete(Path::new("biomebot-rs"));
    }

    #[test]
    fn remove_test() {
        fs::create_dir(Path::new("biomebot-rs")).expect("Error creating dir");

        let config = Config {
            command: "remove".to_string(),
            url: None,
            project: Some("biomebot-rs".to_string()),
        };

        let result = run(config);

        assert_eq!(result.unwrap(), ());
    }
}