# drovah ![build status](https://ci.husk.pro/drovah/badge)
Simple, fast, standalone CI written in Rust

This project is entirely for fun and building on my rust knowledge,
however, was created for the purpose of being an ultra lightweight (and fast!) implementation of a continuous integration service.

Please note that it is still very much WIP!
 
## Setup

### drovah
Prerequisites:
- git
- MongoDB server

Compile using ``cargo build --release`` 
and run the built artifact found in ``target/release/``

#### drovah configuration:
A ``Drovah.toml`` file will be created automatically, and will contain the following:

```toml
[mongo]
mongo_connection_string = "mongodb://localhost:27017"
mongo_db = "drovah"
```

Mongo is used in order to store data related to builds!

#### Webserver configuration:

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
files = ["build/libs/someproject-*.jar"]

[postarchive]
commands = ["echo 'woohoo' >> somefile"]
```

#### Explanation of configuration options
``build`` must be an array of strings which will represent your commands, they are run in order.

(OPTIONAL) ``archive`` must be an array of strings containing path/pattern of files, relative path of your project

(OPTIONAL) ``postarchive`` must be an array of strings which will represent commands to be run AFTER successful builds, they are run in order.

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
- Build status banner retrievable through ``http://host:port/<project>/badge``

## Commands

| Command | Description |
| --------------- | ---------------- |
| new \<url> | Creates new tracked repo
| remove \<project name> | Removes tracked project

### Note: if you wish to disable the build status badge, please also remove the data from the Mongo collection manually

## Manually managing projects

Just ``git clone <repo>`` in the data/projects/ folder, and then webhooks will be supported instantly

Similarly, just remove the folders you no longer want to track

## Things to come:
- [ ] Numerous build file(s) archival (at the moment it's only one build)
- [ ] More config options to enable/disable stuff
- [ ] Swap to async mongo + rocket
- [ ] Cleanup stdout
- [ ] Build logging to file
- [ ] Benchmark
- [ ] Inspect for security
- [ ] Frontend