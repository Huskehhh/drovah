use drovah::{launch_rocket, Config, DrovahConfig};
use std::path::Path;
use std::{fs, io, process, thread};

fn main() {
    let path = Path::new("Drovah.toml");

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

    thread::spawn(|| {
        launch_rocket(drovah_config);
    });
    loop {
        let mut input = String::new();

        if let Ok(_) = io::stdin().read_line(&mut input) {
            thread::spawn(move || {
                if !input.is_empty() {
                    let trimmed = input.trim_end().to_string();
                    let args: Vec<&str> = trimmed.split(" ").collect();

                    let config = Config::new(args).unwrap_or_else(|err| {
                        eprintln!("Problem parsing arguments: {}", err);
                        process::exit(1);
                    });

                    if let Err(e) = drovah::run(config) {
                        eprintln!("Application error: {}", e);
                        process::exit(1);
                    }
                } else {
                    println!("Please enter an actual input!");
                }
            });
        }
    }
}
