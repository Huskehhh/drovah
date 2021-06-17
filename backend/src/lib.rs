#[macro_use]
extern crate diesel;
extern crate actix_web;
extern crate env_logger;

use std::process::{Command, Stdio};
use std::{collections::HashMap, ffi::OsStr};
use std::{env, fs};
use std::{fs::File, io, path::Path};

use actix_cors::Cors;
use actix_web::http::HeaderMap;
use diesel::{insert_into, prelude::*};

use actix_web::{
    middleware::{self, Logger},
    App, HttpResponse, HttpServer,
};
use badge::{Badge, BadgeOptions};
use diesel::MysqlConnection;
use hmac::{Hmac, Mac, NewMac};
use models::{Build, Project};
use routes::{
    get_file_for_build, get_latest_file, get_latest_status_badge, get_project_information,
    get_status_badge_for_build, github_webhook,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::error::Error;

use diesel::r2d2::{self, ConnectionManager};

use crate::schema::builds::dsl as build;
use crate::schema::projects::dsl as proj;

pub mod models;
mod routes;
pub mod schema;

type HmacSha256 = Hmac<Sha256>;

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
#[derive(Debug, Serialize)]
pub struct ProjectData {
    project: String,
    builds: Vec<BuildData>,
}

/// Represents stored build data
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildData {
    build_number: i32,
    build_status: String,
    archived_files: Vec<String>,
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

/// Method to run a build for a project
/// Takes the project name (String) and ref to database (&Database) to store result
fn run_build(project: String, database: &MysqlConnection) -> Result<(), Box<dyn Error>> {
    println!("Building '{}'", project);
    let project_path = format!("data/projects/{}", project);
    let path = Path::new(&project_path);

    let project_id = get_project_id(database, &project);

    if path.exists() && path.is_dir() {
        let settings_file_path = format!("{}/.drovah", project_path);
        let ci_settings_file = Path::new(&settings_file_path);
        let settings_string = fs::read_to_string(ci_settings_file)?;
        let ci_config: CIConfig = toml::from_str(&settings_string)?;

        if run_commands(
            ci_config.build.commands,
            &project_path,
            ci_config.archive.is_some(),
        ) {
            println!("Success! '{}' has been built.", project);

            if let Some(files) = ci_config.archive {
                if archive_files(
                    files.files,
                    project_id.unwrap(),
                    database,
                    files.append_buildnumber,
                ) {
                    println!("Successfully archived files for '{}'", project);

                    if let Some(post_archive) = ci_config.postarchive {
                        if run_commands(post_archive.commands, &project_path, false) {
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
                save_project_build_data(project, "passing".to_owned(), database, vec![]);
            }
        } else {
            println!("'{}' has failed to build.", project);
            save_project_build_data(project, "failing".to_owned(), database, vec![]);
        }
    }
    Ok(())
}

/// Archives nominated files for a project
/// Files are stored in 'data/archive/<project>/<build number>/
fn archive_files(
    files_to_archive: Vec<String>,
    project_id: i32,
    database: &MysqlConnection,
    append_buildnumber: Option<bool>,
) -> bool {
    let project_name = get_project_name(&database, project_id);
    let mut success = false;

    if let Some(project_name) = project_name {
        let archive_folder = format!("data/archive/{}/", project_name);
        let archive_path = Path::new(&archive_folder);
        if !archive_path.exists() {
            if let Err(e) = fs::create_dir_all(archive_path) {
                eprintln!("Error creating directories: {}, {}", archive_folder, e);
            }
        }

        let build_number = get_build_number(&database, project_id) + 1;

        let mut filenames = vec![];

        // Copy log file
        let from = format!("data/projects/{}/build.log", project_name);
        let to = format!("data/archive/{}/{}/build.log", project_name, build_number);
        if copy(&from, &to) {
            filenames.push("build.log".to_owned());
            if let Err(e) = fs::remove_file(Path::new(&from)) {
                eprintln!(
                    "Error when deleting build.log for project: {}, {}",
                    project_name, e
                );
            }
        } else {
            println!(
                "Error copying build.log for {} with build number: {}",
                project_name, build_number
            );
        }

        // Copy other files
        for file_to_match in files_to_archive {
            let path_to_search = format!("data/projects/{}/{}", project_name, file_to_match);
            if let Some(matched) = match_filename_to_file(&path_to_search) {
                let matched_file_name = matched.split('/').last().unwrap();

                if append_buildnumber.is_some() {
                    if append_buildnumber.unwrap() {
                        let ext = Path::new(matched_file_name)
                            .extension()
                            .and_then(OsStr::to_str)
                            .unwrap();

                        let replace = format!(".{}", ext);
                        let filename = matched_file_name.replace(&replace, "");
                        let final_file = format!("{}-b{}.{}", filename, build_number, ext);

                        let to = format!(
                            "data/archive/{}/{}/{}",
                            project_name, build_number, final_file
                        );

                        if copy(&matched, &to) {
                            filenames.push(final_file.to_owned());
                            success = true;
                        }
                    }
                } else {
                    let to = format!(
                        "data/archive/{}/{}/{}",
                        project_name, build_number, matched_file_name
                    );

                    if copy(&matched, &to) {
                        filenames.push(matched_file_name.to_owned());
                        success = true;
                    }
                }
            }
        }

        if success {
            save_project_build_data(project_name, "passing".to_owned(), database, filenames);
        }
    }

    success
}

/// Launches the actix webserver
/// Takes configuration from drovah.toml
pub async fn launch_webserver() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8000".to_owned());

    HttpServer::new(move || {
        let db_url =
            env::var("DATABASE_URL").expect("No DATABASE_URL environment variable defined!");
        let manager = ConnectionManager::<MysqlConnection>::new(db_url);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");

        let allowed_origin =
            env::var("ALLOWED_ORIGIN").expect("No ALLOWED_ORIGIN environment variable set!");

        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allowed_methods(vec!["GET", "POST"]);

        // Create app
        App::new()
            .data(pool)
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .service(get_latest_status_badge)
            .service(get_latest_file)
            .service(get_status_badge_for_build)
            .service(get_file_for_build)
            .service(get_project_information)
            .service(github_webhook)
            .wrap(Logger::default())
    })
    .bind(bind_address)?
    .run()
    .await
}

/// Saves project build data to database
fn save_project_build_data(
    project: String,
    status: String,
    database: &MysqlConnection,
    archived_files: Vec<String>,
) {
    let p_id = get_project_id(&database, &project);
    if let Some(p_id) = p_id {
        let build_num = get_build_number(&database, p_id) + 1;

        let sep_files = archived_files.join(", ");

        if let Err(why) = insert_into(build::builds)
            .values((
                build::project_id.eq(p_id),
                build::build_number.eq(build_num),
                build::branch.eq("master".to_owned()),
                build::files.eq(sep_files),
                build::status.eq(status),
            ))
            .execute(database)
        {
            eprintln!(
                "Error on insert of build {} for {}! {}",
                build_num, project, why
            )
        }
    }
}

/// Verifies the auth header for the commit via webhook
pub fn verify_authentication_header(
    headers: &HashMap<String, String>,
    body: &[u8],
) -> Result<(), HttpResponse> {
    let secret = env::var("GITHUB_SECRET").expect("No GITHUB_SECRET environment variable!");
    let mut signature_valid = false;

    // Check for a correct signature, if we have as secret or both authentication methods are required
    let signature = get_signature_header(headers)?;
    if !signature.is_empty() {
        verify_signature_header(signature, secret, body)?;
        signature_valid = true;
    }

    if signature_valid {
        return Ok(());
    }

    Ok(())
}

/// Credit - https://github.com/Nukesor/webhook-server/blob/master/src/authentication.rs#L88
/// Verify the signature header. Checks our own signature generated by hmac sha256 with secret and payload
/// against the signature provided in the header.
fn verify_signature_header(
    signature: String,
    secret: String,
    body: &[u8],
) -> Result<(), HttpResponse> {
    // Try to decode the sha256 into bytes. Should be a valid hex string
    let signature_bytes = match hex::decode(&signature) {
        Ok(result) => result,
        Err(error) => {
            println!("Error decoding signature: {}, {}", signature, error);
            return Err(HttpResponse::Unauthorized().body("Invalid sha256 signature"));
        }
    };

    // Generate the own hmac sha256 from the secret and body and verify that it's identical to the signature
    let secret_bytes = secret.into_bytes();
    let expected_signature = generate_signature_sha256(&secret_bytes, body);

    match expected_signature.clone().verify(&signature_bytes) {
        Ok(()) => Ok(()),
        Err(_) => {
            println!(
                "Our sha256: {}",
                hex::encode(expected_signature.finalize().into_bytes())
            );
            println!("Got wrong sha256: {}", signature);
            Err(HttpResponse::Unauthorized().body("Invalid sha256 signature"))
        }
    }
}

/// Create a hmac SHA256 instance from a secret and body
fn generate_signature_sha256(secret_bytes: &[u8], body: &[u8]) -> HmacSha256 {
    let mut hmac =
        HmacSha256::new_from_slice(secret_bytes).expect("Couldn't create hmac with current secret");
    hmac.update(body);
    hmac
}

/// Credit - https://github.com/Nukesor/webhook-server/blob/master/src/authentication.rs#L63
/// Extract the correct signature header content from all headers
/// It's possible to receive the signature from multiple Headers, since Github uses their own
/// Header name for their signature method.
fn get_signature_header(headers: &HashMap<String, String>) -> Result<String, HttpResponse> {
    let mut header = headers.get("signature");
    if header.is_none() {
        header = headers.get("x-hub-signature-256");
    }

    // We dont' find any headers for signatures and this method is not required
    let mut header = if let Some(header) = header {
        header.clone()
    } else {
        return Ok("".to_string());
    };

    // Header must be formatted like this: sha256={{hash}}
    if !header.starts_with("sha256=") {
        println!("warning: Got request with missing sha256= prefix");
        Err(HttpResponse::Unauthorized()
            .body("Error while parsing signature: Couldn't find prefix"))
    } else {
        Ok(header.split_off(5))
    }
}

/// Runs the commands required for the build in .drovah
fn run_commands(commands: Vec<String>, directory: &str, save_log: bool) -> bool {
    let mut success = 0;

    let commands_len = commands.len();

    for command in commands {
        let split: Vec<&str> = command.split(' ').collect();

        let program = split.first().expect("Error, commands are formatted wrong!");
        let process;

        if save_log {
            let output_path = format!("{}/build.log", directory);
            let outputs = File::create(&output_path).expect("Error creating file 'build.log'");
            let errors = outputs.try_clone().unwrap();

            process = Command::new(program)
                .current_dir(directory)
                .args(&split[1..])
                .stdout(Stdio::from(outputs))
                .stderr(Stdio::from(errors))
                .spawn()
                .expect(
                    "run_commands failed, are they formatted correctly? is the program installed?",
                );
        } else {
            process = Command::new(program)
                .current_dir(directory)
                .args(&split[1..])
                .stdout(Stdio::piped())
                .spawn()
                .expect(
                    "run_commands failed, are they formatted correctly? is the program installed?",
                );
        }

        let result = process
            .wait_with_output()
            .expect("Unexpectedly died on commands!");

        if result.status.success() {
            success += 1;
        }
    }

    success as usize == commands_len
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
            for file in paths.flatten() {
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

/// Returns status badge for given status
fn get_project_status_badge(status: String) -> String {
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

/// Gets the project id of given project
pub fn get_project_id(connection: &MysqlConnection, project: &str) -> Option<i32> {
    let result = proj::projects
        .filter(proj::project_name.eq(project))
        .limit(1)
        .load::<Project>(connection)
        .expect("Error getting project id!");

    Some(result.first()?.project_id)
}

/// Gets the project name of a given project id
pub fn get_project_name(connection: &MysqlConnection, pid: i32) -> Option<String> {
    let result = proj::projects
        .filter(proj::project_id.eq(pid))
        .limit(1)
        .load::<Project>(connection)
        .expect("Error getting project name from id!");

    Some(result.first()?.project_name.to_owned())
}

/// Gets the latest build number of a given project
pub fn get_build_number(connection: &MysqlConnection, pid: i32) -> i32 {
    let result = build::builds
        .filter(build::project_id.eq(pid))
        .order(build::build_number.desc())
        .load::<Build>(connection)
        .expect("Error getting project name from id!");

    if let Some(first) = result.first() {
        first.build_number
    } else {
        0
    }
}

/// Retrieves the latest build status for a given project
pub fn get_latest_build_status(connection: &MysqlConnection, pid: i32) -> String {
    let result = build::builds
        .filter(build::project_id.eq(pid))
        .limit(1)
        .load::<Build>(connection)
        .expect("Error getting project name from id!");

    if let Some(first) = result.first() {
        first.status.to_string()
    } else {
        "failing".to_string()
    }
}

/// Retrieves the status for a given build number
pub fn get_status_for_build(connection: &MysqlConnection, pid: i32, build_num: i32) -> String {
    let result = build::builds
        .filter(build::project_id.eq(pid))
        .filter(build::build_number.eq(build_num))
        .limit(1)
        .load::<Build>(connection)
        .expect("Error getting project name from id!");

    if let Some(first) = result.first() {
        first.status.to_string()
    } else {
        "failing".to_string()
    }
}

/// Retrieves the data of a project in ProjectData format
pub fn get_project_data(connection: &MysqlConnection, pid: i32) -> Option<ProjectData> {
    let result = build::builds
        .filter(build::project_id.eq(pid))
        .limit(10)
        .load::<Build>(connection)
        .expect("Error getting project name from id!");

    let project_name = get_project_name(connection, pid);

    let mut build_data_vec = vec![];
    for build in result {
        let split_files = build
            .files
            .split_terminator(", ")
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        build_data_vec.push(BuildData {
            build_number: build.build_number,
            build_status: build.status,
            archived_files: split_files,
        });
    }

    Some(ProjectData {
        project: project_name?,
        builds: build_data_vec,
    })
}

/// Credit - https://github.com/Nukesor/webhook-server/blob/master/src/web.rs#L148
pub fn get_headers_hash_map(map: &HeaderMap) -> Result<HashMap<String, String>, HttpResponse> {
    let mut headers = HashMap::new();

    for (key, header_value) in map.iter() {
        let key = key.as_str().to_string();
        let value: String;
        match header_value.to_str() {
            Ok(header_value) => value = header_value.to_string(),
            Err(error) => {
                let message = format!("Couldn't parse header: {}", error);
                println!("{}", message);
                return Err(HttpResponse::Unauthorized().body(message));
            }
        };

        headers.insert(key, value);
    }

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn test_file_append_build_number() {
        let matched_file_name = "project-v2.1.zip";
        let ext = Path::new(matched_file_name)
            .extension()
            .and_then(OsStr::to_str)
            .unwrap();

        let build_number = 5;

        let replace = format!(".{}", ext);
        let filename = matched_file_name.replace(&replace, "");
        let formatted = format!("{}-b{}.{}", filename, build_number, ext);

        assert_eq!(formatted, "project-v2.1-b5.zip");
    }
}
