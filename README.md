# drovah
Simple, fast, standalone CI written in Rust

This project is entirely for fun and building on my rust knowledge,
however, was created for the purpose of being an ultra lightweight (and fast!) implementation of a continuous integration service.

Please note that it is still very much WIP!
 
## Setup

### Drovah
Prerequisites:
- git

Compile using ``cargo build --release`` 
and run the built artifact found in ``target/release/``

Optionally you can use a Rocket.toml to [alter the configuration of the web server](https://rocket.rs/v0.4/guide/configuration/#rockettoml)

An example ``Rocket.toml`` file:
```toml
[production]
address = "127.0.0.1"
port = 8001
```

### In your project
Simple create a ``.drovah`` file in the root of your project

Example ``.drovah`` file

```toml
[build]
commands = ["gradle clean build"]

[archive]
files = ["build/libs/someproject-1.0-SNAPSHOT.jar"]
```

#### Explanation of configuration options
``build`` must be an array of strings which will represent your commands, they are run in order.

(OPTIONAL) ``archive`` must be an array of strings representing the relative path of your project

## Adding tracked projects

Once drovah is running, simply type ``new <url>`` where url is the git url of the project

## Current Features

- Stupid simple configuration
- Supports whatever build tool you want
- Absolute minimal resource usage
- Webhook for automated builds
- Successful build archival
- Latest build retrieval through ``http://host:port/<project>/latest``
- Specific file retrieval through ``http://host:port/<project>/specific/<filename>``

## Commands

| Command | Description |
| --------------- | ---------------- |
| new \<url> | Creates new tracked repo
| remove \<project name> | Removes tracked project
| build \<project name> | Manually builds project

## Things to come:
- [ ] Wildcard matching for file archival
- [ ] Build status banner(s)
- [ ] Numerous build file(s) archival (at the moment it's only one build)
- [ ] Configuration to enable/disable stuff
- [ ] More useful unit tests
- [ ] Cleanup stdout
- [ ] Post archival commands
- [ ] Logging
- [ ] Benchmark
- [ ] Inspect for security
- [ ] Frontend?