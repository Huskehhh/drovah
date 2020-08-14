extern crate actix_web;

use std::path::Path;
use std::{fs, io};

use drovah::{launch_webserver};

#[actix_rt::main]
async fn main() -> io::Result<()> {
    let path = Path::new("drovah.toml");
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
        let default_file = r#"[web]
address = "127.0.0.1:8000"

[mongo]
mongo_connection_string = "mongodb://localhost:27017"
mongo_db = "drovah""#;

        if let Err(e) = fs::write(path, default_file) {
            eprintln!("Error creating default drovah.toml file! {}", e);
        }

        println!("No 'drovah.toml' found, so we generated a default one!");
    }

    let ascii = r#"______                          _
|  _  \                        | |
| | | | _ __  ___ __   __ __ _ | |__
| | | || '__|/ _ \\ \ / // _` || '_ \
| |/ / | |  | (_) |\ V /| (_| || | | |
|___/  |_|   \___/  \_/  \__,_||_| |_|

                                      "#;

    println!("{}", ascii);
    println!("Now up and running!");
    launch_webserver().await
}
