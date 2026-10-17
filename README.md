# Setback
![icon](./setback/assets/images/cardsIcon.png)

This mono-repo contains the source code for all Setback programs server and client side.

```
/Setback
├── api – REST API
├── libsetback – Shared library for all packages
├── local – Local development files
├── server – Dedicated multiplayer game server
├── setback – Game client (web and desktop)
├── website – Website for Setback, account management and web game client
├── .env.template - Template .env file you should copy to .env.local
├── docker-compose.yml - DEV and PROD compose file
└── Dockerfile - DEV and PROD dockerfile
```

The Setback game client aims to be cross-platform. Primarily targeting desktop and web, and maybe expanding to mobile.

The backend services use docker compose to run the server and website. Collectively the backend is often referred to as setbackd in this project. Dokploy is used to deploy the backend to the cloud.

## Developer Setup
### Installations
The following should be installed, some are optional
* [Rust](https://rust-lang.org/tools/install/)
  * Nightly compiler
  * llvm-tools-preview
  * cargo-llvm-cov
* [Just](https://just.systems/man/en/installation.html)
* [Docker](https://docs.docker.com/engine/install/) (or similar i.e. Podman)
* [SQLx CLI](https://github.com/transact-rs/sqlx/blob/main/sqlx-cli/README.md)
* Any PostgreSql client
* [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/) and [system dependencies](https://dioxuslabs.com/learn/0.7/getting_started/)
* [Binaryen](https://github.com/WebAssembly/binaryen)

### IDE Setup
Add your `.<ide>` directory in the [.gitignore](./.gitignore).

For debuggers, make sure you specify `--profile debug` in cargo commands because the default profile (`dev`) will have light optimizations and limited debug information.