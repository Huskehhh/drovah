use drovah::{launch_rocket, Config};
use std::{io, process, thread};

fn main() {
    thread::spawn(|| {
        launch_rocket();
    });
    loop {
        let mut input = String::new();

        if let Ok(_) = io::stdin().read_line(&mut input) {
            thread::spawn(move || {
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
            });
        } else {
            println!("Please enter an actual input!");
        }
    }
}
