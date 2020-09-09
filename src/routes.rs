extern crate actix_files;
extern crate actix_web;

use crate::{
    get_current_build_number, get_latest_build_status, get_project_data, get_project_status_badge,
    run_build, run_commands, WebhookData,
};
use actix_files::NamedFile;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{web, HttpResponse};
use mongodb::Database;
use serde_json::json;
use std::fs;
use std::path::Path;

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
    database: Data<Database>,
) -> actix_web::Result<HttpResponse> {
    let dir = Path::new("data/projects/");

    let mut projects = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let optional_project_data = get_project_data(&file_name, &database).await;

                if let Some(project_data) = optional_project_data {
                    projects.push(project_data);
                }
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
    database: Data<Database>,
) -> HttpResponse {
    let latest_status = get_latest_build_status(&project.into_inner().0, &database).await;

    if let Some(status) = latest_status {
        let badge = get_project_status_badge(status).await;

        if !badge.is_empty() {
            return HttpResponse::Ok().content_type("image/svg+xml").body(badge);
        }
    }

    HttpResponse::NotFound().finish()
}

/// Returns status badge for specific build
/// URL is <host>:<port>/<project>/<build>/badge
pub(crate) async fn get_status_badge_for_build(
    path: web::Path<(String, i32)>,
    database: Data<Database>,
) -> HttpResponse {
    let inner = path.into_inner();
    let project = inner.0;
    let build = inner.1 - 1;

    if let Some(project_data) = get_project_data(&project, &database).await {
        let build_info_optional = project_data.builds.get(build as usize);

        if let Some(build_info) = build_info_optional {
            let status = build_info.build_status.clone();
            let status_badge = get_project_status_badge(status).await;

            if !status_badge.is_empty() {
                return HttpResponse::Ok()
                    .content_type("image/svg+xml")
                    .body(status_badge);
            }
        }
    }

    HttpResponse::NotFound().finish()
}

/// Handles webhook
/// targeted towards GitHub's webhook
/// however, would support others as long as they adhere to format
/// URL is <host>:<port>/webhook
pub(crate) async fn github_webhook(
    webhookdata: Json<WebhookData>,
    database: Data<Database>,
) -> actix_web::Result<HttpResponse> {
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

        return actix_web::Result::Ok(HttpResponse::NoContent().finish());
    }

    actix_web::Result::Ok(HttpResponse::NotAcceptable().body("Project doesn't exist"))
}

/// Returns latest file
/// If one does not exist, will just return an os error of not found
/// <host>:<port>/<project>/latest
pub(crate) async fn get_latest_file(
    project: web::Path<(String,)>,
    database: Data<Database>,
) -> actix_web::Result<NamedFile> {
    let project = project.into_inner().0;
    let build_number = get_current_build_number(&project, &database).await;

    let path_str = format!("data/archive/{}/{}/", &project, build_number);
    let path = Path::new(&path_str);

    let file = NamedFile::open(fs::read_dir(path)?.last().unwrap()?.path())?;

    actix_web::Result::Ok(file)
}

/// Default path, returns the index file
pub(crate) async fn index() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/dist/index.html")))
}
