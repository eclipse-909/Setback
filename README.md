# Setback
![icon](./setback/assets/images/cardsIcon.png)

This mono-repo contains the source code for all Setback programs, server and client side. See the READMEs below for more detail on each workspace.

- [keepalive](keepalive/README.md) – CPU waster to prevent Oracle from reclaiming the VPS.
- [libsetback](libsetback/README.md) – Library for shared code.
- [setback](setback/README.md) – Setback game client.
- [setback_server](setback_server/README.md) – Dedicated game server to host multiplayer games.
- [setback_website](setback_website/README.md) – Website and public API.
- [setbackctl](setbackctl/README.md) – Controller for setbackd.

The Setback game client aims to be cross-platform.

The backend services use docker compose to run the server and website. Collectively the backend is often referred to as setbackd in this project. Dokploy is used to deploy the 
backend to the cloud.