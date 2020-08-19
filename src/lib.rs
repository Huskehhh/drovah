extern crate actix_files;
extern crate actix_web;
extern crate env_logger;

use std::{fs, io};
use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

use actix_files::NamedFile;
use actix_web::{App, HttpResponse, HttpServer, middleware, web};
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::web::{Data, Json};
use badge::{Badge, BadgeOptions};
use futures::executor::block_on;
use mongodb::{
    bson::{Bson, doc},
    Client, Database,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::stream::StreamExt;

#[derive(Debug, Deserialize)]
struct WebhookData {
    repository: RepositoryData,
}

#[derive(Debug, Deserialize)]
struct RepositoryData {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectData {
    project: String,
    builds: Vec<BuildData>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildData {
    build_number: i32,
    build_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInformation {
    project: String,
    has_file: bool,
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
                if archive_files(files.files, &project, database).await {
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

            save_project_build_data(project, "passing".to_owned(), database).await;
        } else {
            println!("'{}' has failed to build.", project);
            save_project_build_data(project, "failing".to_owned(), database).await;
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

async fn archive_files(files_to_archive: Vec<String>, project: &str, database: &Database) -> bool {
    let archive_folder = format!("data/archive/{}/", project);
    let archive_path = Path::new(&archive_folder);
    if !archive_path.exists() {
        if let Err(e) = fs::create_dir_all(archive_path) {
            eprintln!("Error creating directories: {}, {}", archive_folder, e);
        }
    }

    let build_number = get_current_build_number(project, database).await + 1;

    for file_to_match in files_to_archive {
        let path_to_search = format!("data/projects/{}/{}", project, file_to_match);
        if let Some(matched) = match_filename_to_file(&path_to_search) {
            let matched_file_name = matched.split("/").last().unwrap();
            let to = format!(
                "data/archive/{}/{}/{}",
                project, build_number, matched_file_name
            );
            return copy(&matched, &to);
        }
    }

    false
}

async fn get_current_build_number(project: &str, database: &Database) -> i32 {
    if let Some(project_data) = get_project_data(project, database).await {
        return project_data.builds.len() as i32;
    }
    1
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

    if let Err(e) = fs::create_dir_all(to.parent().unwrap()) {
        eprintln!("Error creating directory pre-copy {}", e);
        return false;
    }

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

        tokio::spawn(async move {
            let commands = vec!["git pull".to_owned()];
            run_commands(commands, &project_path);

            if let Err(e) = run_build(webhookdata.repository.name.clone(), &database).await {
                eprintln!("Error! {}", e);
            }
        });

        return HttpResponse::NoContent().finish();
    }

    HttpResponse::NotAcceptable().body("Project doesn't exist")
}

async fn api_get_latest_file(
    project: web::Path<(String, )>,
    database: Data<Database>,
) -> actix_web::Result<NamedFile> {
    let project = project.into_inner().0;
    get_latest_file(&project, &database).await
}

async fn get_latest_file(project: &str, database: &Database) -> actix_web::Result<NamedFile> {
    let build_number = get_current_build_number(&project, &database).await;

    let path_str = format!("data/archive/{}/{}/", &project, build_number);
    let path = Path::new(&path_str);

    let file = NamedFile::open(fs::read_dir(path)?.last().unwrap()?.path())?;

    actix_web::Result::Ok(file)
}

async fn get_project_information(database: Data<Database>) -> actix_web::Result<HttpResponse> {
    let dir = Path::new("data/projects/");

    let mut projects = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let has_file = get_latest_file(&file_name, &database).await.is_ok();
                let project_information = ProjectInformation {
                    project: file_name,
                    has_file,
                };

                projects.push(project_information);
            }
        }
    }

    let json_result = json!({ "projects": projects });

    actix_web::Result::Ok(HttpResponse::Ok().json(json_result))
}

async fn get_status_badge(project: web::Path<(String, )>, database: Data<Database>) -> HttpResponse {
    let status_badge = get_project_status_badge(project.into_inner().0, &database).await;

    if !status_badge.is_empty() {
        return HttpResponse::Ok()
            .content_type("image/svg+xml")
            .body(status_badge);
    }

    HttpResponse::NotFound().finish()
}

async fn index() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/index.html")))
}

async fn get_project_data(project: &str, database: &Database) -> Option<ProjectData> {
    let collection = database.collection("project_data");

    let document = doc! { "project": &project };

    if let Ok(mut cursor) = collection.find(document, None).await {
        while let Some(doc_result) = cursor.next().await {
            if let Ok(document) = doc_result {
                let project_data: ProjectData = bson::from_bson(Bson::Document(document)).unwrap();
                return Some(project_data);
            }
        }
    }
    None
}

async fn save_project_build_data(project: String, status: String, database: &Database) {
    let collection = database.collection("project_data");
    let project_data = get_project_data(&project, database).await;

    // Currently has data, lets replace rather than add new
    if project_data.is_some() {
        let mut project_data = project_data.unwrap();

        let build_data = BuildData {
            build_number: project_data.builds.len() as i32 + 1,
            build_status: status,
        };

        // Add this build to the list
        project_data.builds.push(build_data);

        let serialised = bson::to_bson(&project_data).unwrap();
        let document = serialised.as_document().unwrap().clone();

        let filter = doc! { "project": &project };

        if let Err(e) = collection.update_one(filter, document, None).await {
            eprintln!(
                "Error updating document for project {}, error: {:#?}",
                project, e
            );
        }
    } else {
        let build_data = BuildData {
            build_number: 1,
            build_status: status,
        };
        let project_data: ProjectData = ProjectData {
            project,
            builds: vec![build_data],
        };

        let serialised = bson::to_bson(&project_data).unwrap();
        let document = serialised.as_document().unwrap().clone();

        if let Err(e) = collection.insert_one(document, None).await {
            eprintln!(
                "Error adding document for project {}, error: {:#?}",
                project_data.project, e
            );
        }
    }
}

async fn get_project_status_badge(project: String, database: &Database) -> String {
    let build_status = get_latest_build_status(&project, &database).await;

    let mut badge_options: BadgeOptions = Default::default();

    if let Some(status) = build_status {
        if status.eq("passing") {
            badge_options = BadgeOptions {
                subject: "drovah".to_owned(),
                status,
                color: "#4c1".to_owned(),
            };
        } else if status.eq("failing") {
            badge_options = BadgeOptions {
                subject: "drovah".to_owned(),
                status,
                color: "#ed2e25".to_owned(),
            };
        }
    } else if let None = build_status {
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

async fn get_latest_build_status(project: &str, database: &Database) -> Option<String> {
    let collection = database.collection("project_data");

    let document = doc! { "project": & project };

    if let Ok(mut cursor) = collection.find(document, None).await {
        while let Some(doc_result) = cursor.next().await {
            if let Ok(document) = doc_result {
                if let Some(builds) = document.get("builds").and_then(Bson::as_array) {
                    if let Some(last) = builds.last().and_then(Bson::as_document) {
                        if let Some(latest_build_status) =
                        last.get("buildStatus").and_then(Bson::as_str)
                        {
                            return Some(String::from(latest_build_status));
                        }
                    }
                }
            }
        }
    }

    None
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
            .service(web::resource("/{project}/latest").route(web::get().to(api_get_latest_file)))
            .service(web::resource("/api/projects").route(web::get().to(get_project_information)))
            .service(web::resource("/webhook").route(web::post().to(github_webhook)))
            .service(web::resource("/").route(web::get().to(index)))
            .service(actix_files::Files::new("/", "static").show_files_listing())
            .service(actix_files::Files::new("/data", "data/archive").show_files_listing())
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

        let file_to_find = ".doesnt exist";
        let path = match_filename_to_file(file_to_find);

        assert!(path.is_none());

        let file_to_find = "./.dro";
        let path = match_filename_to_file(file_to_find);

        assert!(path.is_some());
        assert_eq!(path.unwrap(), String::from("./.drovah"));
    }
}
