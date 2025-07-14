use {
	bevy::prelude::*,
	crate::components::*
};

#[derive(Default, Clone, Debug, Eq, Hash, PartialEq, States)]
pub enum UiScene {
	#[default]
	MainMenu,
	Multiplayer,
	Settings,
	Setup,
	Game
}

pub trait UiScenePlugin: Plugin {
	const SCENE: UiScene;

	fn on_exit(mut commands: Commands, root_node_entities: Query<Entity, With<RootNode>>) {
		commands.entity(root_node_entities.single().unwrap()).despawn()
	}
}

#[derive(Default, Clone, Debug, Eq, Hash, PartialEq, States)]
pub enum ConnectionType {
	#[default]
	Singleplayer,
	Host,
	Client
}