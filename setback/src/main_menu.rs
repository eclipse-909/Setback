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
		plugins::*,
		systems::*
	},
	bevy::{
		prelude::*,
		ecs::{
			world::CommandQueue,
			query::QuerySingleError
		}
	},
	bevy_ui_text_input::{TextInputMode, TextInputNode, TextInputPrompt, TextInputFilter, TextInputBuffer},
	libsetback::*,
	keyring::Entry,
	reqwest::Client,
	tokio::sync::{
		mpsc::UnboundedSender,
		Mutex
	},
	regex::Regex
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

fn check_for_update(runtime: Res<TokioRuntime>, api: Res<WebsiteConn>) {
	let api = api.0.clone();
	let tx = runtime.tx.clone();
	runtime.rt.spawn(async move {
		let mut command_queue = CommandQueue::default();
		let Ok(latest_version) = api.update().await else {return};
		command_queue.push(move |world: &mut World| {
			let btn_container = match world.query_filtered::<Entity, With<ButtonContainer>>().single(world) {
				Ok(container) => container,
				// If there isn't a root node, it means the api call returned before the scene loaded.
				// This should be incredibly unlikely, so I'm gonna handle this by not displaying the popup.
				// The popup will instead be displayed if the user clicks multiplayer.
				Err(QuerySingleError::NoEntities(_)) => {
					warn!("Button container does not exist yet");
					return;
				}
				Err(QuerySingleError::MultipleEntities(_)) => panic!("{}", SINGLE_COMPONENT_QUERY_ERROR),
			};
			let asset_server = world.resource::<AssetServer>();
			let font = asset_server.load(GEORGIA);
			let font_bold = asset_server.load(GEORGIA_BOLD);
			let cursor = asset_server.load(BJ_NORMAL_SELECT);
			let mut commands = world.commands();
			display_update_game(commands.entity(btn_container), latest_version, font, font_bold, cursor);
		});
		tx.send(command_queue).expect(TASK_ERROR);
	});
}

//TODO figure out how to do color gradients, letter-spacing, and CSS ::before
fn on_enter(mut commands: Commands, asset_server: Res<AssetServer>) {
	info!("Entering Main Menu");
	let font: Handle<Font> = asset_server.load(GEORGIA);
	let font_bold: Handle<Font> = asset_server.load(GEORGIA_BOLD);
	let times: Handle<Font> = asset_server.load(TIMES_NEW_ROMAN);
	// Create the main background with a green felt texture
	menu_background(&mut commands, times, |commands| {
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
					Text::new(SETBACK_VERSION.to_string()),
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

fn new_input(prompt: &str, font: Handle<Font>, marker: impl Component, filter: Option<&'static Regex>, action: Option<fn(&mut World)>) -> impl Bundle {(
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
			filter: filter.map::<Box<dyn TextInputFilter>, _>(|regex|Box::new(|text|regex.is_match(text))),
            max_chars: Some(6),
            ..default()
        },
		TextColor(PALLET.dark_brown),
		TextFont::from_font(font).with_font_size(18.),
		TextInputPrompt::new(prompt),
        BackgroundColor(PALLET.cream),
		TextInputOnSubmitAction(action),
		marker
	)],
)}

fn new_text(label: &str, font: Handle<Font>) -> impl Bundle {(
	Node {
		padding: UiRect::axes(Val::Px(16.), Val::Px(4.)),
		position_type: PositionType::Relative,
		justify_content: JustifyContent::Center,
		..default()
	},
	children![(
		Text::new(label),
		TextColor(PALLET.cream),
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
	let mut container = commands.entity(button_container.single().expect(SINGLE_COMPONENT_QUERY_ERROR));
	container.despawn_related::<Children>();
	container.with_children(|commands| {
		//multiplayer
		commands
			.spawn(new_primary_button("MULTIPLAYER", font_bold.clone(), cursor.clone()))
			.observe(display_multiplayer_observer);
		//singleplayer
		commands
			.spawn(new_primary_button("SINGLEPLAYER", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, mut next_scene: ResMut<NextState<UiScene>>, mut next_conn_type: ResMut<NextState<ConnectionType>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				next_scene.set(UiScene::Setup);
				next_conn_type.set(ConnectionType::Singleplayer);
			});
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
					next_scene.set(UiScene::Settings);
				});
			//exit
			commands
				.spawn(new_group_button("EXIT GAME", font_bold, cursor))
				.observe(|trigger: Trigger<Pointer<Click>>| {
					if !matches!(trigger.button, PointerButton::Primary) {return}
					std::process::exit(0);
				});
		});
	});
}

fn display_multiplayer_options(mut button_container: EntityCommands, font_bold: Handle<Font>, cursor: Handle<Image>) {
	button_container.despawn_related::<Children>();
	button_container.with_children(|commands| {
		let font_bold_clone = font_bold.clone();
		let cursor_clone = cursor.clone();
		commands
			.spawn(new_primary_button("CREATE PRIVATE GAME", font_bold.clone(), cursor.clone()))
			.observe(move |trigger: Trigger<Pointer<Click>>, mut commands: Commands, button_container: Query<Entity, With<ButtonContainer>>, asset_server: Res<AssetServer>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				let mut btn_container = commands.entity(button_container.single().expect(SINGLE_COMPONENT_QUERY_ERROR));
				btn_container.despawn_related::<Children>();
				btn_container.with_children(|commands| {
					commands
						.spawn(new_input(
							"Enter game password...",
							asset_server.load(GEORGIA),
							GamePasswordInput,
							Some(&*GAME_CODE_REGEX),
							Some(|world: &mut World| {
								let btn_container_entity = world
									.query_filtered::<Entity, With<ButtonContainer>>()
									.single(world)
									.expect(SINGLE_COMPONENT_QUERY_ERROR);
								let mut password = world.query_filtered::<&TextInputBuffer, With<GamePasswordInput>>();
								let password = password.single(world).expect(SINGLE_COMPONENT_QUERY_ERROR).get_text();
								let TokioRuntime {rt, tx} = world.resource::<TokioRuntime>();
								let rt = rt.handle().clone();
								let tx = tx.clone();
								let server = world.resource::<ServerConn>().0.clone();
								let loading = world.resource::<AssetServer>().load(LOADING);
								let mut commands = world.commands();
								let container = commands.entity(btn_container_entity);
								create_game(container, rt, tx, server, Some(password), loading);
							}
						)));
					commands
						.spawn(new_primary_button("CREATE GAME", font_bold_clone.clone(), cursor_clone.clone()))
						.observe(|
							trigger: Trigger<Pointer<Click>>,
							server: Res<ServerConn>,
							runtime: Res<TokioRuntime>,
							password: Query<&TextInputBuffer, With<GamePasswordInput>>,
							btn_container_entity: Query<Entity, With<ButtonContainer>>,
							asset_server: Res<AssetServer>,
							mut commands: Commands
						| {
							if !matches!(trigger.button, PointerButton::Primary) {return}
							create_game(
								commands.entity(btn_container_entity.single().expect(SINGLE_COMPONENT_QUERY_ERROR)),
								runtime.rt.handle().clone(),
								runtime.tx.clone(),
								server.0.clone(),
								Some(password.single().expect(SINGLE_COMPONENT_QUERY_ERROR).get_text()),
								asset_server.load(LOADING)
							);
						});
					commands
						.spawn(new_group_button("BACK", font_bold_clone.clone(), cursor_clone.clone()))
						.observe(display_multiplayer_options_observer);
				});
			});
		commands
			.spawn(new_primary_button("CREATE PUBLIC GAME", font_bold.clone(), cursor.clone()))
			.observe(|
				trigger: Trigger<Pointer<Click>>,
				mut commands: Commands,
				button_container: Query<Entity, With<ButtonContainer>>,
				runtime: Res<TokioRuntime>,
				server: Res<ServerConn>,
				asset_server: Res<AssetServer>
			| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				create_game(
					commands.entity(button_container.single().expect(SINGLE_COMPONENT_QUERY_ERROR)),
					runtime.rt.handle().clone(),
					runtime.tx.clone(),
					server.0.clone(),
					None,
					asset_server.load(LOADING)
				);
			});
		commands
			.spawn(new_primary_button("JOIN PRIVATE GAME", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, commands: Commands, button_container: Query<Entity, With<ButtonContainer>>, asset_server: Res<AssetServer>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				display_join_options(
					commands,
					button_container.single().expect(SINGLE_COMPONENT_QUERY_ERROR),
					asset_server,
					false
				);
			});
		commands
			.spawn(new_primary_button("BROWSE PUBLIC GAMES", font_bold.clone(), cursor.clone()))
			.observe(|trigger: Trigger<Pointer<Click>>, mut next_scene: ResMut<NextState<UiScene>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				next_scene.set(UiScene::Multiplayer);
			});
		commands
			.spawn(new_group_button("BACK", font_bold.clone(), cursor))
			.observe(display_multiplayer_observer);
	});
}

fn display_join_options(mut commands: Commands, button_container: Entity, asset_server: Res<AssetServer>, failed: bool) {
	static FAILED_MSG: &str = "Could not find game with matching code";
	if failed {warn!(FAILED_MSG);}
	let font = asset_server.load(GEORGIA);
	let font_bold = asset_server.load(GEORGIA_BOLD);
	let cursor = asset_server.load(BJ_NORMAL_SELECT);
	let mut button_container = commands.entity(button_container);
	button_container.despawn_related::<Children>();
	button_container.with_children(|commands| {
		if failed {
			commands
				.spawn(new_text(FAILED_MSG, font.clone()));
		}
		commands
			.spawn(new_input("Enter game code...", font.clone(), GameCodeInput, Some(&*GAME_CODE_REGEX), None));
		commands
			.spawn(new_input("Enter game password...", font, GamePasswordInput, None, Some(|world: &mut World| {
				let mut game_code = world.query_filtered::<&TextInputBuffer, With<GameCodeInput>>();
				let mut password = world.query_filtered::<&TextInputBuffer, With<GamePasswordInput>>();
				let server = world.resource::<ServerConn>().0.clone();
				let btn_container_entity = world
					.query_filtered::<Entity, With<ButtonContainer>>()
					.single(world)
					.expect(SINGLE_COMPONENT_QUERY_ERROR);
				let game_code = game_code.single(world).expect(SINGLE_COMPONENT_QUERY_ERROR).get_text();
				let password = password.single(world).expect(SINGLE_COMPONENT_QUERY_ERROR).get_text();
				let TokioRuntime {rt, tx} = world.resource::<TokioRuntime>();
				let handle = rt.handle().clone();
				let tx = tx.clone();
				let loading = world.resource::<AssetServer>().load(LOADING);
				let mut commands = world.commands();
				let container = commands.entity(btn_container_entity);
				join_game(server, game_code, handle, tx, container, Some(password), loading);
			})));
		commands
			.spawn(new_primary_button("JOIN", font_bold.clone(), cursor.clone()))
			.observe(|
				trigger: Trigger<Pointer<Click>>,
				server: Res<ServerConn>,
				runtime: Res<TokioRuntime>,
				game_code: Query<&Text, With<GameCodeInput>>,
				password: Query<&Text, With<GamePasswordInput>>,
				mut commands: Commands,
				btn_container_entity: Query<Entity, With<ButtonContainer>>,
				asset_server: Res<AssetServer>
			| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				join_game(
					server.0.clone(),
					game_code.single().expect(SINGLE_COMPONENT_QUERY_ERROR).0.clone(),
					runtime.rt.handle().clone(),
					runtime.tx.clone(),
					commands.entity(btn_container_entity.single().expect(SINGLE_COMPONENT_QUERY_ERROR)),
					Some(password.single().expect(SINGLE_COMPONENT_QUERY_ERROR).0.clone()),
					asset_server.load(LOADING)
				);
			});
		commands
			.spawn(new_group_button("BACK", font_bold.clone(), cursor))
			.observe(display_multiplayer_options_observer);
	});
}

fn display_login_prompt(
	mut commands: Commands,
	button_container: Entity,
	font: Handle<Font>,
	font_bold: Handle<Font>,
	cursor: Handle<Image>,
	error: Option<Error>
) {
	let mut button_container = commands.entity(button_container);
	button_container.despawn_related::<Children>();
	button_container.with_children(|commands| {
		if let Some(err) = error {
			error!("{err}");
			commands
				.spawn(new_text(format!("{err}").as_str(), font.clone()));
		}
		commands
			.spawn(new_input("Username...", font.clone(), UsernameInput, None, None));
		commands
			.spawn(new_input("Password...", font.clone(), PasswordInput, None, Some(|world: &mut World| {
				let asset_server = world.resource::<AssetServer>();
				let font = asset_server.load(GEORGIA);
				let font_bold = asset_server.load(GEORGIA_BOLD);
				let cursor = asset_server.load(BJ_NORMAL_SELECT);
				let loading = asset_server.load(LOADING);
				let btn_container_entity = world
					.query_filtered::<Entity, With<ButtonContainer>>()
					.single(world)
					.expect(SINGLE_COMPONENT_QUERY_ERROR);
				let api = world.resource::<WebsiteConn>().0.clone();
				let mut username = world.query_filtered::<&TextInputBuffer, With<UsernameInput>>();
				let username = username.single(world).expect(SINGLE_COMPONENT_QUERY_ERROR).get_text();
				let mut password = world.query_filtered::<&TextInputBuffer, With<PasswordInput>>();
				let password = password.single(world).expect(SINGLE_COMPONENT_QUERY_ERROR).get_text();
				let TokioRuntime {rt, tx} = world.resource::<TokioRuntime>();
				let rt = &rt.handle().clone();
				let tx = tx.clone();
				let mut commands = world.commands();
				let container = commands.entity(btn_container_entity);
				display_loading_animation(container, loading);
				rt.spawn(async move {
					login_and_connect(LoginRequest {username, password}, &api, tx, font, font_bold, cursor).await;
				});
			})));
		commands
			.spawn(new_primary_button("LOGIN", font_bold.clone(), cursor.clone()))
			.observe(|
				trigger: Trigger<Pointer<Click>>,
				button_container: Query<Entity, With<ButtonContainer>>,
				mut commands: Commands,
				api: Res<WebsiteConn>,
				runtime: Res<TokioRuntime>,
				username: Query<&TextInputBuffer, With<UsernameInput>>,
				password: Query<&TextInputBuffer, With<PasswordInput>>,
				asset_server: Res<AssetServer>
			| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				let username = username.single().expect(SINGLE_COMPONENT_QUERY_ERROR).get_text();
				let password = password.single().expect(SINGLE_COMPONENT_QUERY_ERROR).get_text();
				let api = api.0.clone();
				let tx = runtime.tx.clone();
				let font = asset_server.load(GEORGIA);
				let font_bold = asset_server.load(GEORGIA_BOLD);
				let cursor = asset_server.load(BJ_NORMAL_SELECT);
				display_loading_animation(commands.entity(button_container.single().expect(SINGLE_COMPONENT_QUERY_ERROR)), asset_server.load(LOADING));
				runtime.rt.spawn(async move {
					login_and_connect(LoginRequest {username, password}, &api, tx, font, font_bold, cursor).await;
				});
			});
		commands
			.spawn(new_text("Don't have an account? Create one!", font.clone()));
		commands
			.spawn((
				Button,
				CursorComponent {
					default: CursorStyle::Icon(cursor.clone()),
					pointer: CursorStyle::Animation(BJ_HELP_SELECT_PAGE)
				},
				Node {
					padding: UiRect::axes(Val::Px(16.), Val::Px(20.)),//TODO mess with layout
					position_type: PositionType::Relative,
					justify_content: JustifyContent::Center,
					..default()
				},
				children![(
					Text::new(website!(URL_SIGNUP)),
					Url,
					TextColor(PALLET.url),
					TextFont::from_font(font).with_font_size(18.),
					TextLayout::new_with_justify(JustifyText::Center)
				)]
			))
			.observe(|trigger: Trigger<Pointer<Click>>, url: Query<&Text, With<Url>>| {
				if !matches!(trigger.button, PointerButton::Primary) {return}
				let Ok(url) = url.get(trigger.target) else {return};
				if open::that_detached(url.0.as_str()).is_err() {
					error!("Failed to open {} in browser", URL_SIGNUP);
				}
			});
		commands
			.spawn(new_group_button("BACK", font_bold, cursor))
			.observe(display_main_buttons_observer);
	});
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
	runtime: Res<TokioRuntime>,
	asset_server: Res<AssetServer>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	let font_bold: Handle<Font> = asset_server.load(GEORGIA_BOLD);
	let font: Handle<Font> = asset_server.load(GEORGIA);
	let cursor: Handle<Image> = asset_server.load(BJ_NORMAL_SELECT);
	let loading = asset_server.load(LOADING);
	let website_conn = website_connection.0.clone();
	let container_entity = button_container.single().expect(SINGLE_COMPONENT_QUERY_ERROR);
	let container = commands.entity(container_entity);
	match server_connection {
		Some(_) => {
			info!("Already connected to server");
			display_multiplayer_options(container, font_bold, cursor);
		}
		None => {
			info!("Not connected to server; attempting to connect");
			let Some(cred) = get_credentials() else {
				info!("Prompting for credentials");
				display_login_prompt(commands, container_entity, font, font_bold, cursor, None);
				return;
			};
			info!("Fetched credentials from keyring");
			display_loading_animation(container, loading);
			let tx = runtime.tx.clone();
			runtime.rt.spawn(async move {
				login_and_connect(cred, &website_conn, tx, font, font_bold, cursor).await;
			});
		}
	};
}

/// Creates game code input, join button, and back button
fn display_multiplayer_options_observer(
	trigger: Trigger<Pointer<Click>>,
	mut commands: Commands,
	button_container: Query<Entity, With<ButtonContainer>>,
	asset_server: Res<AssetServer>
) {
	if !matches!(trigger.button, PointerButton::Primary) {return}
	display_multiplayer_options(
		commands.entity(button_container.single().expect(SINGLE_COMPONENT_QUERY_ERROR)),
		asset_server.load(GEORGIA_BOLD),
		asset_server.load(BJ_NORMAL_SELECT)
	);
}

//not observer because password must be manually passed
fn create_game(
	button_container: EntityCommands,
	rt: tokio::runtime::Handle,
	tx: UnboundedSender<CommandQueue>,
	server: Arc<Mutex<ServerConnection>>,
	password: Option<String>,
	loading: Handle<Image>
) {
	display_loading_animation(button_container, loading);
	rt.spawn(async move {
		let create_game_res = server.lock().await.send_create_game(password).await;
		let mut command_queue = CommandQueue::default();
		command_queue.push(move |world: &mut World| {
			match create_game_res {
				Ok(()) => {
					info!("Creating game");
					world.resource_mut::<NextState<ConnectionType>>().set(ConnectionType::Host);
					world.resource_mut::<NextState<UiScene>>().set(UiScene::Setup);
					//The server will send data about the game, which will be used to render the settings and players and stuff
				}
				Err(err) => {
					error!("Error creating game: {}", err);
					let asset_server = world.resource::<AssetServer>();
					let font = asset_server.load(GEORGIA);
					let font_bold = asset_server.load(GEORGIA_BOLD);
					let cursor = asset_server.load(BJ_NORMAL_SELECT);
					let mut container = world.query_filtered::<Entity, With<ButtonContainer>>();
					let container = container.single(world).expect(SINGLE_COMPONENT_QUERY_ERROR);
					let mut commands = world.commands();
					let container = commands.entity(container);
					display_err(
						container,
						font,
						font_bold,
						cursor,
						err
					);
				}
			}
		});
		tx.send(command_queue).expect(TASK_ERROR);
	});
}

fn display_update_game(
	mut btn_container: EntityCommands,
	latest_version: Version,
	font: Handle<Font>,
	font_bold: Handle<Font>,
	cursor: Handle<Image>
) {
	let message = match compare_version(SETBACK_VERSION, latest_version) {
		ClientUpdate::UpToDate => {
			info!("Game is up to date");
			return
		}
		ClientUpdate::UpdateAvailable => "Update Available",
		ClientUpdate::UpdateRequired => "Update Required For Multiplayer"
	};
	info!(message);
	btn_container.despawn_related::<Children>();
	btn_container.with_children(move |commands| {
		commands
			.spawn(new_text(message, font));
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
			//close
			commands
				.spawn(new_group_button("CLOSE", font_bold.clone(), cursor.clone()))
				.observe(display_main_buttons_observer);
			//update
			commands
				.spawn(new_group_button("UPDATE", font_bold, cursor))
				.observe(|trigger: Trigger<Pointer<Click>>| {
					if !matches!(trigger.button, PointerButton::Primary) {return}
					info!("Updating game");
					todo!("update and restart game")
				});
		});
	});
}

fn display_loading_animation(
	mut container: EntityCommands,
	loading: Handle<Image>
) {
	container.despawn_related::<Children>();
	container.with_children(|commands| {
		commands.spawn((
			Node {
				padding: UiRect::axes(Val::Px(16.), Val::Px(20.)),
				position_type: PositionType::Relative,
				justify_content: JustifyContent::Center,
				..default()
			},
			children![
				ImageNode::new(loading)
			]
		));
	});
}

fn get_credentials() -> Option<LoginRequest> {
	let path = Path::new(USR_CRD);
	let username: String = match fs::read_to_string(path) {
		Ok(username) => username,
		Err(err) => {
			if matches!(err.kind(), ErrorKind::NotFound) {
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
			Some(LoginRequest {username, password})
		}
		Err(err) => {
			warn!("Error getting password from keyring: {}", err);
			None
		}
	}
}

fn save_credentials(username: &str, password: &str) {
	let path = Path::new(USR_CRD);
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

fn join_game(
	server: Arc<Mutex<ServerConnection>>,
	game_code: String,
	rt: tokio::runtime::Handle,
	tx: UnboundedSender<CommandQueue>,
	container: EntityCommands,
	password: Option<String>,
	loading: Handle<Image>
) {
	display_loading_animation(container, loading);
	rt.spawn(async move {
		let join_res = server.lock().await.send_join_game(game_code, password).await;
		let mut command_queue = CommandQueue::default();
		
		command_queue.push(move |world: &mut World| {
			match join_res {
				Ok(()) => {
					info!("Joining game");
					world.resource_mut::<NextState<ConnectionType>>().set(ConnectionType::Client);
					world.resource_mut::<NextState<UiScene>>().set(UiScene::Setup);
					//The server will send data about the game, which will be used to render the settings and players and stuff
				}
				Err(err) => {
					error!("Error joining game: {}", err);
					let asset_server = world.resource::<AssetServer>();
					let font = asset_server.load(GEORGIA);
					let font_bold = asset_server.load(GEORGIA_BOLD);
					let cursor = asset_server.load(BJ_NORMAL_SELECT);
					let mut container_entity = world.query_filtered::<Entity, With<ButtonContainer>>();
					let container_entity = container_entity.single(world).expect(SINGLE_COMPONENT_QUERY_ERROR);
					let mut commands = world.commands();
					let container = commands.entity(container_entity);
					display_err(container, font, font_bold, cursor, err);
				}
			}
		});
		tx.send(command_queue).expect(TASK_ERROR);
	});
}

async fn login_and_connect(
	cred: LoginRequest,
	website_conn: &Client,
	tx: UnboundedSender<CommandQueue>,
	font: Handle<Font>,
	font_bold: Handle<Font>,
	cursor: Handle<Image>
) {
	let res = ServerConnection::new(cred.username.clone(), cred.password.clone(), website_conn).await;
	let mut command_queue = CommandQueue::default();
	
	command_queue.push(move |world: &mut World| {
		//remove loading animation
		let btn_container_entity = world
			.query_filtered::<Entity, With<ButtonContainer>>()
			.single(world)
			.expect(SINGLE_COMPONENT_QUERY_ERROR);
		let btn_container = world.query_filtered::<Entity, With<ButtonContainer>>().single(world).expect(SINGLE_COMPONENT_QUERY_ERROR);
		
		match res {
			Ok((server_conn, ssp_rx)) => {
				info!("Successfully connected to server");
				world.insert_resource(ServerConn(Arc::new(Mutex::new(server_conn))));
				world.insert_resource(ServerMessage(ssp_rx));
				display_multiplayer_options(world.commands().entity(btn_container_entity), font_bold, cursor);
				save_credentials(cred.username.as_str(), cred.password.as_str());
			}
			Err(err @ (Error::Unauthorized | Error::Forbidden)) => display_login_prompt(world.commands(), btn_container_entity, font, font_bold, cursor, Some(err)),
			Err(err @ (Error::ServerError | Error::TimedOut | Error::NetworkError)) => display_err(world.commands().entity(btn_container_entity), font, font_bold, cursor, err),
			Err(Error::BadVersion(latest_version)) => display_update_game(world.commands().entity(btn_container), latest_version, font, font_bold, cursor),
			Err(_) => unreachable!("All possible errors should have been covered")
		}
	});
	tx.send(command_queue).expect(TASK_ERROR);
}

fn display_err(
	mut container: EntityCommands,
	font: Handle<Font>,
	font_bold: Handle<Font>,
	cursor: Handle<Image>,
	err: Error
) {
	container.despawn_related::<Children>();
	container.with_children(move |commands| {
		commands
			.spawn(new_text(format!("{err}").as_str(), font));
		commands
			.spawn(new_group_button("BACK", font_bold, cursor))
			.observe(display_main_buttons_observer);
	});
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