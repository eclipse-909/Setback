# admin_controller
This is a CLI app that interfaces with the server and has access to admin commands. Upon startup, the app will try to detect the running servers. If none are running, the app will display that message and exit. If one is running, it will immediately try connecting to and interfacing with that server. If two are running, you will be prompted to select which server to interface into.
## Commands
Commands always accessible.
- help - displays all commands, kind of like this README.
- exit - disconnects from the server and stops the CLI app.
- login - prompts you for the admin username and password. Attempts to authenticate as an admin with the server. If authenticated, you will have access to admin commands.
- games - displays the number of active games.
- players - displays the number of connected players.
- ls - lists the servers running (there should always be 0, 1, or 2 servers running)
- chsrv - lists the servers running. If multiple servers are running, you will be prompted to select which server to interface into.
## Admin Commands
  Commands only accessible after authenticating.
- close - stops the server from accepting new connections. The server will automatically shutdown when all players have disconnected. All players will receive a message that the server is closing. The game client will handle this by attempting to reconnect to the server that is accepting new connections, but only if the client is not in a setup/game, and there is a running server accepting new connections.
- reopen - allows the server to accept new connections again. The server starts open so this is not needed when the server is first started. This should only be used if you closed the server and want to reopen it. All players will receive a message that the server is reopening. The game client will handle this by not disconnecting from the server.  
- kill - immediately kills the running server process.
- logout - deauthenticates admin access.
## Network
The server uses ADMIN_PORT_1 or ADMIN_PORT_2. Since two servers can run at a time, a server will use one of these two ports, and are distinguished by them. These ports are blocked by the firewall for incoming traffic. This CLI app runs on the same machine as the servers so the connection will be over localhost.