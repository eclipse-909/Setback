This program runs a TCP server for the Setback game. It handles client communication
with the Godot game client by using the resources workspace, which is also used by the
Godot game client for easier compatibility. It manages game state in memory and doesn't
use persistent storage. It handles game logic and is the authority if clients have conflicting
information.