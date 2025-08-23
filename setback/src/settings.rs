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
			.add_systems(Update, (update_checkbox, update_spinbox, update_dropdown, update_button, update_keybind).run_if(in_state(Self::SCENE)))
			.add_systems(OnExit(Self::SCENE), Self::on_exit);
	}
}

fn on_enter(mut commands: Commands, asset_server: Res<AssetServer>, game_settings: Res<GameSettings>) {
	info!("Entering Settings");
	menu_background(&mut commands, asset_server.load(TIMES_NEW_ROMAN), |commands| {
		settings_menu(commands, asset_server.as_ref(), game_settings);
	});
}

//TODO
// Fill in the styling based on the html using the main menu as an example.
// Use font everywhere except for where font_bold is already being used.
// Add CursorComponent to each of the interactable items and use "cursor: Handle<Image>" in the same way the main menu does.

///This function is used to create the settings menu for the settings scene and the settings menu in-game
pub fn settings_menu(commands: &mut RelatedSpawnerCommands<ChildOf>, asset_server: &AssetServer, game_settings: Res<GameSettings>) {
	let font = asset_server.load(GEORGIA);
	let font_bold = asset_server.load(GEORGIA_BOLD);
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
					Text::new("Video"),
					TextFont::from_font(font_bold.clone())
				)]
			)).observe(click_tab);
			commands.spawn((
				Node {
					..default()
				},
				Button,
				TabButton {tab_fn: audio_tab, font: font_bold},
				children![(
					TabLabel::Audio,
					Text::new("Audio"),
					TextFont::from_font(font.clone())
				)]
			)).observe(click_tab);
			commands.spawn((
				Node {
					..default()
				},
				Button,
				TabButton {tab_fn: keybinds_tab, font: font.clone()},
				children![(
					TabLabel::Keybinds,
					Text::new("Keybinds"),
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
					Text::new("Account"),
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
			new_tab(commands, asset_server, game_settings, video_tab);
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
						Text::new("Rebind Keys")
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
						Text::new("Press any keys...")
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

type TabFn = fn(commands: &mut RelatedSpawnerCommands<ChildOf>, asset_server: &AssetServer, game_settings: &GameSettings);
type InteractiveFn<C: SettingsItem> = fn(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings);

fn new_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, asset_server: &AssetServer, game_settings: Res<GameSettings>, tab: TabFn) {
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
			tab(commands, asset_server, game_settings.as_ref());
		});
	});
}

fn video_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, asset_server: &AssetServer, game_settings: &GameSettings) {
	new_setting_item(commands, VSync, asset_server, game_settings, new_checkbox);
	new_setting_item(commands, MaxFrameRate, asset_server, game_settings, new_spinbox);
	new_setting_item(commands, Fullscreen, asset_server, game_settings, new_checkbox);
	new_setting_item(commands, DefaultMonitor, asset_server, game_settings, new_dropdown);
	new_setting_item(commands, MonitorIndex, asset_server, game_settings, new_spinbox);
}

fn audio_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, asset_server: &AssetServer, game_settings: &GameSettings) {
	new_setting_item(commands, MasterVolume, asset_server, game_settings, new_slider);
	new_setting_item(commands, MusicVolume, asset_server, game_settings, new_slider);
	new_setting_item(commands, EffectsVolume, asset_server, game_settings, new_slider);
}

fn keybinds_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, asset_server: &AssetServer, game_settings: &GameSettings) {
	new_keybind_item(commands, PassKeybind(game_settings.bid_0.clone()), asset_server, game_settings);
	new_keybind_item(commands, Bid2Keybind(game_settings.bid_2.clone()), asset_server, game_settings);
	new_keybind_item(commands, Bid3Keybind(game_settings.bid_3.clone()), asset_server, game_settings);
	new_keybind_item(commands, Bid4Keybind(game_settings.bid_4.clone()), asset_server, game_settings);
	new_keybind_item(commands, PlayCard1Keybind(game_settings.play_card_1.clone()), asset_server, game_settings);
	new_keybind_item(commands, PlayCard2Keybind(game_settings.play_card_2.clone()), asset_server, game_settings);
	new_keybind_item(commands, PlayCard3Keybind(game_settings.play_card_3.clone()), asset_server, game_settings);
	new_keybind_item(commands, PlayCard4Keybind(game_settings.play_card_4.clone()), asset_server, game_settings);
	new_keybind_item(commands, PlayCard5Keybind(game_settings.play_card_5.clone()), asset_server, game_settings);
	new_keybind_item(commands, PlayCard6Keybind(game_settings.play_card_6.clone()), asset_server, game_settings);
	new_keybind_item(commands, ToggleSettingsKeybind(game_settings.toggle_settings.clone()), asset_server, game_settings);
}

fn account_tab(commands: &mut RelatedSpawnerCommands<ChildOf>, asset_server: &AssetServer, game_settings: &GameSettings) {
	new_setting_item(commands, SaveLoginCredentials, asset_server, game_settings, new_checkbox);
	new_setting_item(commands, LogoutButton, asset_server, game_settings, new_button);
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
	marker: C,
	asset_server: &AssetServer,
	game_settings: &GameSettings,
	interactive: InteractiveFn<C>
) {
	//setting label
	commands.spawn((
		Node {
			..default()
		},
		children![(
			Text::new(C::get_label())
		)]
	));
	//setting control
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		//checkbox/spin box/dropdown/slider/keybind
		interactive(commands, marker, asset_server, game_settings);
	});
}

fn new_setting_item<C: SettingsItem>(
	commands: &mut RelatedSpawnerCommands<ChildOf>,
	marker: C,
	asset_server: &AssetServer,
	game_settings: &GameSettings,
	interactive: InteractiveFn<C>
) {
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		item_label_and_control(commands, marker, asset_server, game_settings, interactive);
	});
}

fn new_keybind_item<C: SettingsItem>(
	commands: &mut RelatedSpawnerCommands<ChildOf>,
	marker: C,
	asset_server: &AssetServer,
	game_settings: &GameSettings
) {
	commands.spawn((
		Node {
			..default()
		}
	)).with_children(|commands| {
		item_label_and_control(commands, marker, asset_server, game_settings, new_keybind);
	});
}

fn new_checkbox<C: SettingsItem<Output=bool>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let value = C::get_setting(game_settings);
	todo!()
}

fn new_spinbox<C: SettingsItem<Output=f64>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	todo!()
}

fn new_dropdown<const N: usize, C: DropdownOptions<N>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let (curr_opt, remain_opt) = C::get_options(game_settings);
	todo!()
}

fn new_slider<C: SettingsItem<Output=f64>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let font_bold = asset_server.load(GEORGIA_BOLD);
	todo!()
}

fn new_button<C: SettingsButton>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	todo!()
}

fn new_keybind<C: SettingsItem<Output=Vec<KeyCode>>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
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
		(tab_button.tab_fn)(commands, asset_server.as_ref(), game_settings.as_ref());
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
	buttons: Query<&mut Node, With<LogoutButton>>,
	mut logout_message: Query<(&mut Text, &ChildOf), With<LogoutMessage>>,
	logout_message_node: Query<&mut Node>,
	scene: Res<State<UiScene>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	delete_credentials();
	if server.is_some() && matches!(scene.get(), UiScene::Settings) {
		commands.remove_resource::<ServerConn>();
	}
	let mut logout_button = buttons.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
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
	fn get_label() -> &'static str;
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

macro_rules! settings_item {
    ($name:ident, $output:ty, $field:ident, $label:literal) => {
        #[derive(Component)]
        struct $name;
        impl SettingsItem for $name {
            type Output = $output;
            const fn get_setting(game_settings: &GameSettings) -> Self::Output {game_settings.$field.clone()}
	        const fn get_label() -> &'static str {$label}
        }
    };
	($name:ident, $field:ident, $output:ty, $label:literal) => {
        #[derive(Component)]
        struct $name($output);
        impl SettingsItem for $name {
            type Output = $output;
            const fn get_setting(game_settings: &GameSettings) -> Self::Output {game_settings.$field.clone()}
	        const fn get_label() -> &'static str {$label}
        }
    };
}

settings_item!(VSync, bool, vsync, "VSync");
settings_item!(MaxFrameRate, f64, max_frame_rate, "Max Frame Rate");
settings_item!(Fullscreen, bool, fullscreen, "Fullscreen");
settings_item!(DefaultMonitor, u8, default_monitor, "Default Monitor");
settings_item!(MonitorIndex, usize, monitor_index, "Monitor Index");
settings_item!(MasterVolume, f64, master_volume, "Master Volume");
settings_item!(MusicVolume, f64, music_volume, "Music Volume");
settings_item!(EffectsVolume, f64, effects_volume, "Effects Volume");
settings_item!(PassKeybind, bid_0, Vec<KeyCode>, "Pass");
settings_item!(Bid2Keybind, bid_2, Vec<KeyCode>, "Bid 2");
settings_item!(Bid3Keybind, bid_3, Vec<KeyCode>, "Bid 3");
settings_item!(Bid4Keybind, bid_4, Vec<KeyCode>, "Bid 4");
settings_item!(PlayCard1Keybind, play_card_1, Vec<KeyCode>, "Play Card 1");
settings_item!(PlayCard2Keybind, play_card_2, Vec<KeyCode>, "Play Card 2");
settings_item!(PlayCard3Keybind, play_card_3, Vec<KeyCode>, "Play Card 3");
settings_item!(PlayCard4Keybind, play_card_4, Vec<KeyCode>, "Play Card 4");
settings_item!(PlayCard5Keybind, play_card_5, Vec<KeyCode>, "Play Card 5");
settings_item!(PlayCard6Keybind, play_card_6, Vec<KeyCode>, "Play Card 6");
settings_item!(ToggleSettingsKeybind, toggle_settings, Vec<KeyCode>, "Toggle Settings");
settings_item!(SaveLoginCredentials, bool, save_login_credentials, "Save Credentials");

#[derive(Component)]
struct LogoutButton;
impl SettingsItem for LogoutButton {
	type Output = !;
	const fn get_setting(_game_settings: &GameSettings) -> Self::Output {panic!("This item does not have a corresponding setting")}
	const fn get_label() -> &'static str {"Account Actions"}
}
trait SettingsButton: SettingsItem {
	fn get_btn_label() -> &'static str;
}
impl SettingsButton for LogoutButton {
	const fn get_btn_label() -> &'static str {"Logout"}
}

#[derive(Component)]
struct LogoutMessage;

trait DropdownOptions<const N: usize>: SettingsItem<Output=u8> {
	fn get_options(game_settings: &GameSettings) -> (&'static str, [&'static str; N]);
	fn get_monitor_selection(curr_index: u8) -> MonitorSelection;
}

impl DropdownOptions<2> for DefaultMonitor {
	const fn get_options(game_settings: &GameSettings) -> (&'static str, [&'static str; 2]) {
		static CURRENT: &str = "Current";
		static PRIMARY: &str = "Primary";
		static INDEX: &str = "Index";
		match game_settings.default_monitor {
			1 => (PRIMARY, [CURRENT, INDEX]),
			2 => (INDEX, [CURRENT, PRIMARY]),
			_ => (CURRENT, [PRIMARY, INDEX])
		}
	}
	
	const fn get_monitor_selection(game_settings: &GameSettings) -> MonitorSelection {
		match game_settings.default_monitor {
			1 => MonitorSelection::Current,
			2 => MonitorSelection::Index(game_settings.monitor_index),
			_ => MonitorSelection::Primary
		}
	}
}

fn update_checkbox() {}

fn update_spinbox() {}

fn update_dropdown() {}

fn update_button() {}

fn update_keybind() {}