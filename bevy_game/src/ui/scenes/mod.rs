mod main_menu;
mod setup;
mod game;

use {
	crate::SceneType,
	main_menu::MainMenuUIPlugin,
	bevy::prelude::*
};

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((MainMenuUIPlugin));
	}
}

///Mark the root node of the UI hierarchy with this to despawn it when changing scenes.
#[derive(Component)]
pub struct RootUINode;

///Spawns and de-spawns the implementing struct based on the given [Self::SCENE].
///Implementers must make their own spawn function like this:
///```
///fn spawn(params: impl SystemParam) {/* ... */}
///```
///And it must be added to the plugin in the build function like this:
///```
///app.add_systems(OnEnter(Self::SCENE), Self::spawn);
///```
pub trait SceneUI: Plugin {
	///The scene with which this UI will spawn on enter and de-spawn on exit.
	const SCENE: SceneType;
	///Call this method when implementing Plugin for the scene.
	fn build(&self, app: &mut App) {app.add_systems(OnExit(Self::SCENE), de_spawn);}
}

fn de_spawn(mut commands: Commands, root: Query<Entity, With<RootUINode>>) {root.iter().for_each(|r| {commands.entity(r).despawn_recursive();})}