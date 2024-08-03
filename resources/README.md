A library of data types, functions, and other resources used by the
Setback server and the Godot game client. Most resources are used
as game state, for serialization and internet transmission, and use C
representation for interop with the Godot game client via GDExtension.
It also respects connections between different architectures by using
u16s instead of usizes.