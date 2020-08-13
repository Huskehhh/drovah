use drovah::{launch_rocket, Config, DrovahConfig};
use std::path::Path;
use std::{fs, io, thread};

fn main() {
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

        println!("No 'Drovah.toml' found, so we generated one, please restart the program.");
    }

    let conf_str = fs::read_to_string(path).unwrap();
    let drovah_config: DrovahConfig = toml::from_str(&conf_str).unwrap();

    // Launch rocket
    thread::spawn(|| {
        launch_rocket(drovah_config).launch();
    });

    // Continually listen for user input
    loop {
        let mut input = String::new();

        if let Ok(_) = io::stdin().read_line(&mut input) {
            thread::spawn(move || {
                if !input.is_empty() {
                    let trimmed = input.trim_end().to_string();
                    let args: Vec<&str> = trimmed.split(" ").collect();

                    if let Ok(config) = Config::new(args) {
                        if let Err(e) = drovah::run(config) {
                            eprintln!("Application error: {}", e);
                        }
                    }
                } else {
                    println!("Please enter an actual input!");
                }
            });
        }
    }
}
