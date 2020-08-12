#![feature(proc_macro_hygiene, decl_macro)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use badge::{Badge, BadgeOptions};
use mongodb::sync::Database;
use mongodb::{
    bson::{doc, Bson},
    sync::Client,
};
use rocket::http::{ContentType, Status};
use rocket::response::NamedFile;
use rocket::{Response, Rocket, State};
use rocket_contrib::json::Json;
use serde::Deserialize;
use std::io::{Cursor};

#[macro_use]
extern crate rocket;

pub struct Config {
    command: String,
    url: Option<String>,
    project: Option<String>,
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
    archive: Option<ArchiveConfig>,
    postarchive: Option<PostArchiveConfig>,
}

#[derive(Debug, Deserialize)]
struct BuildConfig {
    commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ArchiveConfig {
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PostArchiveConfig {
    commands: Vec<String>
}

#[derive(Debug, Deserialize)]
pub struct DrovahConfig {
    mongo: MongoConfig,
}

#[derive(Debug, Deserialize)]
struct MongoConfig {
    mongo_connection_string: String,
    mongo_db: String,
}

#[derive(Debug, PartialEq)]
pub struct SignedPayload(pub String);

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
    }

    Ok(())
}

fn run_build(project: String, database: State<Database>) -> Result<(), Box<dyn Error>> {
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

            if let Some(files) = ci_config.archive {
                if archive_files(files.files, &project) {
                    println!("Successfully archived files for '{}'", project);

                    if let Some(post_archive) = ci_config.postarchive {
                        if run_commands(post_archive.commands, &project) {
                            println!("Successfully ran post-archive commands for '{}'", project);
                        } else {
                            println!("Error occurred running post-archive commands for '{}'", ".");
                        }
                    }
                }
            }

            save_project_build_status(project, "passing".to_owned(), database);
        } else {
            println!("'{}' has failed to build.", project);
            save_project_build_status(project, "failing".to_owned(), database);
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

fn archive_files(files_to_archive: Vec<String>, project: &str) -> bool {
    let archive_folder = format!("archive/{}", project);
    let archive_path = Path::new(&archive_folder);
    if !archive_path.exists() {
        if let Err(e) = fs::create_dir_all(archive_path) {
            eprintln!("Error creating directories: {}, {}", archive_folder, e);
        }
    }

    let mut commands = vec![];
    for file in files_to_archive {
        let appended_path = format!("{}/{}", project, file);
        let command = format!("cp -R {} {}", appended_path, format!("{}/", archive_folder));
        commands.push(command);
    }

    run_commands(commands, ".")
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
fn github_webhook(webhookdata: Json<WebhookData>, database: State<Database>) -> Status {
    let name = &webhookdata.repository.name;

    let path = Path::new(name);
    if path.exists() {
        // Pull latest changes!
        let commands = vec!["git pull".to_owned()];
        run_commands(commands, name);

        match run_build(name.to_string(), database) {
            Ok(_) => {
                return Status::NoContent;
            }
            Err(e) => {
                eprintln!("Error! {}", e);
            }
        }
    }

    Status::NotAcceptable
}

#[get("/<project>/specific/<file..>")]
fn project_file(project: String, file: PathBuf) -> Option<NamedFile> {
    let path = format!("archive/{}/", project);
    NamedFile::open(Path::new(&path).join(file)).ok()
}

#[get("/<project>/latest")]
fn latest_file(project: String) -> Option<NamedFile> {
    let str_path = format!("archive/{}/", project);
    let path = Path::new(&str_path);

    if let Ok(dir) = fs::read_dir(path) {
        let last = dir.last();

        if let Some(some_last) = last {
            if let Ok(ok_last) = some_last {
                return NamedFile::open(ok_last.path()).ok();
            }
        }
    }

    None
}

#[get("/<project>/statusBadge")]
fn status_badge(project: String, database: State<Database>) -> Response<'static> {
    let status_badge = get_project_status_badge(project, database);

    if !status_badge.is_empty() {
        let response = Response::build()
            .status(Status::Ok)
            .header(ContentType::SVG)
            .sized_body(Cursor::new(status_badge))
            .finalize();

        return response;
    }

    Response::build().status(Status::NotFound).finalize()
}

fn save_project_build_status(project: String, status: String, database: State<Database>) {
    let document = doc! { "project": &project, "buildStatus": &status };

    let collection = database.collection("build_statuses");

    let build_status = get_project_build_status(&project, &database);

    // If it's not already in db - insert, otherwise just update
    if build_status.is_empty() {
        if let Err(e) = collection.insert_one(document, None) {
            eprintln!(
                "Error adding document for project {}, with status of {}, error: {:#?}",
                project, status, e
            );
        }
    } else {
        let filter = doc! { "project": &project };

        if let Err(e) = collection.update_one(filter, document, None) {
            eprintln!(
                "Error updating document for project {}, with status of {}, error: {:#?}",
                project, status, e
            );
        }
    }
}

fn get_project_status_badge(project: String, database: State<Database>) -> String {
    let badge_options: BadgeOptions;

    let build_status = get_project_build_status(&project, &database);
    if build_status.eq("passing") {
        badge_options = BadgeOptions {
            subject: "drovah".to_owned(),
            status: build_status,
            color: "#4c1".to_owned(),
        };
    } else {
        badge_options = BadgeOptions {
            subject: "drovah".to_owned(),
            status: build_status,
            color: "#ed2e25".to_owned(),
        };
    }

    if let Ok(badge) = Badge::new(badge_options) {
        let svg = badge.to_svg();
        return svg;
    }

    "".to_owned()
}

fn get_project_build_status(project: &String, database: &State<Database>) -> String {
    let collection = database.collection("build_statuses");

    if let Ok(cursor) = collection.find(doc! { "project": project }, None) {
        if let Some(doc_result) = cursor.last() {
            if let Ok(document) = doc_result {
                if let Some(status) = document.get("buildStatus").and_then(Bson::as_str) {
                    return status.to_owned();
                }
            }
        }
    }

    "".to_owned()
}

pub fn launch_rocket(drovah_config: DrovahConfig) -> Rocket {
    let client = Client::with_uri_str(&drovah_config.mongo.mongo_connection_string).unwrap();
    let database = client.database(&drovah_config.mongo.mongo_db);

    rocket::ignite().manage(client).manage(database).mount(
        "/",
        routes![github_webhook, project_file, latest_file, status_badge],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::Client;

    #[test]
    fn get_name_from_url_test() {
        let url = "https://github.com/Huskehhh/biomebot-rs";
        let result = get_name_from_url(url).unwrap();
        assert_eq!("biomebot-rs", result);
    }

    #[test]
    fn test_get_status_badge() {
        let conf_str = fs::read_to_string(Path::new("Drovah.toml")).unwrap();
        let drovah_config: DrovahConfig = toml::from_str(&conf_str).unwrap();

        let client: rocket::local::Client = Client::new(launch_rocket(drovah_config)).unwrap();
        let response = client.get("/BiomeChat/statusBadge").dispatch();
        let fail_response = client.get("/asdasdasdasd/statusBadge").dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(fail_response.status(), Status::NotFound);
    }

    #[test]
    fn test_autogen_drovah() {
        let path = Path::new("Drovah.toml");

        let default_file = r#"[mongo]
mongo_connection_string = "mongodb://localhost:27017"
mongo_db = "drovah""#;

        if path.exists() {
            fs::remove_file(path).expect("Error removing file");
        }

        fs::write(path, default_file).expect("Error creating file");

        assert!(path.exists());

        let read_from_file = fs::read_to_string(path).expect("Error reading from file");
        let drovah_config: DrovahConfig =
            toml::from_str(&read_from_file).expect("Error parsing toml");

        assert_eq!(
            drovah_config.mongo.mongo_connection_string,
            "mongodb://localhost:27017"
        );
        assert_ne!(drovah_config.mongo.mongo_connection_string, "");
    }
}