extern crate actix_files;
extern crate actix_web;
extern crate env_logger;

use serde_json::json;
use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};
use std::{fs, io};

use actix_files::NamedFile;
use actix_web::middleware::Logger;
use actix_web::web::{Data, Json};
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use badge::{Badge, BadgeOptions};
use futures::executor::block_on;
use mongodb::{
    bson::{doc, Bson},
    Client, Database,
};
use serde::{Deserialize};
use tokio::stream::StreamExt;

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
    commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DrovahConfig {
    web: WebServerConfig,
    mongo: MongoConfig,
}

#[derive(Debug, Deserialize)]
struct MongoConfig {
    mongo_connection_string: String,
    mongo_db: String,
}

#[derive(Debug, Deserialize)]
struct WebServerConfig {
    address: String,
}

async fn run_build(project: String, database: &Database) -> Result<(), Box<dyn Error>> {
    println!("Building '{}'", project);
    let project_path = format!("data/projects/{}", project);
    let path = Path::new(&project_path);

    if path.exists() && path.is_dir() {
        let settings_file_path = format!("{}/.drovah", project_path);
        let ci_settings_file = Path::new(&settings_file_path);
        let settings_string = fs::read_to_string(ci_settings_file)?;
        let ci_config: CIConfig = toml::from_str(&settings_string)?;

        if run_commands(ci_config.build.commands, &project_path) {
            println!("Success! '{}' has been built.", project);

            if let Some(files) = ci_config.archive {
                if archive_files(files.files, &project) {
                    println!("Successfully archived files for '{}'", project);

                    if let Some(post_archive) = ci_config.postarchive {
                        if run_commands(post_archive.commands, &project_path) {
                            println!("Successfully ran post-archive commands for '{}'", project);
                        } else {
                            println!("Error occurred running post-archive commands for '{}'", ".");
                        }
                    }
                } else {
                    println!("Failed to archive files for '{}'", project);
                }
            }

            save_project_build_status(project, "passing".to_owned(), database).await;
        } else {
            println!("'{}' has failed to build.", project);
            save_project_build_status(project, "failing".to_owned(), database).await;
        }
    }
    Ok(())
}

fn run_commands(commands: Vec<String>, directory: &str) -> bool {
    for command in commands {
        let split: Vec<&str> = command.split(" ").collect();

        let program = split.first().expect("Error, commands are formatted wrong!");

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
    let archive_folder = format!("data/archive/{}/", project);
    let archive_path = Path::new(&archive_folder);
    if !archive_path.exists() {
        if let Err(e) = fs::create_dir_all(archive_path) {
            eprintln!("Error creating directories: {}, {}", archive_folder, e);
        }
    }

    for file_to_match in files_to_archive {
        let path_to_search = format!("data/projects/{}/{}", project, file_to_match);
        if let Some(matched) = match_filename_to_file(&path_to_search) {
            let matched_file_name = matched.split("/").last().unwrap();
            let to = format!("data/archive/{}/{}", project, matched_file_name);
            return copy(&matched, &to);
        }
    }

    false
}

fn match_filename_to_file(filename: &str) -> Option<String> {
    let path = Path::new(filename);

    // If the path is already a file, no need to process further
    if path.is_file() {
        let path_result = String::from(path.to_str().unwrap());
        return Option::Some(path_result);
    }

    // If not, lets look for it
    let file_to_look_for = filename.split("/").last().unwrap();

    // Find all files starting with
    if let Some(path_parent) = path.parent() {
        if let Ok(paths) = fs::read_dir(path_parent) {
            for files in paths {
                if let Ok(file) = files {
                    if file
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&file_to_look_for)
                    {
                        let path = file.path().to_string_lossy().to_string();
                        return Option::Some(path);
                    }
                }
            }
        }
    }

    None
}

fn copy(from_str: &str, to_str: &str) -> bool {
    let from = Path::new(from_str);
    let to = Path::new(to_str);

    println!("Copying {} -> {}", from_str, to_str);

    if let Err(e) = fs::copy(from, to) {
        eprintln!("Error copying file {} -> {}, {}", from_str, to_str, e);
        return false;
    }

    true
}

async fn github_webhook(webhookdata: Json<WebhookData>, database: Data<Database>) -> HttpResponse {
    let project_path = format!("data/projects/{}/", &webhookdata.repository.name);
    let path = Path::new(&project_path);
    if path.exists() {
        // Pull latest changes!
        let commands = vec!["git pull".to_owned()];
        run_commands(commands, &project_path);

        match run_build(webhookdata.repository.name.clone(), &database).await {
            Ok(_) => {
                return HttpResponse::NoContent().body("Build complete");
            }
            Err(e) => {
                eprintln!("Error! {}", e);
            }
        }
    }

    HttpResponse::NotAcceptable().body("Project doesn't exist")
}

async fn get_specific_file(
    project: web::Path<(String,)>,
    file: web::Path<(String,)>,
) -> actix_web::Result<NamedFile> {
    let path_str = format!("data/archive/{}/", project.into_inner().0);
    let path = Path::new(&path_str);
    let file_path = path.join(file.into_inner().0);
    actix_web::Result::Ok(NamedFile::open(file_path)?)
}

async fn get_latest_file(project: web::Path<(String,)>) -> actix_web::Result<NamedFile> {
    let path_str = format!("data/archive/{}/", project.into_inner().0);
    let path = Path::new(&path_str);

    let file = NamedFile::open(fs::read_dir(path)?.last().unwrap()?.path())?;

    actix_web::Result::Ok(file)
}

async fn get_project_information() -> actix_web::Result<HttpResponse> {
    let dir = Path::new("data/projects/");

    let mut projects = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(file_name) = entry.file_name().into_string() {
                projects.push(file_name);
            }
        }
    }

    let json_result = json!({ "projects": projects });

    actix_web::Result::Ok(HttpResponse::Ok().json(json_result))
}

async fn get_status_badge(project: web::Path<(String,)>, database: Data<Database>) -> HttpResponse {
    let status_badge = get_project_status_badge(project.into_inner().0, &database).await;

    if !status_badge.is_empty() {
        return HttpResponse::Ok()
            .content_type("image/svg+xml")
            .body(status_badge);
    }

    HttpResponse::NotFound().finish()
}

async fn save_project_build_status(project: String, status: String, database: &Database) {
    let document = doc! { "project": &project, "buildStatus": &status };

    let collection = database.collection("build_statuses");

    let build_status = get_project_build_status(&project, &database).await;

    // If it's not already in db - insert, otherwise just update
    if build_status.is_empty() {
        if let Err(e) = collection.insert_one(document, None).await {
            eprintln!(
                "Error adding document for project {}, with status of {}, error: {:#?}",
                project, status, e
            );
        }
    } else {
        let filter = doc! { "project": &project };

        if let Err(e) = collection.update_one(filter, document, None).await {
            eprintln!(
                "Error updating document for project {}, with status of {}, error: {:#?}",
                project, status, e
            );
        }
    }
}

async fn get_project_status_badge(project: String, database: &Database) -> String {
    let badge_options: BadgeOptions;

    let build_status = get_project_build_status(&project, &database).await;
    if build_status.eq("passing") {
        badge_options = BadgeOptions {
            subject: "drovah".to_owned(),
            status: build_status,
            color: "#4c1".to_owned(),
        };
    } else if build_status.eq("failing") {
        badge_options = BadgeOptions {
            subject: "drovah".to_owned(),
            status: build_status,
            color: "#ed2e25".to_owned(),
        };
    } else {
        badge_options = BadgeOptions {
            subject: "drovah".to_owned(),
            status: "unknown".to_owned(),
            color: "#A9A9A9".to_owned(),
        };
    }

    if let Ok(badge) = Badge::new(badge_options) {
        let svg = badge.to_svg();
        return svg;
    }

    "".to_owned()
}

async fn get_project_build_status(project: &String, database: &Database) -> String {
    let collection = database.collection("build_statuses");

    if let Ok(mut cursor) = collection.find(doc! { "project": project }, None).await {
        while let Some(doc_result) = cursor.next().await {
            if let Ok(document) = doc_result {
                if let Some(status) = document.get("buildStatus").and_then(Bson::as_str) {
                    return status.to_owned();
                }
            }
        }
    }

    "".to_owned()
}

pub async fn launch_webserver() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let conf_str = fs::read_to_string(Path::new("drovah.toml")).unwrap();
    let drovah_config: DrovahConfig = toml::from_str(&conf_str).unwrap();

    HttpServer::new(move || {
        let drovah_config: DrovahConfig = toml::from_str(&conf_str).unwrap();
        let client_future = Client::with_uri_str(&drovah_config.mongo.mongo_connection_string);
        let client = block_on(client_future).unwrap();
        let database = client.database(&drovah_config.mongo.mongo_db);

        // Create app
        App::new()
            .data(database)
            .wrap(middleware::Logger::default())
            .service(web::resource("/{project}/badge").route(web::get().to(get_status_badge)))
            .service(
                web::resource("/{project}/specific/<file>").route(web::get().to(get_specific_file)),
            )
            .service(web::resource("/{project}/latest").route(web::get().to(get_latest_file)))
            .service(
                web::resource("/api/projects")
                    .route(web::get().to(get_project_information)),
            )
            .service(web::resource("/webhook").route(web::post().to(github_webhook)))
            .service(actix_files::Files::new("/", "static").show_files_listing())
            .wrap(Logger::default())
    })
    .bind(drovah_config.web.address)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_matching() {
        let file_to_find = ".drovah";
        let path = match_filename_to_file(file_to_find);
        assert!(path.is_some());

        let file_to_find = ".asodmasdasd";
        let path = match_filename_to_file(file_to_find);

        assert!(path.is_none());

        let file_to_find = "./.dro";
        let path = match_filename_to_file(file_to_find);

        assert!(path.is_some());
        assert_eq!(path.unwrap(), String::from("./.drovah"));
    }
}
