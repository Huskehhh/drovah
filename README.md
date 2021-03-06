# drovah

![build status](https://github.com/Huskehhh/drovah/actions/workflows/ci.yml/badge.svg)
![docker pulls](https://img.shields.io/docker/pulls/jordynnewnham/drovah)

Simple, fast, standalone continuous integration service written in Rust

This project is entirely for fun and building on my rust knowledge, however, was created for the purpose of being an ultra lightweight, fast and painless implementation of a continuous integration service.

drovah comes with a basic frontend that will allow you to download any archived files as well as view past build statuses!

## Development

[You can follow the development of new features here](https://github.com/Huskehhh/drovah/projects/3)

If you want a feature I don't current have planned, please create an issue!

## Current Features

- Stupid simple configuration
- Supports whatever build tool you want
- Simple frontend supporting viewing previous build statuses and downloading per build archived files
- Minimal resource usage (the demo above is running on 6MB of RAM)
- GitHub Webhook support for automation (includes the use of secure token to ensure security)
- Successful build archival (numerous builds)
- Support for post archival actions

## Setup

Prerequisites:

- Rust
- git
- MySQL server
- npm

## From source

### backend

1. ``cd backend``
2. ``cargo build --release``
3. Done.

### Frontend

1. ``cd frontend``
2. ``npm install``
3. ``npm run build``
4. Done.

## From docker

First off I apologise, it's a bit more work than I would like setting this up, and I aim to improve that in the future!

1. Pull the available image [here](https://hub.docker.com/repository/docker/jordynnewnham/drovah)
2. Refer to ``MySQL setup``
3. Set required ENV variables and run!

## Configuration

You have two _required_ settings to configure, the database and the github secret! Although optionally you can change the bind address

Simply create a ``.env`` containing the following

```env
DATABASE_URL=mysql://user:pass@localhost:3306
GITHUB_SECRET=secretgoeshere
```

Note for ``GITHUB_SECRET``, use something like ``ruby -rsecurerandom -e 'puts SecureRandom.hex(20)'`` [to generate the secret](https://docs.github.com/en/free-pro-team@latest/developers/webhooks-and-events/securing-your-webhooks#setting-your-secret-token)

And if you wish to change the bind address, add ``BIND_ADDRESS=127.0.0.1:8080``

### MySQL setup

1. Install [diesel_cli](https://github.com/diesel-rs/diesel/)
2. Run ``diesel migration run`` in the `backend` directory
3. Done

### In your project

Simple create a ``.drovah`` file in the root of your project

Example ``.drovah`` file

```toml
[build]
commands = ["gradle clean build"]

[archive]
files = ["build/libs/someproject-"]
append_buildnumber = true

[postarchive]
commands = ["echo 'woohoo' >> somefile"]
```

#### Explanation of configuration options

``build`` must be an array of strings which will represent your commands, they are run in order.

(OPTIONAL SECTION) ``archive``

``files`` must be an array of strings containing path/pattern of files, relative path of your project. This will attempt to match the filename, eg, the above ``[archive]`` configuration will match both of these files

``append_buildnumber`` must be a boolean, this option just applies the current build number to the final archived files

- 'build/libs/someproject-1.1.jar'
- 'build/libs/someproject-wahoo.txt'

(OPTIONAL SECTION) ``postarchive``

``commands`` must be an array of strings which will represent commands to be run AFTER successful builds, they are run in order. The running context of these commands is the drovah binary location.

## Managing projects

Just ``git clone <repo>`` in the ``data/projects/`` folder and insert a project to the ``projects`` table of the database, and then webhooks will be supported instantly

Similarly, just remove the folders you no longer want to track

## Webhook

The webhook by default is available at ``http://<host>:<port>/webhook``

This webhook is targetted at GitHub, and can be set up using the ``application/json`` payload.

It will also require a secret which can be set locally through the ``GITHUB_SECRET`` environment variable

If you want to build from some other source, here's an example payload

```json
{
    "repository": {
        "name": "drovah"
    }
}
```

This will attempt to build the ``drovah`` project, if ``data/projects/drovah/`` does not exist, or doesn't contain a ``.drovah`` file, the build will fail

Note when removing a project, also remove it from the database!
