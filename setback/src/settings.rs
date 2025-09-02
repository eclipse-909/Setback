use {
	crate::{
		constants::*,
		resources::*,
		states::*,
		systems::*
	},
	bevy::{
		ecs::{
			relationship::RelatedSpawnerCommands,
			system::IntoObserverSystem
		},
		window::{PresentMode, WindowMode},
		prelude::*
	},
	keyring::Entry,
	libsetback::{SETBACK_VERSION, WebsiteConnection},
	std::{convert::Infallible, fs}
};
use crate::components::CursorComponent;
use crate::plugins::CursorStyle;

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
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	//settings container
	commands.spawn((
		Node {
			display: Display::Flex,
			flex_direction: FlexDirection::Column,
			max_width: Val::Px(800.),
			width: Val::Percent(90.),
			max_height: Val::Vh(90.),
			..default()
		},
		BackgroundColor(Color::srgb_u8(44, 24, 16).with_alpha(0.9)),
		BorderRadius::all(PALLET.border_radius),
		BoxShadow(vec![
			ShadowStyle {
				color: PALLET.card_shadow,
				x_offset: Val::Px(0.),
				y_offset: Val::Px(20.),
				blur_radius: Val::Px(60.),
				..default()
			},
			ShadowStyle {
				color: Color::WHITE.with_alpha(0.1),
				x_offset: Val::Px(0.),
				y_offset: Val::Px(1.),
				blur_radius: Val::Px(0.),
				..default()
			}
		]),
		Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold)
	)).with_children(|commands| {
		//settings header
		commands.spawn((
			Node {
				padding: UiRect::axes(Val::Px(30.), Val::Px(20.)),
				display: Display::Flex,
				align_items: AlignItems::Center,
				justify_content: JustifyContent::SpaceBetween,
				..default()
			},
			BackgroundColor(Color::srgb_u8(26, 122, 62).with_alpha(0.3)),
			BorderColor(PALLET.accent_gold),
			BorderBottom(Val::Px(2.))
		)).with_children(|commands| {
			//back button
			commands.spawn((
				Button,
				Node {
					padding: UiRect::axes(Val::Px(20.), Val::Px(10.)),
					display: Display::Flex,
					align_items: AlignItems::Center,
					column_gap: Val::Px(8.),
					..default()
				},
				BackgroundColor(PALLET.secondary_green),
				BorderRadius::all(Val::Px(8.)),
				Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
				BoxShadow(vec![
					ShadowStyle {
						x_offset: Val::Px(0.),
						y_offset: Val::Px(4.),
						blur_radius: Val::Px(15.),
						color: Color::BLACK.with_alpha(0.2),
						..default()
					}
				]),
				CursorComponent {
					default: CursorStyle::Icon(cursor.clone()),
					pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
				},
				children![(
					Text::new("◀ Back"),
					TextColor(PALLET.cream),
					TextFont::from_font(font.clone()).with_font_size(14.)
				)]
			)).observe(|trigger: Trigger<Pointer<Click>>, mut next_scene: ResMut<NextState<UiScene>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				next_scene.set(UiScene::MainMenu);
			});
			//settings title
			commands.spawn((
				Node {
					// flex: 1.,
					justify_content: JustifyContent::Center,
					..default()
				},
				children![(
					Text::new("Settings"),
					TextColor(PALLET.accent_gold),
					TextFont::from_font(font_bold.clone()).with_font_size(32.),
					TextShadow {
						offset: Vec2::new(2., 2.),
						color: PALLET.text_shadow
					}
				)]
			));
			//spacer
			commands.spawn(Node {
				width: Val::Px(120.),
				..default()
			});
		});
		//tab container
		commands.spawn((
			Node {
				display: Display::Flex,
				..default()
			},
			BackgroundColor(Color::BLACK.with_alpha(0.2))
		)).with_children(|commands| {
			// Video tab
			commands.spawn((
				Button,
				Node {
					// flex: 1.,
					padding: UiRect::axes(Val::Px(20.), Val::Px(16.)),
					..default()
				},
				BackgroundColor(PALLET.accent_gold),
				BorderRight(Val::Px(1.)),
				BorderColor(PALLET.accent_gold),
				TabButton {tab_fn: video_tab, font: font.clone()},
				CursorComponent {
					default: CursorStyle::Icon(cursor.clone()),
					pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
				},
				TabActive,
				children![(
					TabLabel::Video,
					Text::new("Video"),
					TextColor(PALLET.dark_brown),
					TextFont::from_font(font_bold.clone()).with_font_size(16.)
				)]
			)).observe(click_tab);
			// Audio tab
			commands.spawn((
				Button,
				Node {
					// flex: 1.,
					padding: UiRect::axes(Val::Px(20.), Val::Px(16.)),
					..default()
				},
				BackgroundColor(Color::srgb_u8(13, 93, 46).with_alpha(0.5)),
				BorderRight(Val::Px(1.)),
				BorderColor(PALLET.accent_gold),
				TabButton {tab_fn: audio_tab, font: font_bold.clone()},
				CursorComponent {
					default: CursorStyle::Icon(cursor.clone()),
					pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
				},
				children![(
					TabLabel::Audio,
					Text::new("Audio"),
					TextColor(PALLET.cream),
					TextFont::from_font(font.clone()).with_font_size(16.)
				)]
			)).observe(click_tab);
			// Keybinds tab
			commands.spawn((
				Button,
				Node {
					// flex: 1.,
					padding: UiRect::axes(Val::Px(20.), Val::Px(16.)),
					..default()
				},
				BackgroundColor(Color::srgb_u8(13, 93, 46).with_alpha(0.5)),
				BorderRight(Val::Px(1.)),
				BorderColor(PALLET.accent_gold),
				TabButton {tab_fn: keybinds_tab, font: font.clone()},
				CursorComponent {
					default: CursorStyle::Icon(cursor.clone()),
					pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
				},
				children![(
					TabLabel::Keybinds,
					Text::new("Keybinds"),
					TextColor(PALLET.cream),
					TextFont::from_font(font.clone()).with_font_size(16.)
				)]
			)).observe(click_tab);
			// Account tab
			commands.spawn((
				Button,
				Node {
					// flex: 1.,
					padding: UiRect::axes(Val::Px(20.), Val::Px(16.)),
					..default()
				},
				BackgroundColor(Color::srgb_u8(13, 93, 46).with_alpha(0.5)),
				TabButton {tab_fn: account_tab, font: font.clone()},
				CursorComponent {
					default: CursorStyle::Icon(cursor.clone()),
					pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
				},
				children![(
					TabLabel::Account,
					Text::new("Account"),
					TextColor(PALLET.cream),
					TextFont::from_font(font.clone()).with_font_size(16.)
				)]
			)).observe(click_tab);
		});
		//tab content
		commands.spawn((
			Node {
				// flex: 1.,
				padding: UiRect::all(Val::Px(30.)),
				overflow: Overflow::scroll_y(),
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
				justify_content: JustifyContent::Center,
				padding: UiRect::all(Val::Px(20.)),
				..default()
			},
			BorderTop(Val::Px(1.)),
			BorderColor(PALLET.accent_gold.with_alpha(0.3)),
			children![(
				Text::new(format!("Version {SETBACK_VERSION}")),
				TextFont::from_font(font.clone()).with_font_size(12.),
				TextColor(PALLET.light_brown.with_alpha(0.7)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		//modal overlay
		commands.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(0.),
				left: Val::Px(0.),
				width: Val::Percent(100.),
				height: Val::Percent(100.),
				display: Display::None,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::BLACK.with_alpha(0.7)),
			KeybindModalOverlay
		)).with_children(|commands| {
			//keybind modal
			commands.spawn((
				Node {
					min_width: Val::Px(400.),
					padding: UiRect::all(Val::Px(30.)),
					display: Display::Flex,
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::Center,
					..default()
				},
				BackgroundColor(PALLET.dark_brown),
				BorderRadius::all(PALLET.border_radius),
				Outline::new(Val::Px(3.), Val::Px(0.), PALLET.accent_gold),
				BoxShadow(vec![
					ShadowStyle {
						x_offset: Val::Px(0.),
						y_offset: Val::Px(20.),
						blur_radius: Val::Px(60.),
						color: PALLET.card_shadow,
						..default()
					}
				])
			)).with_children(|commands| {
				//modal title
				commands.spawn((
					Node {
						margin: UiRect::bottom(Val::Px(20.)),
						..default()
					},
					children![(
						Text::new("Rebind Key"),
						TextColor(PALLET.accent_gold),
						TextFont::from_font(font_bold.clone()).with_font_size(24.)
					)]
				));
				//modal instruction
				commands.spawn((
					Node {
						margin: UiRect::bottom(Val::Px(20.)),
						..default()
					},
					children![(
						Text::new("Press the key(s) you want to bind for this action:"),
						TextColor(PALLET.cream),
						TextFont::from_font(font.clone()).with_font_size(16.)
					)]
				));
				//key display
				commands.spawn((
					Node {
						min_height: Val::Px(60.),
						min_width: Val::Px(300.),
						padding: UiRect::all(Val::Px(15.)),
						margin: UiRect::vertical(Val::Px(20.)),
						display: Display::Flex,
						align_items: AlignItems::Center,
						justify_content: JustifyContent::Center,
						..default()
					},
					BackgroundColor(Color::BLACK.with_alpha(0.5)),
					BorderRadius::all(Val::Px(8.)),
					Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
					KeybindModalDisplay,
					children![(
						Text::new("Press any keys..."),
						TextColor(PALLET.light_gold),
						TextFont::from_font_size(18.)
					)]
				));
				//modal buttons
				commands.spawn((
					Node {
						display: Display::Flex,
						column_gap: Val::Px(15.),
						margin: UiRect::top(Val::Px(20.)),
						..default()
					}
				)).with_children(|commands| {
					//cancel button
					commands.spawn((
						Button,
						Node {
							padding: UiRect::axes(Val::Px(20.), Val::Px(10.)),
							..default()
						},
						BackgroundColor(PALLET.medium_brown),
						BorderRadius::all(Val::Px(8.)),
						Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
						CursorComponent {
							default: CursorStyle::Icon(cursor.clone()),
							pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
						},
						children![(
							Text::new("Cancel"),
							TextColor(PALLET.cream),
							TextFont::from_font(font.clone()).with_font_size(14.)
						)]
					)).observe(|trigger: Trigger<Pointer<Click>>, mut modal: Query<&mut Node, With<KeybindModalOverlay>>| {
						if !matches!(trigger.button, PointerButton::Primary) {return}
						modal.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR).display = Display::None;
					});
					//apply button
					commands.spawn((
						Button,
						Node {
							padding: UiRect::axes(Val::Px(20.), Val::Px(10.)),
							..default()
						},
						BackgroundColor(PALLET.accent_gold),
						BorderRadius::all(Val::Px(8.)),
						Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
						CursorComponent {
							default: CursorStyle::Icon(cursor),
							pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
						},
						children![(
							Text::new("Apply Binding"),
							TextColor(PALLET.dark_brown),
							TextFont::from_font(font_bold).with_font_size(14.)
						)]
					)).observe(|trigger: Trigger<Pointer<Click>>| {
						if !matches!(trigger.button, PointerButton::Primary) {return}
						// Apply keybind logic will be implemented with the keybind system
						todo!("Apply new keybinding")
					});
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
			display: Display::Flex,
			flex_direction: FlexDirection::Column,
			..default()
		}
	)).with_children(|commands| {
		//setting group
		commands.spawn((
			Node {
				display: Display::Flex,
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(24.),
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
			margin: UiRect::top(Val::Px(10.)),
			display: Display::None, // Initially hidden
			..default()
		},
		children![(
			Text::new("You have been logged out successfully."),
			TextColor(PALLET.light_gold),
			TextFont::from_font(asset_server.load(GEORGIA_ITALIC)).with_font_size(16.)
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
			min_width: Val::Px(150.),
			..default()
		},
		children![(
			Text::new(C::get_label()),
			TextColor(PALLET.cream),
			TextFont::from_font(asset_server.load(GEORGIA)).with_font_size(16.)
		)]
	));
	//setting control
	commands.spawn((
		Node {
			display: Display::Flex,
			align_items: AlignItems::Center,
			column_gap: Val::Px(10.),
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
			display: Display::Flex,
			align_items: AlignItems::Center,
			justify_content: JustifyContent::SpaceBetween,
			padding: UiRect::vertical(Val::Px(12.)),
			margin: UiRect::bottom(Val::Px(16.)),
			..default()
		}
	)).with_children(|commands| {
		item_label_and_control(commands, marker, asset_server, game_settings, interactive);
	});
}

fn new_keybind_item<C: SettingsItem<Output=Vec<KeyCode>>>(
	commands: &mut RelatedSpawnerCommands<ChildOf>,
	marker: C,
	asset_server: &AssetServer,
	game_settings: &GameSettings
) {
	commands.spawn((
		Node {
			display: Display::Flex,
			align_items: AlignItems::Center,
			justify_content: JustifyContent::SpaceBetween,
			padding: UiRect::all(Val::Px(15.)),
			margin: UiRect::bottom(Val::Px(12.)),
			..default()
		},
		BackgroundColor(Color::BLACK.with_alpha(0.2)),
		BorderRadius::all(Val::Px(8.)),
		Outline::new(Val::Px(1.), Val::Px(0.), PALLET.accent_gold.with_alpha(0.3))
	)).with_children(|commands| {
		item_label_and_control(commands, marker, asset_server, game_settings, new_keybind);
	});
}

#[derive(Component)]
struct Checkbox(bool);

#[derive(Component)]
struct SpinBox(usize);

#[derive(Component)]
struct Dropdown<T, const N: usize>(T, [T; N]);

#[derive(Component)]
struct Slider(f64);

#[derive(Component)]
struct Keybind(Vec<KeyCode>);

#[derive(Component)]
struct KeybindModalOverlay;

#[derive(Component)]
struct KeybindModalDisplay;

#[derive(Component)]
struct TabActive;

fn new_checkbox<C: SettingsItem<Output=bool>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let checked = C::get_setting(game_settings);
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	commands.spawn((
		Button,
		Node {
			width: Val::Px(20.),
			height: Val::Px(20.),
			display: Display::Flex,
			align_items: AlignItems::Center,
			justify_content: JustifyContent::Center,
			..default()
		},
		BackgroundColor(if checked { PALLET.accent_gold } else { PALLET.dark_brown }),
		BorderRadius::all(Val::Px(4.)),
		Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
		CursorComponent {
			default: CursorStyle::Icon(cursor),
			pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
		},
		Checkbox(checked),
		marker,
		children![
			if checked {(
				Text::new("✓"),
				TextColor(PALLET.dark_brown),
				TextFont::from_font_size(16.)
			)} else {(
				Text::new(""),
				TextColor(PALLET.dark_brown),
				TextFont::from_font_size(16.)
			)}
		]
	)).observe(C::apply());
}

fn new_spinbox<C: SettingsItem<Output=usize>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let num = C::get_setting(game_settings);
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	let font = asset_server.load(GEORGIA);
	let disabled = if std::any::type_name::<C>().contains("MaxFrameRate") {
		game_settings.vsync
	} else if std::any::type_name::<C>().contains("MonitorIndex") {
		!matches!(DefaultMonitor::get_selection(game_settings), MonitorSelection::Index(_))
	} else {
		false
	};

	commands.spawn((
		Node {
			width: Val::Px(120.),
			padding: UiRect::axes(Val::Px(12.), Val::Px(8.)),
			display: Display::Flex,
			align_items: AlignItems::Center,
			justify_content: JustifyContent::Center,
			..default()
		},
		BackgroundColor(if disabled { PALLET.disabled_bg } else { PALLET.dark_brown }),
		BorderRadius::all(Val::Px(6.)),
		Outline::new(Val::Px(2.), Val::Px(0.), if disabled { PALLET.disabled_color } else { PALLET.accent_gold }),
		CursorComponent {
			default: CursorStyle::Icon(cursor.clone()),
			pointer: if disabled { CursorStyle::Icon(cursor) } else { CursorStyle::Animation(BJ_HELP_SELECT_PAGE) }
		},
		SpinBox(num),
		marker,
		children![(
			Text::new(num.to_string()),
			TextColor(if disabled { PALLET.disabled_color } else { PALLET.cream }),
			TextFont::from_font(font).with_font_size(14.)
		)]
	));
}

fn new_dropdown<T, const N: usize, C: DropdownOptions<T, N>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let (curr_opt, remain_opt) = C::get_options(game_settings);
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	let font = asset_server.load(GEORGIA);

	commands.spawn((
		Button,
		Node {
			width: Val::Px(120.),
			padding: UiRect::axes(Val::Px(12.), Val::Px(8.)),
			display: Display::Flex,
			align_items: AlignItems::Center,
			justify_content: JustifyContent::SpaceBetween,
			..default()
		},
		BackgroundColor(PALLET.dark_brown),
		BorderRadius::all(Val::Px(6.)),
		Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
		CursorComponent {
			default: CursorStyle::Icon(cursor),
			pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
		},
		Dropdown(curr_opt.clone(), remain_opt),
		marker,
		children![(
			Text::new(C::display(curr_opt)),
			TextColor(PALLET.cream),
			TextFont::from_font(font).with_font_size(14.)
		)]
	)).observe(C::apply());
}

fn new_slider<C: SettingsItem<Output=f64>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let num = C::get_setting(game_settings);
	let font_bold = asset_server.load(GEORGIA_BOLD);
	let cursor = asset_server.load(BJ_NORMAL_SELECT);

	commands.spawn((
		Node {
			display: Display::Flex,
			align_items: AlignItems::Center,
			column_gap: Val::Px(15.),
			width: Val::Px(250.),
			..default()
		}
	)).with_children(|commands| {
		// Slider track
		commands.spawn((
			Node {
				// flex: 1.,
				height: Val::Px(6.),
				position_type: PositionType::Relative,
				..default()
			},
			BackgroundColor(PALLET.dark_brown),
			BorderRadius::all(Val::Px(3.)),
			CursorComponent {
				default: CursorStyle::Icon(cursor.clone()),
				pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
			},
			Slider(num),
			marker
		)).with_children(|commands| {
			// Slider thumb
			commands.spawn((
				Node {
					width: Val::Px(20.),
					height: Val::Px(20.),
					position_type: PositionType::Absolute,
					left: Val::Percent((num * 100.) as f32),
					top: Val::Px(-7.), // Center on track
					..default()
				},
				BackgroundColor(PALLET.accent_gold),
				BorderRadius::all(Val::Percent(50.)),
				Outline::new(Val::Px(2.), Val::Px(0.), PALLET.dark_gold),
				CursorComponent {
					default: CursorStyle::Icon(cursor),
					pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
				}
			));
		});

		// Value display
		commands.spawn((
			Node {
				min_width: Val::Px(40.),
				justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new(format!("{}%", (num * 100.) as i32)),
				TextColor(PALLET.light_gold),
				TextFont::from_font(font_bold).with_font_size(14.)
			)]
		));
	});
}

fn new_button<C: SettingsButton>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	let font = asset_server.load(GEORGIA);

	commands.spawn((
		Button,
		Node {
			padding: UiRect::axes(Val::Px(24.), Val::Px(12.)),
			..default()
		},
		BackgroundColor(PALLET.secondary_green),
		BorderRadius::all(PALLET.border_radius),
		Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
		BoxShadow(vec![
			ShadowStyle {
				x_offset: Val::Px(0.),
				y_offset: Val::Px(4.),
				blur_radius: Val::Px(15.),
				color: Color::BLACK.with_alpha(0.2),
				..default()
			}
		]),
		CursorComponent {
			default: CursorStyle::Icon(cursor),
			pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
		},
		marker,
		children![(
			Text::new(C::get_btn_label()),
			TextColor(PALLET.cream),
			TextFont::from_font(font).with_font_size(16.)
		)]
	)).observe(C::apply());
}

fn new_keybind<C: SettingsItem<Output=Vec<KeyCode>>>(commands: &mut RelatedSpawnerCommands<ChildOf>, marker: C, asset_server: &AssetServer, game_settings: &GameSettings) {
	let key_codes = C::get_setting(game_settings);
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	let font = asset_server.load(GEORGIA);

	// Keybind display
	commands.spawn((
		Node {
			padding: UiRect::axes(Val::Px(12.), Val::Px(6.)),
			min_width: Val::Px(80.),
			..default()
		},
		BackgroundColor(PALLET.dark_brown),
		BorderRadius::all(Val::Px(6.)),
		Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
		Keybind(key_codes.clone()),
		children![(
			Text::new(format_keycodes(&key_codes)),
			TextColor(PALLET.light_gold),
			TextFont::from_font_size(14.),
			TextLayout::new_with_justify(JustifyText::Center)
		)]
	));

	// Rebind button
	commands.spawn((
		Button,
		Node {
			padding: UiRect::axes(Val::Px(16.), Val::Px(6.)),
			..default()
		},
		BackgroundColor(PALLET.secondary_green),
		BorderRadius::all(Val::Px(6.)),
		Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
		CursorComponent {
			default: CursorStyle::Icon(cursor),
			pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
		},
		marker,
		children![(
			Text::new("Rebind"),
			TextColor(PALLET.cream),
			TextFont::from_font(font).with_font_size(14.)
		)]
	)).observe(|trigger: Trigger<Pointer<Click>>, mut modal: Query<&mut Node, With<KeybindModalOverlay>>| {
		if !matches!(trigger.button, PointerButton::Primary) {return}
		modal.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR).display = Display::Flex;
	});
}

fn format_keycodes(codes: &[KeyCode]) -> String {
	if codes.is_empty() {
		"None".to_string()
	} else {
		codes.iter()
			.map(|code| format!("{:?}", code))
			.collect::<Vec<_>>()
			.join(" + ")
	}
}

fn click_tab(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	tab_content: Query<Entity, With<TabContent>>,
	tab_button: Query<&TabButton>,
	mut all_tabs: Query<(Entity, &mut BackgroundColor, Option<&TabActive>), With<TabButton>>,
	mut tab_labels: Query<(&mut TextFont, &mut TextColor, &ChildOf), With<TabLabel>>,
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

	// Update tab appearances
	for (entity, mut bg_color, has_active) in all_tabs.iter_mut() {
		if entity == trigger.target {
			bg_color.0 = PALLET.accent_gold;
			if has_active.is_none() {
				commands.entity(entity).insert(TabActive);
			}
		} else {
			bg_color.0 = Color::srgb_u8(13, 93, 46).with_alpha(0.5);
			if has_active.is_some() {
				commands.entity(entity).remove::<TabActive>();
			}
		}
	}

	// Update tab label appearances
	for (mut tab_font, mut tab_color, child_of) in tab_labels.iter_mut() {
		if child_of.parent() == trigger.target {
			tab_font.font = asset_server.load(GEORGIA_BOLD);
			tab_color.0 = PALLET.dark_brown;

			#[cfg(debug_assertions)]
			changed = true;
		} else {
			tab_font.font = asset_server.load(GEORGIA);
			tab_color.0 = PALLET.cream;
		}
	}
	debug_assert!(changed, "Tab label font weight was not updated");
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
	api: Res<WebsiteConn>,
	runtime: Res<TokioRuntime>,
	buttons: Query<&mut Node, With<LogoutButton>>,
	mut logout_message: Query<(&mut Text, &ChildOf), With<LogoutMessage>>,
	logout_message_node: Query<&mut Node>,
	scene: Res<State<UiScene>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	delete_credentials();
	let mut logout_button = buttons.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let (mut logout_message, parent) = logout_message.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let mut logout_message_node = logout_message_node.get(parent.parent()).expect(SINGLE_COMPONENT_QUERY_ERROR);
	todo!("disable and gray out the logout button");
	logout_message.0 = String::from(if let Some(server) = server {
		if matches!(scene.get(), UiScene::Settings) {
			runtime.spawn(async move {
				let server = server.0.lock().await;
				api.0.logout(server.auth_token.as_str()).await;
				server.auth_token = String::new();
				server.refresh_token = String::new();
			});
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
	fn apply() -> impl IntoObserverSystem<_,_,_>;
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
    ($name:ident, $output:ty, $field:ident, $label:literal, $apply_fn:ident) => {
        #[derive(Component)]
        struct $name;
        impl SettingsItem for $name {
            type Output = $output;
            const fn get_setting(game_settings: &GameSettings) -> Self::Output {game_settings.$field.clone()}
	        const fn get_label() -> &'static str {$label}
	        const fn apply() -> impl IntoObserverSystem<_,_,_> {$apply_fn}
        }
    };
	($name:ident, $field:ident, $output:ty, $label:literal, $apply_fn:ident) => {
        #[derive(Component)]
        struct $name($output);
        impl SettingsItem for $name {
            type Output = $output;
            const fn get_setting(game_settings: &GameSettings) -> Self::Output {game_settings.$field.clone()}
	        const fn get_label() -> &'static str {$label}
	        const fn apply() -> impl IntoObserverSystem<_,_,_> {$apply_fn}
        }
    };
}

fn apply_vsync(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>,
	mut window: Query<&mut Window>,
	vsync: Query<&Checkbox, With<VSync>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}
	let window = window.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let vsync = vsync.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	window.present_mode = if vsync.0 {PresentMode::AutoVsync} else {PresentMode::AutoNoVsync};
	game_settings.vsync = vsync.0;
	todo!("if vsync is on, disable max frame rate setting")
}
fn apply_max_frame_rate(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>,
	mut framepace_settings: ResMut<bevy_framepace::FramepaceSettings>,
	frame_rate: Query<&SpinBox, With<MaxFrameRate>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}
	if game_settings.vsync {return;}
	let frame_rate = frame_rate.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	framepace_settings.limiter = bevy_framepace::Limiter::from_framerate(frame_rate.0 as f64);
	game_settings.max_frame_rate = frame_rate.0;
}
fn apply_fullscreen(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>,
	mut window: Query<&mut Window>,
	fullscreen: Query<&Checkbox, With<Fullscreen>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}
	let window = window.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let fullscreen = fullscreen.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	window.mode = if fullscreen.0 {WindowMode::Fullscreen(DefaultMonitor::get_selection(game_settings.as_ref()), VideoModeSelection::Current)} else {WindowMode::Windowed};
	game_settings.fullscreen = fullscreen.0;
	todo!("if not fullscreen, disable default monitor setting")
}
fn apply_default_monitor(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>,
	mut window: Query<&mut Window>,
	default_monitor: Query<&Dropdown<MonitorSelection, 2>, With<DefaultMonitor>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}
	if !game_settings.fullscreen {return;}
	let window = window.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let default_monitor = default_monitor.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	window.mode = WindowMode::Fullscreen(default_monitor.0, VideoModeSelection::Current);
	game_settings.default_monitor = DefaultMonitor::get_index(default_monitor.0);
	todo!("if not index, disable monitor index setting")
}
fn apply_monitor_index(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>,
	mut window: Query<&mut Window>,
	monitor_index: Query<&SpinBox, With<MonitorIndex>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}
	if !game_settings.fullscreen || matches!(DefaultMonitor::get_selection(game_settings.as_ref()), MonitorSelection::Index(_)) {return;}
	let window = window.single_mut().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let monitor_index = monitor_index.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	window.mode = WindowMode::Fullscreen(MonitorSelection::Index(monitor_index.0), VideoModeSelection::Current);
	game_settings.monitor_index = monitor_index.0;
}
fn apply_master_volume(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_music_volume(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_effects_volume(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_pass(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<PassKeybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_bid2(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<Bid2Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_bid3(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<Bid3Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_bid4(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<Bid4Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_play1(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<PlayCard1Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_play2(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<PlayCard2Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_play3(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<PlayCard3Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_play4(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<PlayCard4Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_play5(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<PlayCard5Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_play6(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<PlayCard6Keybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_toggle(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>, keybind: Query<ToggleSettingsKeybind>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}
fn apply_save_login(
	trigger: Trigger<Pointer<Click>>,
	mut game_settings: ResMut<GameSettings>,
	save_login: Query<&Checkbox, With<SaveLoginCredentials>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return;}

	todo!()
}

settings_item!(VSync, bool, vsync, "VSync", apply_vsync);
settings_item!(MaxFrameRate, usize, max_frame_rate, "Max Frame Rate", apply_max_frame_rate);
settings_item!(Fullscreen, bool, fullscreen, "Fullscreen", apply_fullscreen);
settings_item!(DefaultMonitor, u8, default_monitor, "Default Monitor", apply_default_monitor);
settings_item!(MonitorIndex, usize, monitor_index, "Monitor Index", apply_monitor_index);
settings_item!(MasterVolume, f64, master_volume, "Master Volume", apply_master_volume);
settings_item!(MusicVolume, f64, music_volume, "Music Volume", apply_music_volume);
settings_item!(EffectsVolume, f64, effects_volume, "Effects Volume", apply_effects_volume);
settings_item!(PassKeybind, bid_0, Vec<KeyCode>, "Pass", apply_pass);
settings_item!(Bid2Keybind, bid_2, Vec<KeyCode>, "Bid 2", apply_bid2);
settings_item!(Bid3Keybind, bid_3, Vec<KeyCode>, "Bid 3", apply_bid3);
settings_item!(Bid4Keybind, bid_4, Vec<KeyCode>, "Bid 4", apply_bid4);
settings_item!(PlayCard1Keybind, play_card_1, Vec<KeyCode>, "Play Card 1", apply_play1);
settings_item!(PlayCard2Keybind, play_card_2, Vec<KeyCode>, "Play Card 2", apply_play2);
settings_item!(PlayCard3Keybind, play_card_3, Vec<KeyCode>, "Play Card 3", apply_play3);
settings_item!(PlayCard4Keybind, play_card_4, Vec<KeyCode>, "Play Card 4", apply_play4);
settings_item!(PlayCard5Keybind, play_card_5, Vec<KeyCode>, "Play Card 5", apply_play5);
settings_item!(PlayCard6Keybind, play_card_6, Vec<KeyCode>, "Play Card 6", apply_play6);
settings_item!(ToggleSettingsKeybind, toggle_settings, Vec<KeyCode>, "Toggle Settings", apply_toggle);
settings_item!(SaveLoginCredentials, bool, save_login_credentials, "Save Credentials", apply_save_login);

#[derive(Component)]
struct LogoutButton;
impl SettingsItem for LogoutButton {
	type Output = Infallible;
	const fn get_setting(_game_settings: &GameSettings) -> Self::Output {panic!("This item does not have a corresponding setting")}
	const fn get_label() -> &'static str {"Account Actions"}
	const fn apply() -> impl IntoObserverSystem<_,_,_> {logout}
}
trait SettingsButton: SettingsItem {
	fn get_btn_label() -> &'static str;
}
impl SettingsButton for LogoutButton {
	const fn get_btn_label() -> &'static str {"Logout"}
}

#[derive(Component)]
struct LogoutMessage;

trait DropdownOptions<T, const N: usize>: SettingsItem<Output=u8> {
	fn get_options(game_settings: &GameSettings) -> (T, [T; N]);
	fn get_selection(game_settings: &GameSettings) -> T;
	fn get_index(item: T) -> u8;
	fn display(item: T) -> &'static str;
}

impl DropdownOptions<MonitorSelection, 2> for DefaultMonitor {
	const fn get_options(game_settings: &GameSettings) -> (MonitorSelection, [MonitorSelection; 2]) {
		match game_settings.default_monitor {
			1 => (MonitorSelection::Primary, [MonitorSelection::Current, MonitorSelection::Index(game_settings.monitor_index)]),
			2 => (MonitorSelection::Index(game_settings.monitor_index), [MonitorSelection::Current, MonitorSelection::Primary]),
			_ => (MonitorSelection::Current, [MonitorSelection::Primary, MonitorSelection::Index(game_settings.monitor_index)])
		}
	}

	const fn get_selection(game_settings: &GameSettings) -> MonitorSelection {
		match game_settings.default_monitor {
			1 => MonitorSelection::Current,
			2 => MonitorSelection::Index(game_settings.monitor_index),
			_ => MonitorSelection::Primary
		}
	}

	const fn get_index(item: MonitorSelection) -> u8 {
		match item {
			MonitorSelection::Current => 1,
			MonitorSelection::Index(_) => 2,
			MonitorSelection::Primary => 0,
			MonitorSelection::Entity(_) => unreachable!("MonitorSelection::Entity should never be constructed")
		}
	}

	const fn display(item: MonitorSelection) -> &'static str {
		match item {
			MonitorSelection::Current => "Current",
			MonitorSelection::Index(_) => "Index",
			MonitorSelection::Primary => "Primary",
			MonitorSelection::Entity(_) => unreachable!("MonitorSelection::Entity should never be constructed")
		}
	}
}

fn update_checkbox(
	mut checkboxes: Query<(&Interaction, &mut Checkbox, &mut BackgroundColor, &Children), (Changed<Interaction>, With<Checkbox>)>,
	mut texts: Query<&mut Text>
) {
	for (interaction, mut checkbox, mut bg_color, children) in checkboxes.iter_mut() {
		if *interaction == Interaction::Pressed {
			checkbox.0 = !checkbox.0;
			bg_color.0 = if checkbox.0 { PALLET.accent_gold } else { PALLET.dark_brown };

			if let Ok(mut text) = texts.get_mut((*children)[0]) {
				text.0 = if checkbox.0 { "✓".to_string() } else { String::new() };
			}
		}
	}
}

fn update_spinbox(
	mut spinboxes: Query<(&Interaction, &mut SpinBox, &Children), Changed<Interaction>>,
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut texts: Query<&mut Text>
) {
	for (interaction, mut spinbox, children) in spinboxes.iter_mut() {
		if *interaction == Interaction::Pressed {
			// Simple increment/decrement logic - could be enhanced with proper input handling
			if keyboard_input.pressed(KeyCode::ArrowUp) {
				spinbox.0 = spinbox.0.saturating_add(1);
			} else if keyboard_input.pressed(KeyCode::ArrowDown) {
				spinbox.0 = spinbox.0.saturating_sub(1);
			}

			if let Ok(mut text) = texts.get_mut((*children)[0]) {
				text.0 = spinbox.0.to_string();
			}
		}
	}
}

fn update_dropdown() {
	// Dropdown cycling logic would go here
	// This would cycle through available options when clicked
}

fn update_slider(
	mut sliders: Query<(&Interaction, &mut Slider, &Children), Changed<Interaction>>,
	mut thumbs: Query<&mut Node, Without<Slider>>,
	mut value_displays: Query<&mut Text>
) {
	// Slider drag logic would go here
	// This would update the thumb position and value display
}

fn update_button(
	mut buttons: Query<(&Interaction, &mut BackgroundColor, &mut Node), (Changed<Interaction>, With<LogoutButton>)>
) {
	for (interaction, mut bg_color, mut node) in buttons.iter_mut() {
		match *interaction {
			Interaction::None => {
				bg_color.0 = PALLET.secondary_green;
				node.top = Val::Px(0.);
			}
			Interaction::Hovered => {
				bg_color.0 = PALLET.accent_gold;
				node.top = Val::Px(-2.);
			}
			Interaction::Pressed => {
				node.top = Val::Px(0.);
			}
		}
	}
}

fn update_keybind(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut modal_display: Query<(&mut Text, &ChildOf), With<KeybindModalDisplay>>,
	modal: Query<&Node, With<KeybindModalOverlay>>
) {
	let modal_node = modal.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	if modal_node.display == Display::Flex {
		// Capture key presses when modal is open
		for key in keyboard_input.get_just_pressed() {
			if let Ok((mut text, _)) = modal_display.single_mut() {
				text.0 = format!("{:?}", key);
			}
		}
	}
}