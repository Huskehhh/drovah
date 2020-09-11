extern crate actix_files;
extern crate actix_web;
extern crate env_logger;

use std::{fs, io};
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

use actix_web::{App, HttpServer, middleware, web};
use actix_web::middleware::Logger;
use badge::{Badge, BadgeOptions};
use futures::executor::block_on;
use mongodb::{
    bson::{Bson, doc},
    Client, Database,
};
use serde::{Deserialize, Serialize};
use tokio::stream::StreamExt;

use crate::routes::{
    get_file_for_build, get_latest_file, get_latest_status_badge, get_project_information,
    get_status_badge_for_build, github_webhook, index,
};

mod routes;

/// Represents data taken from github webhook
#[derive(Debug, Deserialize)]
struct WebhookData {
    repository: RepositoryData,
}

/// Represents data taken from github webhook
#[derive(Debug, Deserialize)]
struct RepositoryData {
    name: String,
}

/// Represents data to be provided through 'get_project_information'
#[derive(Debug, Serialize, Deserialize)]
struct ProjectData {
    project: String,
    builds: Vec<BuildData>,
}

/// Represents stored build data
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildData {
    build_number: i32,
    build_status: String,
    archived_files: Option<Vec<String>>,
}

/// Represents project build configuration (.drovah)
#[derive(Debug, Deserialize)]
struct CIConfig {
    build: BuildConfig,
    archive: Option<ArchiveConfig>,
    postarchive: Option<PostArchiveConfig>,
}

/// Represents the build section of .drovah
#[derive(Debug, Deserialize)]
struct BuildConfig {
    commands: Vec<String>,
}

/// Represents the archive section of .drovah
#[derive(Debug, Deserialize)]
struct ArchiveConfig {
    files: Vec<String>,
    append_buildnumber: Option<bool>,
}

/// Represents the post archival section of .drovah
#[derive(Debug, Deserialize)]
struct PostArchiveConfig {
    commands: Vec<String>,
}

/// Represents the configuration of drovah (drovah.toml)
#[derive(Debug, Deserialize)]
struct DrovahConfig {
    web: WebServerConfig,
    mongo: MongoConfig,
}

/// Represents the mongo section of drovah.toml
#[derive(Debug, Deserialize)]
struct MongoConfig {
    mongo_connection_string: String,
    mongo_db: String,
}

/// Represents the web section of drovah.toml
#[derive(Debug, Deserialize)]
struct WebServerConfig {
    address: String,
}

/// Method to run a build for a project
/// Takes the project name (String) and ref to database (&Database) to store result
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
                if archive_files(files.files, &project, database, files.append_buildnumber).await {
                    println!("Successfully archived files for '{}'", project);

                    if let Some(post_archive) = ci_config.postarchive {
                        if run_commands(post_archive.commands, &project_path) {
                            println!("Successfully ran post-archive commands for '{}'", project);
                        } else {
                            println!(
                                "Error occurred running post-archive commands for '{}'",
                                project
                            );
                        }
                    }
                } else {
                    println!("Failed to archive files for '{}'", project);
                }
            } else {
                save_project_build_data(project, "passing".to_owned(), database, None).await;
            }
        } else {
            println!("'{}' has failed to build.", project);
            save_project_build_data(project, "failing".to_owned(), database, None).await;
        }
    }
    Ok(())
}

fn run_commands(commands: Vec<String>, directory: &str) -> bool {
    let mut success = false;

    for command in commands {
        let split: Vec<&str> = command.split(' ').collect();

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

        if result.status.success() {
            success = true;
        }
    }

    success
}

/// Archives nominated files for a project
/// Files are stored in 'data/archive/<project>/<build number>/
async fn archive_files(files_to_archive: Vec<String>, project: &str, database: &Database, append_buildnumber: Option<bool>) -> bool {
    let archive_folder = format!("data/archive/{}/", project);
    let archive_path = Path::new(&archive_folder);
    if !archive_path.exists() {
        if let Err(e) = fs::create_dir_all(archive_path) {
            eprintln!("Error creating directories: {}, {}", archive_folder, e);
        }
    }

    let build_number = get_current_build_number(project, database).await + 1;

    let mut success = false;
    let mut filenames = vec![];

    for file_to_match in files_to_archive {
        let path_to_search = format!("data/projects/{}/{}", project, file_to_match);
        if let Some(matched) = match_filename_to_file(&path_to_search) {
            let matched_file_name = matched.split('/').last().unwrap();

            if append_buildnumber.is_some() {
                if append_buildnumber.unwrap() {
                    let ext = Path::new(matched_file_name)
                        .extension()
                        .and_then(OsStr::to_str).unwrap();

                    let replace = format!(".{}", ext);
                    let filename = matched_file_name.replace(&replace, "");
                    let final_file = format!("{}-b{}.{}", filename, build_number, ext);

                    let to = format!(
                        "data/archive/{}/{}/{}",
                        project, build_number, final_file
                    );

                    if copy(&matched, &to) {
                        filenames.push(final_file.to_owned());
                        success = true;
                    }
                }
            } else {
                let to = format!(
                    "data/archive/{}/{}/{}",
                    project, build_number, matched_file_name
                );

                if copy(&matched, &to) {
                    filenames.push(matched_file_name.to_owned());
                    success = true;
                }
            }
        }
    }

    if success {
        save_project_build_data(
            project.to_owned(),
            "passing".to_owned(),
            database,
            Some(filenames),
        )
            .await;
    }

    success
}

/// Returns the current build number of given project
/// Defaults at 1
async fn get_current_build_number(project: &str, database: &Database) -> i32 {
    if let Some(project_data) = get_project_data(project, database).await {
        return project_data.builds.len() as i32;
    }
    1
}

/// Matches a filename into a file
fn match_filename_to_file(filename: &str) -> Option<String> {
    let path = Path::new(filename);

    // If the path is already a file, no need to process further
    if path.is_file() {
        let path_result = String::from(path.to_str().unwrap());
        return Option::Some(path_result);
    }

    // If not, lets look for it
    let file_to_look_for = filename.split('/').last().unwrap();

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

/// Copies file from source to destination
/// Will return whether or not it was successful
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

/// Returns project data struct optionally
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

/// Saves project build data to database
async fn save_project_build_data(
    project: String,
    status: String,
    database: &Database,
    archived_files: Option<Vec<String>>,
) {
    let collection = database.collection("project_data");
    let project_data = get_project_data(&project, database).await;

    // Currently has data, lets replace rather than add new
    if project_data.is_some() {
        let mut project_data = project_data.unwrap();

        let build_data = BuildData {
            build_number: project_data.builds.len() as i32 + 1,
            build_status: status,
            archived_files,
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
            archived_files,
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

/// Returns status badge for given status
async fn get_project_status_badge(status: String) -> String {
    let mut badge_options: BadgeOptions = Default::default();

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

    if let Ok(badge) = Badge::new(badge_options) {
        let svg = badge.to_svg();
        return svg;
    }

    "".to_owned()
}

/// Retrieves the latest build status for a given project
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

/// Launches the actix webserver
/// Takes configuration from drovah.toml
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
            .service(
                web::resource("/{project}/badge").route(web::get().to(get_latest_status_badge)),
            )
            .service(web::resource("/{project}/latest").route(web::get().to(get_latest_file)))
            .service(
                web::resource("/{project}/{build}/badge")
                    .route(web::get().to(get_status_badge_for_build)),
            )
            .service(
                web::resource("/{project}/{build}/{file}").route(web::get().to(get_file_for_build)),
            )
            .service(web::resource("/api/projects").route(web::get().to(get_project_information)))
            .service(web::resource("/webhook").route(web::post().to(github_webhook)))
            .service(web::resource("/").route(web::get().to(index)))
            .service(actix_files::Files::new("/", "static/dist/").show_files_listing())
            .wrap(Logger::default())
    })
        .bind(drovah_config.web.address)?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    pub async fn setup_database() -> Database {
        let conf_str = fs::read_to_string(Path::new("drovah.toml")).unwrap();
        let drovah_config: DrovahConfig = toml::from_str(&conf_str).unwrap();
        let client = Client::with_uri_str(&drovah_config.mongo.mongo_connection_string)
            .await
            .unwrap();
        client.database(&drovah_config.mongo.mongo_db)
    }

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

    #[tokio::test]
    async fn test_latest_build_status() {
        let db = setup_database().await;

        let status = get_latest_build_status("drovah", &db).await;

        assert!(status.is_some());

        let status = get_latest_build_status("something_completely_abstract", &db).await;

        assert!(status.is_none());
    }

    #[tokio::test]
    async fn test_get_status_badge() {
        let status_badge = get_project_status_badge("passing".to_owned()).await;

        assert_ne!(status_badge, "".to_owned());
    }

    #[tokio::test]
    async fn test_get_project_data() {
        let db = setup_database().await;

        let project_data = get_project_data("drovah", &db).await;

        assert!(project_data.is_some());

        let project_data = get_project_data("something_absurd", &db).await;

        assert!(project_data.is_none());
    }

    #[tokio::test]
    async fn test_get_build_number() {
        let db = setup_database().await;

        let build_num = get_current_build_number("drovah", &db).await;

        assert_ne!(build_num, 1);
    }

    #[tokio::test]
    async fn test_run_build() {
        let db = setup_database().await;

        let build_result = run_build("drovah".to_owned(), &db).await;

        assert!(build_result.is_ok());
    }

    #[test]
    fn test_file_append_build_number() {
        let matched_file_name = "project-v2.1.zip";
        let ext = Path::new(matched_file_name)
            .extension()
            .and_then(OsStr::to_str).unwrap();

        let build_number = 5;

        let replace = format!(".{}", ext);
        let filename = matched_file_name.replace(&replace, "");
        let formatted = format!("{}-b{}.{}", filename, build_number, ext);

        assert_eq!(formatted, "project-v2.1-b5.zip");
    }
}
