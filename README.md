# Setback
![icon](./setback/assets/images/cardsIcon.png)

This mono-repo contains the source code for all Setback programs, server and client side. See the READMEs below for more detail on each workspace.

- [libsetback](libsetback/README.md) – Library for shared code for all packages.
- [setback](setback/README.md) – Setback game client.
- [setback_server](setback_server/README.md) – Dedicated game server to host multiplayer games.
- [setback_api](setback_api/README.md) – Website and public API.
- [setbackctl](setbackctl/README.md) – Controller for setbackd.

The Setback game client aims to be cross-platform. Primarily targeting desktop and web, and maybe expanding to mobile.

The backend services use docker compose to run the server and website. Collectively the backend is often referred to as setbackd in this project. Dokploy is used to deploy the 
backend to the cloud.