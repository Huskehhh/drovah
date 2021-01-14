extern crate actix_web;
extern crate dotenv;

use dotenv::dotenv;
use drovah::launch_webserver;

use std::path::Path;
use std::{fs, io};

#[actix_rt::main]
async fn main() -> io::Result<()> {
    dotenv().ok();

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
