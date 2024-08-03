mod ui;

use {
	crate::ui::{MainMenuUIPlugin, SetupUIPlugin},
	bevy::{
		prelude::*,
		window::{PresentMode, WindowMode, EnabledButtons, WindowTheme},
		winit::WinitSettings
	},
	bevy_inspector_egui::quick::WorldInspectorPlugin,
	sickle_ui::SickleUiPlugin
};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				present_mode: PresentMode::AutoVsync,
				mode: WindowMode::BorderlessFullscreen,
				resolution: (1920., 1080.).into(),
				title: "Setback".into(),
				name: Some("setback.app".into()),
				resizable: false,
				enabled_buttons: EnabledButtons {
					minimize: false,
					maximize: false,
					close: false
				},
				decorations: false,
				window_theme: Some(WindowTheme::Dark),
				..default()
			}),
			..default()
		}))
		.add_plugins(SickleUiPlugin)
		.add_plugins(WorldInspectorPlugin::default())//TODO remember to comment this line for release builds
		.add_plugins((MainMenuUIPlugin, SetupUIPlugin))
		.insert_resource(WinitSettings::game())
		.init_state::<SceneType>()
		.init_state::<HostType>()
		.add_event::<SceneChangedEvent>()
		.add_systems(Startup, setup)
		.observe(on_scene_changed)
		.run();
}

fn setup(mut commands: Commands) {commands.spawn(Camera2dBundle::default());}

///Game state which stores what scene is currently active.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum SceneType {
	#[default]
	MainMenu,
	Setup,
	Multiplayer,
	Game,
	Settings
}

///Game state which stores the relation between the player and the type of game they're playing.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum HostType {
	///Not in a game.
	#[default]
	None,
	Singleplayer,
	///Created a multiplayer game as the host.
	Host,
	///Joined a multiplayer game as a client.
	Client
}

#[derive(Event, Clone)]
struct SceneChangedEvent {
	scene: SceneType,
	host: Option<HostType>
}

fn on_scene_changed(
	trigger: Trigger<SceneChangedEvent>,
	curr_scene: Res<State<SceneType>>,
	mut next_scene: ResMut<NextState<SceneType>>,
	mut next_host: ResMut<NextState<HostType>>
) {
	let event = trigger.event();
	if event.scene == *curr_scene.get() {return;}
	if let Some(host) = event.host.clone() {
		next_host.set(host);
	}
	next_scene.set(event.scene.clone());
}