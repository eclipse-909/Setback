use {
	std::{
		io::ErrorKind,
		fs::{self, File},
		path::Path,
		sync::Arc
	},
	crate::{
		components::*,
		constants::*,
		resources::*,
		states::*,
	},
	bevy::{
		prelude::*, tasks::prelude::*, winit::cursor::CursorIcon
	},
	libsetback::*,
	keyring::{default, credential, Entry},
	reqwest::Client
};

pub struct MainMenuPlugin;

impl UiScene for MainMenuPlugin {
	const SCENE: Scene = Scene::MainMenu;
}

impl Plugin for MainMenuPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_systems(OnEnter(Self::SCENE), (on_enter, set_main_buttons).chain())
			.add_systems(Update, update_menu_button.run_if(in_state(Self::SCENE)))
			.add_systems(OnExit(Self::SCENE), Self::on_exit);
	}
}

//TODO figure out how to do color gradients, letter-spacing, and CSS ::before
fn on_enter(mut commands: Commands, asset_server: Res<AssetServer>) {
	let font: Handle<Font> = asset_server.load("fonts/georgia.ttf");
	let font_bold: Handle<Font> = asset_server.load("fonts/georgiab.ttf");
	let times: Handle<Font> = asset_server.load("fonts/times.ttf");
	// Create the main background with a green felt texture
	commands.spawn((
		Node {
			width: Val::Percent(100.),
			height: Val::Percent(100.),
			margin: UiRect::all(Val::Px(0.)),
			padding: UiRect::all(Val::Px(0.)),
			box_sizing: BoxSizing::BorderBox,
			min_height: Val::Vh(100.),
			display: Display::Flex,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			overflow: Overflow::hidden(),
			..default()
		},
		BackgroundColor(PALLET.primary_green),
		RootNode
	)).with_children(|commands| {
		//Suit icons
		commands.spawn((
			Node {
				top: Val::Px(20.),
            	left: Val::Px(20.),
            	position_type: PositionType::Absolute,
            	justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♠"),
				TextFont::from_font(times.clone()).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		commands.spawn((
			Node {
				top: Val::Px(20.),
            	right: Val::Px(20.),
             	position_type: PositionType::Absolute,
              	justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♦"),
				TextFont::from_font(times.clone()).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		commands.spawn((
			Node {
				bottom: Val::Px(20.),
            	left: Val::Px(20.),
            	position_type: PositionType::Absolute,
             	justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♣"),
				TextFont::from_font(times.clone()).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		commands.spawn((
			Node {
				bottom: Val::Px(20.),
            	right: Val::Px(20.),
				position_type: PositionType::Absolute,
				justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♥"),
				TextFont::from_font(times).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		//menu
		commands.spawn((
			Node {
				display: Display::Flex,
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Center,
				max_width: Val::Px(500.),
				width: Val::Percent(90.),
				padding: UiRect::all(Val::Px(40.)),
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
			//logo
			commands.spawn(
				Node {
					display: Display::Flex,
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::Center,
					margin: UiRect::bottom(Val::Px(40.)),
					..default()
				}
			).with_children(|commands| {
				//image
				commands.spawn((
					Node {
						width: Val::Px(120.),
						height: Val::Px(120.),
						display: Display::Flex,
						align_items: AlignItems::Center,
						justify_content: JustifyContent::Center,
						margin: UiRect::bottom(Val::Px(20.)),
						position_type: PositionType::Relative,
						overflow: Overflow::visible(),
						..default()
					},
					BackgroundColor(PALLET.light_gold),
					BorderRadius::all(Val::Percent(50.)),
					Outline::new(Val::Px(4.), Val::Px(0.), PALLET.dark_gold),
					BoxShadow(vec![
						ShadowStyle {
							x_offset: Val::Px(0.),
							y_offset: Val::Px(8.),
							blur_radius: Val::Px(20.),
							color: PALLET.card_shadow,
							..default()
						}
					]),
					ImageNode::new(asset_server.load("images/cardsIcon.png"))
				));
				//title
				commands.spawn((
					Node {
						margin: UiRect::bottom(Val::Px(10.)),
						justify_content: JustifyContent::Center,
						..default()
					},
					children![(
						Text::new("Setback"),
						TextFont::from_font(font_bold.clone()).with_font_size(48.),
						TextColor(PALLET.accent_gold),
						TextLayout::new_with_justify(JustifyText::Center),
						TextShadow {
							offset: Vec2::new(3., 3.),
							color: PALLET.text_shadow
						}
					)]
				));
				//subtitle
				commands.spawn((
					Node {
						margin: UiRect::bottom(Val::Px(20.)),
						justify_content: JustifyContent::Center,
						..default()
					},
					children![(
						Text::new("Classic Card Game"),
						TextFont::from_font(asset_server.load("fonts/georgiai.ttf")).with_font_size(16.),
						TextColor(PALLET.light_gold),
						TextLayout::new_with_justify(JustifyText::Center),
						TextShadow {
							offset: Vec2::new(1., 1.),
							color: PALLET.text_shadow
						}
					)]
				));
			});
			//buttons
			commands.spawn((
				Node {
					display: Display::Flex,
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(16.),
					column_gap: Val::Px(16.),
					width: Val::Percent(100.),
					..default()
				},
				ButtonContainer
			));
			//version
			commands.spawn((
				Node {
					margin: UiRect::top(Val::Px(30.)),
					justify_content: JustifyContent::Center,
					..default()
				},
				children![(
					Text::new(Version::LATEST.to_string()),
					TextFont::from_font(font).with_font_size(12.),
					TextColor(PALLET.light_brown.with_alpha(0.7)),
					TextLayout::new_with_justify(JustifyText::Center)
				)]
			));
		});
	});
}

#[derive(Component)]
struct ButtonContainer;

#[derive(Component)]
enum MenuButton {
	Primary,
	ButtonGroup
}

fn new_primary_button(label: &str, font: Handle<Font>) -> impl Bundle {(
	Button,
	MenuButton::Primary,
	BackgroundColor(PALLET.accent_gold),
	Outline::new(Val::Px(2.), Val::Px(0.), PALLET.light_gold),
	BorderRadius::all(PALLET.border_radius),
	Node {
		padding: UiRect::axes(Val::Px(16.), Val::Px(20.)),
		position_type: PositionType::Relative,
		overflow: Overflow::hidden(),
		justify_content: JustifyContent::Center,
		..default()
	},
	BoxShadow(vec![
		ShadowStyle {
			x_offset: Val::Px(0.),
			y_offset: Val::Px(4.),
			blur_radius: Val::Px(15.),
			color: Color::BLACK.with_alpha(0.2),
			..default()
		}
	]),
	children![(
		Text::new(label),
		TextColor(PALLET.dark_brown),
		TextFont::from_font(font).with_font_size(18.),
		TextLayout::new_with_justify(JustifyText::Center)
	)]
	//TODO
	// transition: all 0.3s ease;
)}

fn new_group_button(label: &str, font: Handle<Font>) -> impl Bundle {(
	Button,
	MenuButton::ButtonGroup,
	BackgroundColor(PALLET.primary_green),
	Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
	BorderRadius::all(PALLET.border_radius),
	Node {
		padding: UiRect::axes(Val::Px(14.), Val::Px(12.)),
		position_type: PositionType::Relative,
		overflow: Overflow::hidden(),
		flex_grow: 1.,
		justify_content: JustifyContent::Center,
		..default()
	},
	BoxShadow(vec![
		ShadowStyle {
			x_offset: Val::Px(0.),
			y_offset: Val::Px(4.),
			blur_radius: Val::Px(15.),
			color: Color::BLACK.with_alpha(0.2),
			..default()
		}
	]),
	children![(
		Text::new(label),
		TextColor(PALLET.cream),
		TextFont::from_font(font).with_font_size(16.),
		TextLayout::new_with_justify(JustifyText::Center)
	)]
	//TODO
	// transition: all 0.3s ease;
)}

/// Creates singleplayer, multiplayer, settings, exit game buttons
fn set_main_buttons(
	mut commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>,
	asset_server: Res<AssetServer>
) {
	let font_bold: Handle<Font> = asset_server.load("fonts/georgiab.ttf");
	let children = &[
		//singleplayer
		commands
			.spawn(new_primary_button("SINGLEPLAYER", font_bold.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				//TODO
				// Go to setup scene
			})
			.id(),
		//multiplayer
		commands
			.spawn(new_primary_button("MULTIPLAYER", font_bold.clone()))
			.observe(set_multiplayer_buttons)
			.id(),
		//button group
		commands.spawn(
			Node {
				display: Display::Flex,
				row_gap: Val::Px(12.),
				column_gap: Val::Px(12.),
				width: Val::Percent(100.),
				..default()
			}
		).with_children(|commands| {
			//settings
			commands
				.spawn(new_group_button("SETTINGS", font_bold.clone()))
				.observe(|trigger: Trigger<Pointer<Click>>| {
					if !matches!(trigger.button, PointerButton::Primary) {return}
					//TODO
					// Go to settings scene
				});
			//exit
			commands
				.spawn(new_group_button("EXIT GAME", font_bold.clone()))
				.observe(|trigger: Trigger<Pointer<Click>>| {
					if !matches!(trigger.button, PointerButton::Primary) {return}
					std::process::exit(0);
				});
		}).id()
	];
	let mut container = commands.entity(button_container.single().expect("There should be exactly one ButtonContainer"));
	container.despawn_related::<Children>();
	container.add_children(children);
}

/// This is an observer-function only
///
/// If logged in,
/// creates buttons for create game, join game, and back.
///
/// Else,
/// attempts to login to create buttons or prompt for credentials.
fn set_multiplayer_buttons(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>,
	mut server_connection: Option<ResMut<ServerConn>>,
	website_connection: Option<Res<WebsiteConn>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	let thread_pool = AsyncComputeTaskPool::get();
	let website_conn = match website_connection {
		Some(conn) => conn.0.clone(),
		None => {
			let api = Arc::new(Client::new_connection());
			commands.insert_resource(WebsiteConn(api.clone()));
			api
		}
	};
	match server_connection {
		Some(conn) => {
			let server_conn = conn.0.clone();
			let task = thread_pool.spawn(async move {
			
			});
		}
		None => {
			let Some(cred) = get_stored_credentials() else {
				todo!("prompt for credentials");
			};
			let task = thread_pool.spawn(async move {
				match ServerConnection::new_connection(cred.username, cred.password, website_conn).await {
					Ok(server_conn) => {}
					Err(Error::Authentication) => {}
					Err(Error::NetworkIO) => {}
					Err(Error::ServerError) => {}
					Err(_) => unreachable!("All errors should have been covered")
				}
			});
		}
	};

	//TODO
	//Check if the user is connected to the server and logged in.
	//If they are, display buttons for create game, join game, and back.
	//If they aren't, do the following:
	//Establish a connection to the server.
	//Check if the user's credentials are securely stored on the local machine.
	//If the user has stored credentials and has chosen to automatically login, use them to login.
	//If the user doesn't have credentials or isn't automatically logging in, prompt for the credentials.
	//Once the credentials are obtained, send a login request to the server.
	//If the login fails, prompt for credentials.
	//Eventually I should handle too many failed attempts, but I'll do that later.
	//If the login succeeds, go to the multiplayer scene.

	let children = &[
		//TODO
	];
	let mut container = commands.entity(button_container.single().expect("There should be exactly one ButtonContainer"));
	container.despawn_related::<Children>();
	container.add_children(children);
}

/// Creates game code input, join button, and back button
fn set_join_game(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	//TODO
	let children = &[
		//TODO
	];
	let mut container = commands.entity(button_container.single().expect("There should be exactly one ButtonContainer"));
	container.despawn_related::<Children>();
	container.add_children(children);
}

fn set_login_prompt(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	//TODO
	let children = &[
		//TODO
	];
	let mut container = commands.entity(button_container.single().expect("There should be exactly one ButtonContainer"));
	container.despawn_related::<Children>();
	container.add_children(children);
}

fn get_stored_credentials() -> Option<LoginCredentials> {
	let path = Path::new(USR_CRD);
	let username: String = match fs::read_to_string(path) {
		Ok(username) => username,
		Err(err) => {
			if matches!(err.kind(), ErrorKind::NotFound) {
				let res = fs::create_dir_all(path.parent().unwrap());
				if let Err(err) = res {
					error!("Error creating usr directory: {}", err);
				}
				let res = File::create(USR_CRD);
				if let Err(err) = res {
					error!("Error creating usr.crd file: {}", err);
				}
			}
			return None;
		}
	};
	match Entry::new(KEYRING_PASSWORD, whoami::username().as_str()).and_then(|entry| Ok(entry.get_password()?)) {
		Ok(password) => {
			Some(LoginCredentials {username, password})
		}
		Err(err) => {
			error!("Error getting password from keyring: {}", err);
			None
		}
	}
}

fn set_stored_credentials(username: &str, password: &str) {
	let path = Path::new(USR_CRD);
	let res = fs::create_dir_all(path.parent().unwrap());
	if let Err(err) = res {
		error!("Error creating usr directory: {}", err);
	}
	let res = fs::write(path, username);
	if let Err(err) = res {
		error!("Error creating usr.crd file: {}", err);
	}
	let res = Entry::new(KEYRING_PASSWORD, whoami::username().as_str())
		.and_then(|entry| Ok(entry.set_password(password)?));
	if let Err(err) = res {
		error!("Error setting password in keyring: {}", err);
	}
}

fn update_menu_button(
	mut menu_buttons: Query<(
		&Interaction,
		&MenuButton,
		&mut BackgroundColor,
		&mut Node,
		&mut BoxShadow,
		&mut Outline,
		&Children
	), Changed<Interaction>>,
	mut text_colors: Query<&mut TextColor>,
	mut cursor_icon: Query<&mut CursorIcon>,
	mut cursor_atlas: ResMut<CursorAtlas>,
	mut default_cursor: Local<Option<Handle<Image>>>,
	asset_server: Res<AssetServer>
) {
	let Ok(mut cursor_icon) = cursor_icon.single_mut() else {return};
	if default_cursor.is_none() {
		*default_cursor = Some(asset_server.load(BJ_NORMAL_SELECT));
	}
	for (
		interaction,
		menu_button,
		mut bg_color,
		mut node,
		mut box_shadow,
		mut outline,
		children
	) in menu_buttons.iter_mut() {
		let child = if children.len() == 1 {children[0]} else {unreachable!("Node is supposed to have 1 child")};
		let mut text_color = text_colors.get_mut(child).expect("Child is supposed to have TextColors component");
		match *interaction {
			Interaction::None => {
				bg_color.0 = match *menu_button {
					MenuButton::Primary => PALLET.accent_gold,
					MenuButton::ButtonGroup => PALLET.primary_green,
				};
				text_color.0 = match *menu_button {
					MenuButton::Primary => PALLET.dark_brown,
					MenuButton::ButtonGroup => PALLET.cream,
				};
				node.top = Val::Px(0.);
				box_shadow.0 = vec![
					ShadowStyle {
						x_offset: Val::Px(0.),
						y_offset: Val::Px(4.),
						blur_radius: Val::Px(15.),
						color: Color::BLACK.with_alpha(0.2),
						..default()
					}
				];
				if matches!(*menu_button, MenuButton::Primary) {
					outline.color = PALLET.light_gold;
				}
				*cursor_icon = custom_cursor_icon(cursor_atlas.as_mut(), default_cursor.as_ref().unwrap().clone());
			}
			Interaction::Hovered => {
				bg_color.0 = match *menu_button {
					MenuButton::Primary => PALLET.light_gold,
					MenuButton::ButtonGroup => PALLET.accent_gold,
				};
				text_color.0 = PALLET.dark_brown;
				node.top = Val::Px(-2.);
				box_shadow.0 = vec![
					ShadowStyle {
						x_offset: Val::Px(0.),
						y_offset: Val::Px(6.),
						blur_radius: Val::Px(20.),
						color: Color::BLACK.with_alpha(0.3),
						..default()
					}
				];
				if matches!(*menu_button, MenuButton::Primary) {
					outline.color = PALLET.cream;
				}
				*cursor_icon = custom_cursor_anim(cursor_atlas.as_mut(), BJ_HELP_SELECT_PAGE);
			}
			Interaction::Pressed => {
				node.top = Val::Px(0.);
				box_shadow.0 = vec![
					ShadowStyle {
						x_offset: Val::Px(0.),
						y_offset: Val::Px(2.),
						blur_radius: Val::Px(10.),
						color: Color::BLACK.with_alpha(0.2),
						..default()
					}
				];
			}
		}
	}
}