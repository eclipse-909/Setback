use {
	std::fs,
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
	keyring::Entry,
	libsetback::SETBACK_VERSION
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

fn on_enter(mut commands: Commands, asset_server: Res<AssetServer>, game_settings: Res<GameSettings>) {
	info!("Entering Settings");
	menu_background(&mut commands, asset_server.load(TIMES_NEW_ROMAN), |commands| {
		settings_menu(commands, asset_server.load(GEORGIA), asset_server.load(GEORGIA_BOLD), game_settings);
	});
}

//TODO
// Fill in the styling based on the html using the main menu as an example.
// Use font everywhere except for where font_bold is already being used.
// Add CursorComponent to each of the interactable items and use "cursor: Handle<Image>" in the same way the main menu does.

///This function is used to create the settings menu for the settings scene and the settings menu in-game
pub fn settings_menu(commands: &mut RelatedSpawnerCommands<ChildOf>, font: Handle<Font>, font_bold: Handle<Font>, game_settings: Res<GameSettings>) {
	//settings container
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		//settings header
		commands.spawn((
			Node {
				..default()
			}
		)).with_children(|commands| {
			//back button
			commands.spawn((
				Node {
					..default()
				},
				Button,
				children![(
					Text::new("◀ Back")
				)]
			));
			//settings title
			commands.spawn((
				Node {
					..default()
				},
				children![(
					Text::new("Settings")
				)]
			));
			//spacer
			commands.spawn(Node {
				..default()
			});
		});
		//tab container
		commands.spawn((
			Node {
				..default()
			}
		)).with_children(|commands| {
			commands.spawn((
				Node {
					..default()
				},
				Button,
				TabButton {tab_fn: video_tab, font: font.clone()},
				children![(
					TabLabel::Video,
					Text::new("video"),
					TextFont::from_font(font_bold.clone())
				)]
			)).observe(click_tab);
			commands.spawn((
				Node {
					..default()
				},
				Button,
				TabButton {tab_fn: audio_tab, font: font.clone()},
				children![(
					TabLabel::Audio,
					Text::new("audio"),
					TextFont::from_font(font.clone())
				)]
			)).observe(click_tab);
			commands.spawn((
				Node {
					..default()
				},
				Button,
				TabButton {tab_fn: keybinds_tab, font: font_bold},
				children![(
					TabLabel::Keybinds,
					Text::new("keybinds"),
					TextFont::from_font(font.clone())
				)]
			)).observe(click_tab);
			commands.spawn((
				Node {
					..default()
				},
				Button,
				TabButton {tab_fn: account_tab, font: font.clone()},
				children![(
					TabLabel::Account,
					Text::new("account"),
					TextFont::from_font(font.clone())
				)]
			)).observe(click_tab);
		});
		//tab content
		commands.spawn((
			Node {
				..default()
			},
			TabContent
		)).with_children(|commands| {
			//video tab is default tab
			new_tab(commands, font, game_settings, video_tab);
		});
		//version text
		commands.spawn((
			Node {
				..default()
			},
			children![(
				Text::new(format!("Version {SETBACK_VERSION}"))
			)]
		));
		//modal overlay
		commands.spawn((
			Node {
				..default()
			}
		)).with_children(|commands| {
			//keybind modal
			commands.spawn((
				Node {
					..default()
				}
			)).with_children(|commands| {
				//modal title
				commands.spawn((
					Node {
						..default()
					},
					children![(
						Text::new("Rebind Key")
					)]
				));
				//modal instruction
				commands.spawn((
					Node {
						..default()
					},
					children![(
						Text::new("Press the key(s) you want to bind for this action:")
					)]
				));
				//key display
				commands.spawn((
					Node {
						..default()
					},
					children![(
						Text::new("Press any key...")
					)]
				));
				//modal buttons
				commands.spawn((
					Node {
						..default()
					}
				)).with_children(|commands| {
					//cancel button
					commands.spawn((
						Node {
							..default()
						},
						children![(
							Text::new("Cancel")
						)]
					));
					//apply button
					commands.spawn((
						Node {
							..default()
						},
						children![(
							Text::new("Apply Binding")
						)]
					));
				});
			});
		});
	});
}

type TabFn = fn(commands: &mut RelatedSpawnerCommands<ChildOf>, font: Handle<Font>, game_settings: &GameSettings);
type InteractiveFn<C: SettingsItem> = fn(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, font: Handle<Font>, game_settings: &GameSettings);

fn new_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, font: Handle<Font>, game_settings: Res<GameSettings>, tab: TabFn) {
	//tab panel
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		//setting group
		commands.spawn((
			Node {
				..default()
			}
		)).with_children(|commands| {
			//specific tab
			tab(commands, font, game_settings.as_ref());
		});
	});
}

fn video_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, font: Handle<Font>, game_settings: &GameSettings) {
	new_setting_item(commands, "VSync", VSync, font.clone(), game_settings, new_checkbox);
	new_setting_item(commands, "Max Frame Rate", MaxFrameRate, font.clone(), game_settings, new_spinbox);
	new_setting_item(commands, "Fullscreen", Fullscreen, font.clone(), game_settings, new_checkbox);
	new_setting_item(commands, "Default Monitor", DefaultMonitor, font.clone(), game_settings, new_dropdown);
	new_setting_item(commands, "Monitor Index", MonitorIndex, font, game_settings, new_spinbox);
}

fn audio_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, font: Handle<Font>, game_settings: &GameSettings) {
	new_setting_item(commands, "Master Volume", MasterVolume, font.clone(), game_settings, new_slider);
	new_setting_item(commands, "Music Volume", MusicVolume, font.clone(), game_settings, new_slider);
	new_setting_item(commands, "Effects Volume", EffectsVolume, font, game_settings, new_slider);
}

fn keybinds_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, font_bold: Handle<Font>, game_settings: &GameSettings) {
	new_keybind_item(commands, "Pass", PassKeybind(game_settings.bid_0.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Bid 2", Bid2Keybind(game_settings.bid_2.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Bid 3", Bid3Keybind(game_settings.bid_3.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Bid 4", Bid4Keybind(game_settings.bid_4.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Play Card 1", PlayCard1Keybind(game_settings.play_card_1.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Play Card 2", PlayCard2Keybind(game_settings.play_card_2.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Play Card 3", PlayCard3Keybind(game_settings.play_card_3.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Play Card 4", PlayCard4Keybind(game_settings.play_card_4.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Play Card 5", PlayCard5Keybind(game_settings.play_card_5.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Play Card 6", PlayCard6Keybind(game_settings.play_card_6.clone()), game_settings, font_bold.clone());
	new_keybind_item(commands, "Toggle Settings", ToggleSettingsKeybind(game_settings.toggle_settings.clone()), game_settings, font_bold);
}

fn account_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, font: Handle<Font>, game_settings: &GameSettings) {
	new_setting_item(commands, "Save Credentials", SaveLoginCredentials, font.clone(), game_settings, new_checkbox);
	new_setting_item(commands, "Account Actions", LogoutButton, font.clone(), game_settings, new_logout_button);
	//logout message
	commands.spawn((
		LogoutMessage,
		Node {
			..default()
		},
		children![(
			Text::new("You have been logged out successfully.")
		)]
	));
}

fn item_label_and_control<C: SettingsItem>(
	commands: &mut RelatedSpawnerCommands<ChildOf>,
	label: &str,
	marker: C,
	font: Handle<Font>,
	game_settings: &GameSettings,
	interactive: InteractiveFn<C>
) {
	//setting label
	commands.spawn((
		Node {
			..default()
		},
		children![(
			Text::new(label)
		)]
	));
	//setting control
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		//checkbox/spin box/dropdown/slider/keybind
		interactive(commands, marker, font, game_settings);
	});
}

fn new_setting_item<C: SettingsItem>(
	commands: &mut RelatedSpawnerCommands<ChildOf>,
	label: &str,
	marker: C,
	font: Handle<Font>,
	game_settings: &GameSettings,
	interactive: InteractiveFn<C>
) {
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		item_label_and_control(commands, label, marker, font, game_settings, interactive);
	});
}

fn new_keybind_item(
	commands: &mut RelatedSpawnerCommands<ChildOf>,
	label: &str,
	marker: impl SettingsItem,
	game_settings: &GameSettings,
	font_bold: Handle<Font>
) {
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		item_label_and_control(commands, label, marker, font_bold, game_settings, new_keybind);
	});
}

fn new_checkbox<C: SettingsItem<Output=bool>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, font: Handle<Font>, game_settings: &GameSettings) {
	let default_val = C::get_setting(game_settings);
	todo!()
}

fn new_spinbox<C: SettingsItem<Output=f64>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, font: Handle<Font>, game_settings: &GameSettings) {
	todo!()
}

fn new_dropdown<C: SettingsItem>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, font: Handle<Font>, game_settings: &GameSettings) {
	todo!()
}

fn new_slider<C: SettingsItem<Output=f64>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, font_bold: Handle<Font>, game_settings: &GameSettings) {
	todo!()
}

fn new_logout_button<C: SettingsItem<Output=()>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, font: Handle<Font>, game_settings: &GameSettings) {
	todo!()
}

fn new_keybind<C: SettingsItem<Output=Vec<KeyCode>>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, font: Handle<Font>, game_settings: &GameSettings) {
	todo!()
}

fn click_tab(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	tab_content: Query<Entity, With<TabContent>>,
	tab_button: Query<&TabButton>,
	mut tab_labels: Query<(&mut TextFont, &ChildOf), With<TabLabel>>,
	asset_server: Res<AssetServer>,
	game_settings: Res<GameSettings>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	let mut tab_content = commands.entity(tab_content.single().expect(SINGLE_COMPONENT_QUERY_ERROR));
	let tab_button = tab_button.get(trigger.target).expect("Tab button observer should target entity containing TabButton component");
	tab_content.despawn_related::<Children>();
	tab_content.with_children(|commands| {
		(tab_button.tab_fn)(commands, tab_button.font.clone(), game_settings.as_ref());
	});
	
	#[cfg(debug_assertions)]
	let mut changed = false;
	
	for (mut tab_label, child_of) in tab_labels.iter_mut() {
		tab_label.font = if child_of.parent() == trigger.target {
			changed = true;
			asset_server.load(GEORGIA_BOLD)
		} else {
			asset_server.load(GEORGIA)
		};
	}
	debug_assert!(changed, "Tab label font weight was not updated");
}

fn save_settings(
	trigger: Trigger<Pointer<Click>>,
	game_settings: ResMut<GameSettings>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	todo!()
}

fn delete_credentials() {
	if let Err(err) = fs::remove_file(USR_CRD) {
		warn!("Error deleting usr.crd: {}", err);
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
trait SettingsItem: Component {
	type Output;
	fn get_setting(game_settings: &GameSettings) -> Self::Output;
}

#[derive(Component)]
struct TabButton {
	tab_fn: TabFn,
	font: Handle<Font>
}

#[derive(Component)]
enum TabLabel {
	Video,
	Audio,
	Keybinds,
	Account
}

#[derive(Component)]
struct TabContent;

//TODO find out if any options require restarting the game when changed
// #[derive(Component)]
// struct RestartMessage;

#[derive(Component)]
struct VSync;
impl SettingsItem for VSync {
	type Output = bool;
	fn get_setting(game_settings: &GameSettings) -> bool {game_settings.vsync}
}

#[derive(Component)]
struct MaxFrameRate;
impl SettingsItem for MaxFrameRate {
	type Output = f64;
	fn get_setting(game_settings: &GameSettings) -> f64 {game_settings.max_frame_rate}
}

#[derive(Component)]
struct Fullscreen;
impl SettingsItem for Fullscreen {
	type Output = bool;
	fn get_setting(game_settings: &GameSettings) -> bool {game_settings.fullscreen}
}

#[derive(Component)]
struct DefaultMonitor;
impl SettingsItem for DefaultMonitor {
	type Output = MonitorSelection;
	fn get_setting(game_settings: &GameSettings) -> MonitorSelection {game_settings.default_monitor}
}

#[derive(Component)]
struct MonitorIndex;
impl SettingsItem for MonitorIndex {
	type Output = usize;
	fn get_setting(game_settings: &GameSettings) -> usize {game_settings.monitor_index}
}

#[derive(Component)]
struct MasterVolume;
impl SettingsItem for MasterVolume {
	type Output = f64;
	fn get_setting(game_settings: &GameSettings) -> f64 {game_settings.master_volume}
}

#[derive(Component)]
struct MusicVolume;
impl SettingsItem for MusicVolume {
	type Output = f64;
	fn get_setting(game_settings: &GameSettings) -> f64 {game_settings.music_volume}
}

#[derive(Component)]
struct EffectsVolume;
impl SettingsItem for EffectsVolume {
	type Output = f64;
	fn get_setting(game_settings: &GameSettings) -> f64 {game_settings.effects_volume}
}

#[derive(Component)]
struct PassKeybind(Vec<KeyCode>);
impl SettingsItem for PassKeybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.bid_0.clone()}
}

#[derive(Component)]
struct Bid2Keybind(Vec<KeyCode>);
impl SettingsItem for Bid2Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.bid_2.clone()}
}

#[derive(Component)]
struct Bid3Keybind(Vec<KeyCode>);
impl SettingsItem for Bid3Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.bid_3.clone()}
}

#[derive(Component)]
struct Bid4Keybind(Vec<KeyCode>);
impl SettingsItem for Bid4Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.bid_4.clone()}
}

#[derive(Component)]
struct PlayCard1Keybind(Vec<KeyCode>);
impl SettingsItem for PlayCard1Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.play_card_1.clone()}
}

#[derive(Component)]
struct PlayCard2Keybind(Vec<KeyCode>);
impl SettingsItem for PlayCard2Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.play_card_2.clone()}
}

#[derive(Component)]
struct PlayCard3Keybind(Vec<KeyCode>);
impl SettingsItem for PlayCard3Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.play_card_3.clone()}
}

#[derive(Component)]
struct PlayCard4Keybind(Vec<KeyCode>);
impl SettingsItem for PlayCard4Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.play_card_4.clone()}
}

#[derive(Component)]
struct PlayCard5Keybind(Vec<KeyCode>);
impl SettingsItem for PlayCard5Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.play_card_5.clone()}
}

#[derive(Component)]
struct PlayCard6Keybind(Vec<KeyCode>);
impl SettingsItem for PlayCard6Keybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.play_card_6.clone()}
}

#[derive(Component)]
struct ToggleSettingsKeybind(Vec<KeyCode>);
impl SettingsItem for ToggleSettingsKeybind {
	type Output = Vec<KeyCode>;
	fn get_setting(game_settings: &GameSettings) -> Vec<KeyCode> {game_settings.toggle_settings.clone()}
}

#[derive(Component)]
struct SaveLoginCredentials;
impl SettingsItem for SaveLoginCredentials {
	type Output = bool;
	fn get_setting(game_settings: &GameSettings) -> bool {game_settings.save_login_credentials}
}

#[derive(Component)]
struct LogoutButton;
impl SettingsItem for LogoutButton {
	type Output = ();
	fn get_setting(_game_settings: &GameSettings) {panic!("This is only to satisfy the compiler")}
}

#[derive(Component)]
struct LogoutMessage;