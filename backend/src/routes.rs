use std::collections::HashMap;
use std::fs;
use std::path::Path;

use actix_files::NamedFile;
use actix_web::http::header::HeaderMap;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{web, HttpResponse, get, post};
use diesel::MysqlConnection;
use serde_json::json;

use diesel::r2d2::{self, ConnectionManager};

use crate::{
    get_build_number, get_latest_build_status, get_project_data, get_project_id,
    get_project_status_badge, get_status_for_build, run_build, run_commands,
    verify_authentication_header, WebhookData,
};

type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

/// Returns specific file
#[get("/api/v1/{project}/{build}/{file}")]
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
#[get("/api/v1/projects")]
pub(crate) async fn get_project_information(pool: Data<DbPool>) -> actix_web::Result<HttpResponse> {
    let dir = Path::new("data/projects/");
    let database = pool.get().expect("couldn't get db connection from pool");

    let mut projects = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let project_id = get_project_id(&database, &file_name);
                if let Some(project_id) = project_id {
                    let project_data = get_project_data(&database, project_id);
                    projects.push(project_data);
                }
            }
        }
    }

    let json_result = json!({ "projects": projects });

    actix_web::Result::Ok(HttpResponse::Ok().json(json_result))
}

/// Returns latest status badge for given project
#[get("/api/v1/{project}/badge")]
pub(crate) async fn get_latest_status_badge(
    project: web::Path<(String,)>,
    pool: Data<DbPool>,
) -> HttpResponse {
    let database = pool.get().expect("couldn't get db connection from pool");
    let project_id = get_project_id(&database, &project.into_inner().0);

    if let Some(project_id) = project_id {
        let latest_status = get_latest_build_status(&database, project_id);
        let badge = get_project_status_badge(latest_status);

        if !badge.is_empty() {
            return HttpResponse::Ok().content_type("image/svg+xml").body(badge);
        }
    }

    HttpResponse::NotFound().finish()
}

/// Returns status badge for specific build
#[get("/api/v1/{project}/{build}/badge")]
pub(crate) async fn get_status_badge_for_build(
    path: web::Path<(String, i32)>,
    pool: Data<DbPool>,
) -> HttpResponse {
    let database = pool.get().expect("couldn't get db connection from pool");
    let inner = path.into_inner();
    let project = inner.0;
    let build = inner.1;
    let project_id = get_project_id(&database, &project);

    if let Some(project_id) = project_id {
        let status = get_status_for_build(&database, project_id, build);
        let status_badge = get_project_status_badge(status);

        if !status_badge.is_empty() {
            return HttpResponse::Ok()
                .content_type("image/svg+xml")
                .body(status_badge);
        }
    }

    HttpResponse::NotFound().finish()
}

/// Handles webhook
/// targeted towards GitHub's webhook
/// however, would support others as long as they adhere to format
/// URL is <host>:<port>/webhook
#[post("/api/v1/webhook")]
pub(crate) async fn github_webhook(
    request: web::HttpRequest,
    body: web::Bytes,
    pool: Data<DbPool>,
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
        tokio::spawn(async move {
            let commands = vec!["git pull".to_owned()];
            run_commands(commands, &project_path, false);

            let database = pool.get().expect("couldn't get db connection from pool");

            if let Err(e) = run_build(webhookdata.repository.name, &database) {
                eprintln!("Error! {}", e);
            }
        });

        return actix_web::Result::Ok(HttpResponse::NoContent().finish());
    }

    actix_web::Result::Ok(HttpResponse::NotAcceptable().body("Project doesn't exist"))
}

/// Returns latest file
/// If one does not exist, will just return an os error of not found
#[get("/api/v1/{project}/latest")]
pub(crate) async fn get_latest_file(
    project: web::Path<(String,)>,
    pool: Data<DbPool>,
) -> actix_web::Result<NamedFile> {
    let database = pool.get().expect("couldn't get db connection from pool");
    let project = project.into_inner().0;
    let project_id = get_project_id(&database, &project).unwrap();

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
