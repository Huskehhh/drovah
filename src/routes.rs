extern crate actix_files;
extern crate actix_web;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use actix_files::NamedFile;
use actix_web::http::header::HeaderMap;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{web, HttpResponse};
use diesel::MysqlConnection;
use serde_json::json;

use crate::{
    get_build_number, get_latest_build_status, get_project_data, get_project_id,
    get_project_status_badge, get_status_for_build, run_build, run_commands,
    verify_authentication_header, WebhookData,
};

/// Returns specific file
/// URL is <host>:<port>/<project>/<build>/<file>
pub(crate) async fn get_file_for_build(
    path: web::Path<(String, String, String)>,
) -> actix_web::Result<NamedFile> {
    let inner = path.into_inner();
    let project = inner.0;
    let build = inner.1;
    let file = inner.2;

    let formatted = format!("data/archive/{}/{}/{}", project, build, file);

    let p = Path::new(&formatted);

    actix_web::Result::Ok(NamedFile::open(p)?)
}

/// Returns project information for current path
/// URL is <host>:<port>/api/projects
pub(crate) async fn get_project_information(
    database: Data<MysqlConnection>,
) -> actix_web::Result<HttpResponse> {
    let dir = Path::new("data/projects/");

    let mut projects = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let project_id = get_project_id(&database, &file_name);
                let project_data = get_project_data(&database, project_id);
                projects.push(project_data);
            }
        }
    }

    let json_result = json!({ "projects": projects });

    actix_web::Result::Ok(HttpResponse::Ok().json(json_result))
}

/// Returns latest status badge for given project
/// URL is <host>:<port>/<project>/badge
pub(crate) async fn get_latest_status_badge(
    project: web::Path<(String,)>,
    database: Data<MysqlConnection>,
) -> HttpResponse {
    let project_id = get_project_id(&database, &project.into_inner().0);
    let latest_status = get_latest_build_status(&database, project_id);

    let badge = get_project_status_badge(latest_status);

    if !badge.is_empty() {
        return HttpResponse::Ok().content_type("image/svg+xml").body(badge);
    }

    HttpResponse::NotFound().finish()
}

/// Returns status badge for specific build
/// URL is <host>:<port>/<project>/<build>/badge
pub(crate) async fn get_status_badge_for_build(
    path: web::Path<(String, i32)>,
    database: Data<MysqlConnection>,
) -> HttpResponse {
    let inner = path.into_inner();
    let project = inner.0;
    let build = inner.1;
    let project_id = get_project_id(&database, &project);

    let status = get_status_for_build(&database, project_id, build);
    let status_badge = get_project_status_badge(status);

    if !status_badge.is_empty() {
        return HttpResponse::Ok()
            .content_type("image/svg+xml")
            .body(status_badge);
    }

    HttpResponse::NotFound().finish()
}

/// Handles webhook
/// targeted towards GitHub's webhook
/// however, would support others as long as they adhere to format
/// URL is <host>:<port>/webhook
pub(crate) async fn github_webhook(
    database: Data<MysqlConnection>,
    request: web::HttpRequest,
    body: web::Bytes,
) -> actix_web::Result<HttpResponse> {
    // Begin github secret auth
    let body: Vec<u8> = body.to_vec();
    let headers = get_headers_hash_map(request.headers())?;

    verify_authentication_header(&headers, &body)?;

    // Parse json manually as actix doesn't support multiple extractors
    let webhookdata: WebhookData = serde_json::from_slice(&body).expect("Json error");

    let project_path = format!("data/projects/{}/", &webhookdata.repository.name);
    let path = Path::new(&project_path);
    if path.exists() {
        let commands = vec!["git pull".to_owned()];
        run_commands(commands, &project_path, false);

        if let Err(e) = run_build(webhookdata.repository.name.clone(), &database).await {
            eprintln!("Error! {}", e);
        }

        return actix_web::Result::Ok(HttpResponse::NoContent().finish());
    }

    actix_web::Result::Ok(HttpResponse::NotAcceptable().body("Project doesn't exist"))
}

/// Credit - https://github.com/Nukesor/webhook-server/blob/master/src/web.rs#L148
fn get_headers_hash_map(map: &HeaderMap) -> Result<HashMap<String, String>, HttpResponse> {
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

/// Returns latest file
/// If one does not exist, will just return an os error of not found
/// <host>:<port>/<project>/latest
pub(crate) async fn get_latest_file(
    project: web::Path<(String,)>,
    database: Data<MysqlConnection>,
) -> actix_web::Result<NamedFile> {
    let project = project.into_inner().0;
    let project_id = get_project_id(&database, &project);

    let build_number = get_build_number(&database, project_id);

    let path_str = format!("data/archive/{}/{}/", &project, build_number);
    let path = Path::new(&path_str);

    let dir = fs::read_dir(path)?;

    for file in dir {
        if let Ok(unwrapped) = file {
            if !unwrapped.path().extension().unwrap().eq("log") {
                return actix_web::Result::Ok(NamedFile::open(unwrapped.path())?);
            }
        }
    }

    let file = NamedFile::open(fs::read_dir(path)?.last().unwrap()?.path())?;

    actix_web::Result::Ok(file)
}

/// Default path, returns the index file
pub(crate) async fn index() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/dist/index.html")))
}
