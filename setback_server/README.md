# setback_server
This is the dedicated server that will host multiplayer games for Setback.
## Network
The server will use the following ports:
- ACCEPT_PORT - this port is used to accept clients who are connecting to the server. Once a client has been accepted and authenticated, the server will send them the CLIENT_PORT that is being used by the process. The client will disconnect from the ACCEPT_PORT and connect to the CLIENT_PORT. If 2 servers are running, only 1 will have the ACCEPT_PORT open because only 1 of the servers should accept new connections. The purpose of this is to gracefully shutdown one server and start another server (presumably for an update). The old server stops accepting clients and the new server takes over.
- CLIENT_PORT_1 - after a client has been accepted and authorized by the server, they are sent a CLIENT_PORT to connect to. Once the client connects with this port, all further communication between the client and server will take place on this port.
- CLIENT_PORT_2 - ibid
## Async Tasks
- One task is used to accept admin connections via a Unix socket and communicate with that client. Only 1 admin may connect at a time so the task will not accept new connections after 1 has connected. Once that admin client disconnects, it will begin accepting connections again. One task can be used for all of this.
- One task is used to accept new connections.
- When a client first connects to the server, a task will be spawned to communicate with the client. This task is responsible for authenticating the client and sending back the port that the client should connect to. There will be one task per client.
- One task is used to accept connections from clients who have authenticated.
- When a client connects to the server, a task will be spawned to communicate with the client. This task is responsible for communication with the client until they disconnect. There will be one task per client.