# Setback
This repo contains the source code for all Setback programs, server and client side. See the READMEs below for more detail on each workspace.

- [libsetback](libsetback/README.md) - Library for shared code.
- [setback](setback/README.md) - Setback game client.
- [setback_server](setback_server/README.md) - Instantiable dedicated game server to host multiplayer games.
- [setback_website](setback_website/README.md) - Website and public API.
- [setbackctl](setbackctl/README.md) - Controller for [setbackd](setbackd/README.md).
- [setbackd](setbackd/README.md) - Daemon for managing [setback_server](setback_server/README.md).

The Setback game client aims to be cross-platform.

The backend services must run in a Linux environment and depends on core components like systemd and FHS.
It is recommended to use a virtual machine to run the backend services.
A [Distrobox](./distrobox.ini) file has been created for convenience.
On Windows you might be able to use WSL, but you have to enable systemd.
If you use Mac OS, don't.