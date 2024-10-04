mod main_menu;
mod setup;
mod slider_u8;
mod setup_player;
mod spinbox;
mod button;
mod game;

use {
	bevy::prelude::*,
	sickle_ui::prelude::*
};

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((button::ButtonPlugin, slider_u8::SliderU8Plugin, spinbox::SpinBoxPlugin, setup_player::SetupPlayerPlugin))
			.add_plugins((main_menu::MainMenuUIPlugin, setup::SetupUIPlugin));
	}
}

mod scene {
	use {
		crate::SceneType,
		bevy::prelude::*
	};
	
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
		fn build(&self, app: &mut App) {app.add_systems(OnExit(Self::SCENE), Self::de_spawn);}
		fn de_spawn(mut commands: Commands, root: Query<Entity, With<RootUINode>>) {commands.entity(root.single()).despawn_recursive();}
	}
}

trait OverrideTheme: Sized + Component + UiContext + DefaultTheme {
	fn overriding_theme0() -> Theme<Self>;
	fn overriding_theme1() -> Theme<Self> {Self::overriding_theme0()}
	fn overriding_style0(style_builder: &mut StyleBuilder, theme_data: &ThemeData);
	fn overriding_style1(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {Self::overriding_style0(style_builder, theme_data)}
}

#[derive(Component)]
struct EntityReference(Entity);