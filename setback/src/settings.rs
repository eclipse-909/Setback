use {
	std::{
		fs::File,
		io::Write,
		path::Path
	},
	bevy::{
		prelude::*,
		ecs::relationship::RelatedSpawnerCommands
	},
	crate::{
		states::*,
		constants::*,
		resources::*,
		systems::*
	},
	keyring::Entry
};

pub struct SettingsPlugin;

impl UiScenePlugin for SettingsPlugin {
	const SCENE: UiScene = UiScene::Settings;
}

impl Plugin for SettingsPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_systems(OnEnter(Self::SCENE), on_enter)
			.add_systems(OnExit(Self::SCENE), Self::on_exit);
	}
}

fn on_enter(mut commands: Commands, asset_server: Res<AssetServer>) {
	info!("Entering Settings");
	let times: Handle<Font> = asset_server.load(TIMES_NEW_ROMAN);
	menu_background(&mut commands, times, |commands| {
		settings_menu(commands);
	});
}

pub fn settings_menu(commands: &mut RelatedSpawnerCommands<ChildOf>) {
	todo!()
}

fn new_checkbox() -> impl Bundle {
	todo!()
}

fn new_spinbox() -> impl Bundle {
	todo!()
}

fn new_slider() -> impl Bundle {
	todo!()
}

fn new_keybind() -> impl Bundle {
	todo!()
}

fn save_settings(
	trigger: Trigger<Pointer<Click>>,
	game_settings: ResMut<GameSettings>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	todo!()
}

fn delete_credentials() {
	let path = Path::new(USR_CRD);
	match File::options().truncate(true).write(true).open(path) {
		Ok(mut file) => {
			if let Err(err) = file.write_all(b"") {
				warn!("Error truncating usr.crd: {}", err);
			}
		}
		Err(err) => {
			warn!("Error opening usr.crd: {}", err);
		}
	}
	if let Err(err) = Entry::new(KEYRING_PASSWORD, whoami::username().as_str()).and_then(|entry| entry.delete_credential()) {
		warn!("Error deleting password in keyring: {}", err);
	}
	info!("Deleted credentials");
}

fn logout(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	server: Option<Res<ServerConn>>,
	logout_button: Query<&mut Node, With<LogoutButton>>,
	mut logout_message: Query<(&mut Text, &ChildOf), With<LogoutMessage>>,
	logout_message_node: Query<&mut Node>,
	scene: Res<State<UiScene>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	delete_credentials();
	if server.is_some() && matches!(scene.get(), UiScene::Settings) {
		commands.remove_resource::<ServerConn>();
	}
	let mut logout_button = logout_button.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let (mut logout_message, parent) = logout_message.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let mut logout_message_node = logout_message_node.get(parent.parent()).expect(SINGLE_COMPONENT_QUERY_ERROR);
	todo!("disable and gray out the logout button");
	logout_message.0 = String::from(if server.is_some() {
		if matches!(scene.get(), UiScene::Settings) {
			commands.remove_resource::<ServerConn>();
			info!("Disconnected from server");
			"You have logged out"
		} else {
			"You will be logged out once you disconnect"
		}
	} else {
		"You have logged out"
	});
	todo!("display logout message")
}

#[derive(Component)]
struct RestartMessage;

#[derive(Component)]
struct Vsync;

#[derive(Component)]
struct MaxFrameRate;

#[derive(Component)]
struct Fullscreen;

#[derive(Component)]
struct DefaultMonitor;

#[derive(Component)]
struct MonitorIndex;

#[derive(Component)]
struct MasterVolume;

#[derive(Component)]
struct MusicVolume;

#[derive(Component)]
struct EffectsVolume;

#[derive(Component)]
struct PassKeybind(Vec<KeyCode>);

#[derive(Component)]
struct Bid2Keybind(Vec<KeyCode>);

#[derive(Component)]
struct Bid3Keybind(Vec<KeyCode>);

#[derive(Component)]
struct Bid4Keybind(Vec<KeyCode>);

#[derive(Component)]
struct PlayCard1Keybind(Vec<KeyCode>);

#[derive(Component)]
struct PlayCard2Keybind(Vec<KeyCode>);

#[derive(Component)]
struct PlayCard3Keybind(Vec<KeyCode>);

#[derive(Component)]
struct PlayCard4Keybind(Vec<KeyCode>);

#[derive(Component)]
struct PlayCard5Keybind(Vec<KeyCode>);

#[derive(Component)]
struct PlayCard6Keybind(Vec<KeyCode>);

#[derive(Component)]
struct ToggleSettings(Vec<KeyCode>);

#[derive(Component)]
struct SaveLoginCredentials;

#[derive(Component)]
#[require(Node)]
struct LogoutButton;

#[derive(Component)]
#[require(Text, ChildOf)]
struct LogoutMessage;