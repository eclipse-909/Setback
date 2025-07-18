use {
	std::{
		io::ErrorKind,
		fs::{self, File},
		path::Path,
		task::{Context, Poll},
		sync::{Arc, RwLock}
	},
	crate::{
		components::*,
		constants::*,
		resources::*,
		states::*,
		plugins::*
	},
	bevy::{
		prelude::*,
		tasks::{
			prelude::*,
			Task
		},
		ecs::world::CommandQueue
	},
	bevy_ui_text_input::{TextInputMode, TextInputNode, TextInputPrompt, TextInputFilter},
	futures::FutureExt,
	libsetback::*,
	keyring::Entry
};

pub struct MainMenuPlugin;

impl UiScenePlugin for MainMenuPlugin {
	const SCENE: UiScene = UiScene::MainMenu;
}

impl Plugin for MainMenuPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_systems(Startup, check_for_update)
			.add_systems(OnEnter(Self::SCENE), (on_enter, display_main_buttons).chain())
			.add_systems(Update, update_menu_button.run_if(in_state(Self::SCENE)))
			.add_systems(OnExit(Self::SCENE), Self::on_exit);
	}
}

fn check_for_update() {
	//TODO
	// Use WebsiteConnection to check the latest version and see if there is an update to the game.
	// Display a popup if the game can be updated.
}

//TODO figure out how to do color gradients, letter-spacing, and CSS ::before
fn on_enter(mut commands: Commands, asset_server: Res<AssetServer>) {
	info!("Building Main Menu UI");
	let font: Handle<Font> = asset_server.load(GEORGIA);
	let font_bold: Handle<Font> = asset_server.load(GEORGIA_BOLD);
	let times: Handle<Font> = asset_server.load(TIMES_NEW_ROMAN);
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
						TextFont::from_font(asset_server.load(GEORGIA_ITALIC)).with_font_size(16.),
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

macro_rules! box_shadow_styles {
    () => {
	    vec![
			ShadowStyle {
				x_offset: Val::Px(0.),
				y_offset: Val::Px(4.),
				blur_radius: Val::Px(15.),
				color: Color::BLACK.with_alpha(0.2),
				..default()
			}
		]
    };
}

fn new_primary_button(label: &str, font_bold: Handle<Font>, cursor: Handle<Image>) -> impl Bundle {(
	Button,
	MenuButton::Primary,
	CursorComponent {
		default: CursorStyle::Icon(cursor),
		pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
	},
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
	BoxShadow(box_shadow_styles!()),
	children![(
		Text::new(label),
		TextColor(PALLET.dark_brown),
		TextFont::from_font(font_bold).with_font_size(18.),
		TextLayout::new_with_justify(JustifyText::Center)
	)]
	//TODO
	// transition: all 0.3s ease;
)}

fn new_group_button(label: &str, font_bold: Handle<Font>, cursor: Handle<Image>) -> impl Bundle {(
	Button,
	MenuButton::ButtonGroup,
	CursorComponent {
		default: CursorStyle::Icon(cursor),
		pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
	},
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
	BoxShadow(box_shadow_styles!()),
	children![(
		Text::new(label),
		TextColor(PALLET.cream),
		TextFont::from_font(font_bold).with_font_size(16.),
		TextLayout::new_with_justify(JustifyText::Center)
	)]
	//TODO
	// transition: all 0.3s ease;
)}

fn new_input(prompt: &str, font: Handle<Font>, action: Option<fn(&mut World)>) -> impl Bundle {(
	Node {
		width: Val::Percent(100.),
		height: Val::Percent(100.),
		justify_content: JustifyContent::Center,
		align_items: AlignItems::Center,
		flex_direction: FlexDirection::Column,
		row_gap: Val::Px(10.),
		column_gap: Val::Px(20.),
		..default()
	},
	Outline::new(Val::Px(2.), Val::Px(0.), PALLET.accent_gold),
	BorderRadius::all(PALLET.border_radius),
	BoxShadow(box_shadow_styles!()),
	children![(
		Node {
            width: Val::Px(250.),//TODO mess with layout and style
            height: Val::Px(25.),
            ..default()
        },
		TextInputNode {
			clear_on_submit: false,
            mode: TextInputMode::SingleLine,
			filter: Some(TextInputFilter::Hex),
            max_chars: Some(6),
            ..default()
        },
		TextColor(PALLET.dark_brown),
		TextFont::from_font(font).with_font_size(18.),
		TextInputPrompt::new(prompt),
        BackgroundColor(PALLET.cream),
		TextInputOnSubmitAction(action)
	)]
)}

fn new_text(label: &str, font: Handle<Font>, visible: bool) -> impl Bundle {(
	Node {
		padding: UiRect::axes(Val::Px(16.), Val::Px(20.)),
		position_type: PositionType::Relative,
		overflow: Overflow::hidden(),
		justify_content: JustifyContent::Center,
		display: if visible {Display::default()} else {Display::None},
		..default()
	},
	children![(
		Text::new(label),
		TextColor(PALLET.dark_brown),
		TextFont::from_font(font).with_font_size(18.),
		TextLayout::new_with_justify(JustifyText::Center)
	)]
)}

/// Creates singleplayer, multiplayer, settings, exit game buttons
fn display_main_buttons(
	mut commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>,
	asset_server: Res<AssetServer>
) {
	let font_bold: Handle<Font> = asset_server.load(GEORGIA_BOLD);
	let cursor: Handle<Image> = asset_server.load(BJ_NORMAL_SELECT);
	let children = &[
		//singleplayer
		commands
			.spawn(new_primary_button("SINGLEPLAYER", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, mut next_scene: ResMut<NextState<UiScene>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				info!("Set scene to Setup");
				next_scene.set(UiScene::Setup);
			})
			.id(),
		//multiplayer
		commands
			.spawn(new_primary_button("MULTIPLAYER", font_bold.clone(), cursor.clone()))
			.observe(display_multiplayer_observer)
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
				.spawn(new_group_button("SETTINGS", font_bold.clone(), cursor.clone()))
				.observe(|trigger: Trigger<Pointer<Click>>, mut next_scene: ResMut<NextState<UiScene>>| {
					if !matches!(trigger.button, PointerButton::Primary) {return}
					info!("Set scene to Settings");
					next_scene.set(UiScene::Settings);
				});
			//exit
			commands
				.spawn(new_group_button("EXIT GAME", font_bold, cursor))
				.observe(|trigger: Trigger<Pointer<Click>>| {
					if !matches!(trigger.button, PointerButton::Primary) {return}
					std::process::exit(0);
				});
		}).id()
	];
	let mut container = commands.entity(button_container.single().expect(BTN_CONTAINER_ERR));
	container.despawn_related::<Children>();
	container.add_children(children);
}

fn display_multiplayer_options(mut commands: Commands, button_container: Entity, font_bold: Handle<Font>, cursor: Handle<Image>) {
	let children = &[
		commands
			.spawn(new_primary_button("CREATE GAME", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, mut next_scene: ResMut<NextState<UiScene>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				info!("Creating game");
				info!("Set scene to Setup");
				next_scene.set(UiScene::Setup);
			})
			.id(),
		commands
			.spawn(new_primary_button("JOIN PRIVATE GAME", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, commands: Commands, button_container: Query<Entity, With<ButtonContainer>>, asset_server: Res<AssetServer>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				display_join_options(
					commands,
					button_container.single().expect(BTN_CONTAINER_ERR),
					asset_server,
					false
				);
			})
			.id(),
		commands
			.spawn(new_primary_button("BROWSE PUBLIC GAMES", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, mut next_scene: ResMut<NextState<UiScene>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				info!("Set scene to Multiplayer");
				next_scene.set(UiScene::Multiplayer);
			})
			.id(),
		commands
			.spawn(new_group_button("BACK", font_bold.clone(), cursor))
			.observe(display_multiplayer_observer)
			.id()
	];
	let mut button_container = commands.entity(button_container);
	button_container.despawn_related::<Children>();
	button_container.add_children(children);
}

fn display_join_options(mut commands: Commands, button_container: Entity, asset_server: Res<AssetServer>, failed: bool) {
	static FAILED_MSG: &str = "Could not find game with matching code";
	if failed {info!(FAILED_MSG);}
	let font = asset_server.load(GEORGIA);
	let font_bold = asset_server.load(GEORGIA_BOLD);
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	let children = &[
		commands
			.spawn(new_text(FAILED_MSG, font.clone(), failed))
			.id(),
		commands
			.spawn(new_input("Enter game code...", font, Some(|world: &mut World| {
				join_game();
			})))
			.id(),
		commands
			.spawn(new_primary_button("JOIN", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				join_game();
			})
			.id(),
		commands
			.spawn(new_group_button("BACK", font_bold.clone(), cursor))
			.observe(display_multiplayer_options_observer)
			.id()
	];
	let mut button_container = commands.entity(button_container);
	button_container.despawn_related::<Children>();
	button_container.add_children(children);
}

fn display_login_prompt(mut commands: Commands, button_container: Entity, font: Handle<Font>, font_bold: Handle<Font>, cursor: Handle<Image>, failed: bool) {
	static FAILED_MSG: &str = "Username or password was incorrect";
	if failed {info!(FAILED_MSG);}
	let children = &[
		commands
			.spawn(new_text(FAILED_MSG, font.clone(), failed))
			.id(),
		commands
			.spawn(new_input("Username...", font.clone(), None))
			.id(),
		commands
			.spawn(new_input("Password...", font.clone(), Some(|world: &mut World| {
				let api = world.resource::<WebsiteConn>();
				login(api);
			})))
			.id(),
		commands
			.spawn(new_primary_button("LOGIN", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, api: Res<WebsiteConn>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				login(api.as_ref());
			})
			.id(),
		commands
			.spawn(new_text("Don't have an account? Create one!", font.clone(), true))
			.id(),
		commands
			.spawn((
				Button,
				Url(website!(URL_ACCOUNT)),
				Node {
					padding: UiRect::axes(Val::Px(16.), Val::Px(20.)),//TODO mess with layout
					position_type: PositionType::Relative,
					overflow: Overflow::hidden(),
					justify_content: JustifyContent::Center,
					..default()
				},
				children![(
					Text::new(website!(URL_ACCOUNT)),
					TextColor(PALLET.url),
					TextFont::from_font(font).with_font_size(18.),
					TextLayout::new_with_justify(JustifyText::Center)
				)]
			))
			.observe(|trigger: Trigger<Pointer<Click>>, url: Query<&Url>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				let Ok(url) = url.get(trigger.target) else {return};
				if open::that_detached(url.0.as_str()).is_ok() {
					info!("Opened /account in browser");
				} else {
					error!("Failed to open /account in browser");
				}
			})
			.id(),
		commands
			.spawn(new_group_button("BACK", font_bold, cursor))
			.observe(display_main_buttons_observer)
			.id()
	];
	let mut button_container = commands.entity(button_container);
	button_container.despawn_related::<Children>();
	button_container.add_children(children);
}

fn display_main_buttons_observer(
	trigger: Trigger<Pointer<Click>>,
	commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>,
	asset_server: Res<AssetServer>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	display_main_buttons(commands, button_container, asset_server)
}

/// This is an observer-function only
///
/// If logged in,
/// creates buttons for create game, join game, and back.
///
/// Else,
/// attempts to login to create buttons or prompt for credentials.
fn display_multiplayer_observer(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>,
	server_connection: Option<ResMut<ServerConn>>,
	website_connection: Res<WebsiteConn>,
	task_schedule: ResMut<TaskSchedule>,
	asset_server: Res<AssetServer>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	let font_bold: Handle<Font> = asset_server.load(GEORGIA_BOLD);
	let font: Handle<Font> = asset_server.load(GEORGIA);
	let cursor: Handle<Image> = asset_server.load(BJ_NORMAL_SELECT);
	let thread_pool = AsyncComputeTaskPool::get();
	let website_conn = website_connection.0.clone();
	let container_entity = button_container.single().expect(BTN_CONTAINER_ERR);
	let mut container = commands.entity(container_entity);
	match server_connection {
		Some(_) => {
			info!("Already connected to server");
			display_multiplayer_options(commands, container_entity, font_bold, cursor);
		}
		None => {
			info!("Not connected to server; attempting to connect to server");
			let Some(cred) = get_credentials() else {
				info!("Prompting for credentials");
				display_login_prompt(commands, container_entity, font, font_bold, cursor, false);
				return;
			};
			let children = &[
				//TODO display loading animation
			];
			container.despawn_related::<Children>();
			container.add_children(children);
			let mut task: Task<CommandQueue> = thread_pool.spawn(async move {
				info!("Attempting to connect to server");
				let res = ServerConnection::new_connection(cred.username.clone(), cred.password.clone(), website_conn).await;
				let mut command_queue = CommandQueue::default();
				
				command_queue.push(move |world: &mut World| {
					//remove loading animation
					let btn_container_entity = world
						.query_filtered::<Entity, With<ButtonContainer>>()
						.single(world)
						.expect(BTN_CONTAINER_ERR);
					let mut commands = world.commands();
					
					match res {
						Ok(server_conn) => {
							info!("Successfully connected to server");
							display_multiplayer_options(commands, btn_container_entity, font_bold, cursor);
							world.insert_resource(ServerConn(Arc::new(RwLock::new(server_conn))));
							save_credentials(cred.username.as_str(), cred.password.as_str());
						}
						Err(Error::Authentication) => {
							display_login_prompt(commands, btn_container_entity, font, font_bold, cursor,  true);
						}
						Err(Error::NetworkIO) => {
							let children = &[
								commands
									.spawn(new_text("Network Error - make sure you are connected to the internet, or try again later.", font, true))
									.id(),
								commands
									.spawn(new_group_button("BACK", font_bold, cursor))
									.observe(display_main_buttons_observer)
									.id()
							];
							let mut btn_container = commands.entity(btn_container_entity);
							btn_container.despawn_related::<Children>();
							btn_container.add_children(children);
						}
						Err(Error::ServerError) => {
							let children = &[
								commands
									.spawn(new_text("Server Error - an unexpected error has occurred on the server. Please try again later.", font, true))
									.id(),
								commands
									.spawn(new_group_button("BACK", font_bold, cursor))
									.observe(display_main_buttons_observer)
									.id()
							];
							let mut btn_container = commands.entity(btn_container_entity);
							btn_container.despawn_related::<Children>();
							btn_container.add_children(children);
						}
						Err(Error::IncompatibleVersion) => {
							todo!("display message to update game")
						}
						Err(_) => unreachable!("All errors should have been covered")
					}
				});
				
				command_queue
			});
			//task must be polled once to start execution.
			if let Poll::Ready(mut command_queue) = task.poll_unpin(&mut Context::from_waker(&futures::task::noop_waker())) {
				//check if it's ready just in case (it probably won't be)
				commands.append(&mut command_queue);
			} else {
				task_schedule.0.push(task);
			}
		}
	};
}

/// Creates game code input, join button, and back button
fn display_multiplayer_options_observer(
	trigger: Trigger<Pointer<Click>>,
	commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>,
	asset_server: Res<AssetServer>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	display_multiplayer_options(commands, button_container.single().expect(BTN_CONTAINER_ERR), asset_server.load(GEORGIA_BOLD), asset_server.load(BJ_NORMAL_SELECT));
}

fn get_credentials() -> Option<LoginCredentials> {
	let path = Path::new(USR_CRD);
	let username: String = match fs::read_to_string(path) {
		Ok(username) => username,
		Err(err) => {
			if matches!(err.kind(), ErrorKind::NotFound) {
				if let Err(err) = fs::create_dir_all(path.parent().unwrap()) {
					error!("Error creating usr directory: {}", err);
					return None;
				}
				if let Err(err) = File::create(USR_CRD) {
					error!("Error creating usr.crd file: {}", err);
				}
			}
			return None;
		}
	};
	match Entry::new(KEYRING_PASSWORD, whoami::username().as_str()).and_then(|entry| entry.get_password()) {
		Ok(password) => {
			info!("Retrieved credentials");
			Some(LoginCredentials {username, password})
		}
		Err(err) => {
			error!("Error getting password from keyring: {}", err);
			None
		}
	}
}

fn save_credentials(username: &str, password: &str) {
	let path = Path::new(USR_CRD);
	if let Err(err) = fs::create_dir_all(path.parent().unwrap()) {
		error!("Error creating usr directory: {}", err);
		return;
	}
	if let Err(err) = fs::write(path, username) {
		error!("Error creating usr.crd file: {}", err);
		return;
	}
	if let Err(err) = Entry::new(KEYRING_PASSWORD, whoami::username().as_str()).and_then(|entry| entry.set_password(password)) {
		error!("Error setting password in keyring: {}", err);
		return;
	}
	info!("Saved credentials successfully");
}

fn join_game() {
	//TODO
	// Attempt to join game
	info!("Attempting to join game");
}
fn login(api: &WebsiteConn) {
	//TODO
	// Attempt to login
	info!("Attempting to login");
}

#[allow(clippy::type_complexity)]
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
	mut text_colors: Query<&mut TextColor>
) {
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
				box_shadow.0 = box_shadow_styles!();
				if matches!(*menu_button, MenuButton::Primary) {
					outline.color = PALLET.light_gold;
				}
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