# Setback
![icon](./setback/assets/images/cardsIcon.png)

This mono-repo contains the source code for all Setback programs, server and client side. See the READMEs below for more detail on each workspace.

- [api](api/README.md) – REST API with public and private endpoints.
- [libsetback](libsetback/README.md) – Library for shared code for all packages.
- [server](server/README.md) – Dedicated game server to host multiplayer games.
- [setback](setback/README.md) – Setback game client.
- [website](website/README.md) – Frontend website for Setback and account management.

The Setback game client aims to be cross-platform. Primarily targeting desktop and web, and maybe expanding to mobile.

The backend services use docker compose to run the server and website. Collectively the backend is often referred to as setbackd in this project. Dokploy is used to deploy the backend to the cloud.

## Developer Setup
The following should be installed
* [Rust](https://rust-lang.org/tools/install/)
  * Nightly compiler
  * llvm-tools-preview
  * cargo-llvm-cov
* [Just](https://just.systems/man/en/installation.html)
* [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/) and [system dependencies](https://dioxuslabs.com/learn/0.7/getting_started/)
* [Docker](https://docs.docker.com/engine/install/)