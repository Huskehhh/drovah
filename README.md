# drovah ![build status](https://ci.husk.pro/drovah/badge)
Simple, fast, standalone continuous integration service written in Rust

This project is entirely for fun and building on my rust knowledge,
however, was created for the purpose of being an ultra lightweight, fast and painless implementation of a continuous integration service.

Please note that drovah is still very much WIP!

## Demo

[Demo available here](https://ci.husk.pro/)

## Development
[You can follow the development of new features here](https://github.com/Huskehhh/drovah/projects/2)

If you want a feature I don't current have planned, please create an issue

## Current Features

- Simple configuration
- Supports whatever build tool you want
- Simple frontend (Latest file retrieval + build status. More coming soon)
- Minimal resource usage
- Webhook for automated builds/archival/status
- Successful build archival (numerous builds)
- Support for post archival actions
- Latest build retrieval through ``http://host:port/<project>/latest``
- Build status banner retrievable through ``http://host:port/<project>/badge``

## Setup

### drovah
Prerequisites:
- git
- MongoDB server

Clone the repo and run using ``cargo run --release`` - default server will be running at http://localhost:8000

For production use, I recommend binding drovah to localhost and creating a reverse proxy from nginx/apache.

#### drovah configuration:
A ``drovah.toml`` file will be created automatically, and will contain the following:

```toml
[web]
address = "127.0.0.1:8000"

[mongo]
mongo_connection_string = "mongodb://localhost:27017"
mongo_db = "drovah"
```

``address`` is used to specify the ip and port to bind the webserver to

``mongo_connection_string`` is used to specify the connection string for your mongodb server
``mongo_db`` is used to specify the database to use to store data in

### In your project
Simple create a ``.drovah`` file in the root of your project

Example ``.drovah`` file

```toml
[build]
commands = ["gradle clean build"]

[archive]
files = ["build/libs/someproject-"]

[postarchive]
commands = ["echo 'woohoo' >> somefile"]
```

#### Explanation of configuration options
``build`` must be an array of strings which will represent your commands, they are run in order.

(OPTIONAL) ``archive`` must be an array of strings containing path/pattern of files, relative path of your project. This will attempt to match the filename, eg, the above ``[archive]`` configuration will match both of these files

- 'build/libs/someproject-1.1.jar'
- 'build/libs/someproject-wahoo.txt'


(OPTIONAL) ``postarchive`` must be an array of strings which will represent commands to be run AFTER successful builds, they are run in order. The running context of these commands is the drovah binary location.

## Managing projects

Just ``git clone <repo>`` in the ``data/projects/`` folder, and then webhooks will be supported instantly

Similarly, just remove the folders you no longer want to track

## Webhook

The webhook by default is available at http://localhost:8000/webhook

This webhook supports GitHub, and can be set up using the ``application/json`` payload.

If you want to build from some other source, here's an example payload
```json
{
    "repository": {
        "name": "drovah"
    }
}
```

This will attempt to build the ``drovah`` project, if ``data/projects/drovah/`` does not exist, or doesn't contain a ``.drovah`` file, the build will fail

### Note if you wish to remove the build status badge when removing a project, you will need to do so manually in the database
