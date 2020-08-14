extern crate actix_web;

use std::path::Path;
use std::{fs, io, env};

use drovah::{launch_webserver};

#[actix_rt::main]
async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");

    let path = Path::new("Drovah.toml");
    let projects_path = Path::new("data/projects/");
    let archive_path = Path::new("data/archive/");

    if !projects_path.exists() {
        if let Err(e) = fs::create_dir_all(projects_path) {
            eprintln!("Error occurred: {}", e);
        }
    }

    if !archive_path.exists() {
        if let Err(e) = fs::create_dir(archive_path) {
            eprintln!("Error occurred: {}", e);
        }
    }

    if !path.exists() {
        let default_file = r#"[mongo]
mongo_connection_string = "mongodb://localhost:27017"
mongo_db = "drovah""#;

        if let Err(e) = fs::write(path, default_file) {
            eprintln!("Error creating default Drovah.toml file! {}", e);
        }

        println!("No 'Drovah.toml' found, so we generated a default one!");
    }


    launch_webserver().await
}
