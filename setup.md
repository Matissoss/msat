# Downloading msat

**msat** can be downloaded with two ways

## Downloading compiled binaries (Recommended)

Easiest way to install **msat** is through compiled binaries. Head into releases section and select version that interest you.

## Build From Source

To build **msat** from source, recommended way is to use `ci/build.sh` script. This approach requires following dependencies:

- bash compatible shell/terminal
- cargo (Rust build tool)
- tar (compressing tool found in most Linux Distros)
- git (optional)

(optional) clone repo using git: `git clone https://github.com/Matissoss/msat`.

Go into `ci` directory and use command `sh build.sh` or `./build.sh`

# Setup

When you will uncompress your build, it will be mostly deployment ready, but it is better to check.

***MAKE SURE THAT***
- there is `data` directory there
- there is `web` directory there

Then create `config.toml` file in `data` directory.

Example configuration:
```toml
# Password for testing purposes
password="test"
# CHOOSE ONE
language="Polish|English"

[http_server]
port = 8000
ip = "127.0.0.1"
# max connections server can have at once
max_connections = 100
# How much seconds will server wait with next requests
max_timeout_seconds = 10

# application_server has no usecase rignt now, but this config is required for msat to work :)
[application_server]
port = 8888
ip = "127.0.0.1"
max_connections = 100
max_timeout_seconds = 10
```

# Startup

Now you can launch `app_server` and `admin_dashboard` executables.
