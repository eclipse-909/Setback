use {
	crate::{
		SceneType, HostType,
		ui::{
			OverrideTheme,
			scene::{SceneUI, RootUINode},
			button::{ButtonAction, UiButtonExt},
			slider_u8::{SliderU8, UiSliderU8Ext, SliderU8Config},
			setup_player::{UiSetupPlayerExt, SetupPlayer},
			spinbox::SpinBox
		}
	},
	bevy::prelude::*,
	sickle_ui::{
		prelude::*,
		ui_commands::UpdateTextExt
	},
	resources::client_items::MAX_PLAYERS,
	std::f32::consts::PI
};

///The UI associated with the Setup scene. This will automatically spawn when the [SceneType] state enters [SceneUI::SCENE] and de-spawns when it exits.
#[derive(Component)]
pub struct SetupUIPlugin;

impl Plugin for SetupUIPlugin {
	fn build(&self, app: &mut App) {
		SceneUI::build(self, app);
		app.add_plugins(ComponentThemePlugin::<ButtonTheme>::default())
			.add_plugins(ComponentThemePlugin::<AddPlayerButtonTheme>::default())
			.add_systems(OnEnter(Self::SCENE), Self::spawn)
			.add_systems(OnExit(Self::SCENE), Self::despawn)
			.add_systems(Update, Self::update_player_pos.run_if(in_state(Self::SCENE)))
			.add_systems(Update, (Self::update_team_mode_checkbox, Self::update_player_teams).chain().after(FluxInteractionUpdate).run_if(in_state(Self::SCENE)));
	}
}
impl SceneUI for SetupUIPlugin {
	const SCENE: SceneType = SceneType::Setup;
}

impl SetupUIPlugin {
	const ELLIPSE_RADIUS: f32 = 525.;
	const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;
	const PLAYER_ANIM_SPEED: f32 = 10.;
	
	fn spawn(mut commands: Commands, host: Res<State<HostType>>, asset_server: Res<AssetServer>) {
		commands.ui_builder(UiRoot).container(
			NodeBundle {
				style: Style {
					width: Val::Percent(100.),
					height: Val::Percent(100.),
					..default()
				},
				background_color: BackgroundColor(Color::srgb_u8(0, 92, 24)),
				..default()
			}, |ui_root| {
				//back button
				let mut back_button: UiBuilder<Entity> = ui_root.button(ButtonAction::SceneChanged(SceneType::MainMenu, Some(HostType::None)));
				back_button
					.insert(ButtonTheme)
					.label(LabelConfig::from("Back")).style().font_size(50.);
				back_button.style()
					.position_type(PositionType::Absolute)
					.top(Val::Px(15.))
					.left(Val::Px(15.));
				//start/ready button
				let host_type: HostType = host.get().to_owned();
				match host_type {
					HostType::None => {unreachable!("Must have a HostType to be in the Setup scene");}
					HostType::Singleplayer => {
						let mut start_button: UiBuilder<Entity> = ui_root.button(ButtonAction::SceneChanged(SceneType::Game, None));
						start_button
							.insert(ButtonTheme)
							.label(LabelConfig::from("Start")).style().font_size(50.);
						start_button
					}
					HostType::Host | HostType::Client => {
						let mut start_button: UiBuilder<Entity> = ui_root.button(ButtonAction::ReadyClicked);
						start_button
							.insert(ButtonTheme)
							.label(LabelConfig::from("Ready")).style().font_size(50.);
						start_button
					}
				}.style()
					.position_type(PositionType::Absolute)
					.bottom(Val::Px(15.))
					.right(Val::Px(15.));
				//settings
				ui_root.panel(String::from("Settings"), |settings_container| {
					settings_container
						.checkbox(Some(String::from("Standard Team Mode")), true)
						.insert((Checkbox::overriding_theme1(), TeamMode));
					settings_container
						.slider_u8(SliderU8Config::horizontal(Some(String::from("Max Score")), 2, 124, 21, true))
						.insert(SliderU8::overriding_theme0())
						.style()
						.margin(UiRect::vertical(Val::Px(15.)));
					settings_container
						.checkbox(Some(String::from("Win By 2")), false)
						.insert(Checkbox::overriding_theme1());
					settings_container
						.checkbox(Some(String::from("Negative Scores")), true)
						.insert(Checkbox::overriding_theme1());
					let mut add_player_button: UiBuilder<Entity> = settings_container.button(ButtonAction::PlayerAdded);
					add_player_button
						.insert((AddPlayerButtonTheme, AddPlayerButton))
						.label(LabelConfig::from("Add Player"))
						.style()
						.font_size(30.);
				}).style()
					.position_type(PositionType::Absolute)
					.top(Val::Px(15.))
					.right(Val::Px(15.))
					.background_color(Color::srgba_u8(0, 0, 0, 92))
					.flex_direction(FlexDirection::Column)
					.width(Val::Percent(25.))
					.height(Val::Percent(29.))
					.justify_content(JustifyContent::FlexStart);
				//players
				ui_root.container((NodeBundle::default(), PlayerContainer), |container| {
					let mut pos: Vec2 = Self::calc_player_pos(0., MAX_PLAYERS as f32);
					let mut setup_player0: UiBuilder<Entity> = container.setup_player(0, true, 0, &asset_server, false, false, false);
					if host_type != HostType::Client {
						setup_player0.insert(LocalPlayer);
					}
					setup_player0
						.style()
						.position_type(PositionType::Absolute)
						.left(Val::Px(-pos.x))
						.bottom(Val::Px(pos.y));
					for i in 1..MAX_PLAYERS {
						pos = Self::calc_player_pos(i as f32, MAX_PLAYERS as f32);
						container.setup_player(i, false, i % 2, &asset_server, true, false, host_type == HostType::Host)
							.style()
							.position_type(PositionType::Absolute)
							.left(Val::Px(-pos.x))
							.bottom(Val::Px(pos.y));
					}
				}).style()
					.left(Val::Percent(32.))
					.bottom(Val::Percent(-12.5));
			}
		).insert(RootUINode)
			.style()
			.align_items(AlignItems::Center);
		//global state
		commands.insert_resource(SetupData {num_players: MAX_PLAYERS, lerp_anim_num_players: MAX_PLAYERS as f32});
		commands.insert_resource(SetupSettings {std_teams: true});
	}
	
	fn despawn(mut commands: Commands) {
		commands.remove_resource::<SetupData>();
		commands.remove_resource::<SetupSettings>();
	}
	
	pub fn calc_player_pos(lerp_anim_index: f32, lerp_anim_num_players: f32) -> Vec2 {
		let theta: f32 = 2. * PI * lerp_anim_index / lerp_anim_num_players + 3. * PI / 2.;
		Vec2 {x: theta.cos(), y: Self::ELLIPSE_HEIGHT_RATIO * theta.sin()} * Self::ELLIPSE_RADIUS
	}
	
	fn update_player_pos(mut setup_data: ResMut<SetupData>, time: Res<Time>, mut players: Query<(&mut SetupPlayer, &mut Style)>) {
		//floats are not directly comparable so I'm saying that they are equal if they are 0.0001 off
		if (setup_data.num_players as f32 - setup_data.lerp_anim_num_players).abs() < 0.0001 {
			setup_data.lerp_anim_num_players = setup_data.num_players as f32;
			return;
		}
		setup_data.lerp_anim_num_players = setup_data.lerp_anim_num_players.lerp(setup_data.num_players as f32, (time.delta_seconds() * Self::PLAYER_ANIM_SPEED).clamp(0.1, 0.9));
		for (mut player, mut style) in players.iter_mut() {
			if (player.lerp_anim_index - player.index as f32).abs() < 0.0001 {
				player.lerp_anim_index = player.index as f32;
			} else {
				player.lerp_anim_index = player.lerp_anim_index.lerp(player.index as f32, (time.delta_seconds() * Self::PLAYER_ANIM_SPEED).clamp(0.1, 0.9));
			}
			let pos: Vec2 = Self::calc_player_pos(player.lerp_anim_index, setup_data.lerp_anim_num_players);
			style.left = Val::Px(-pos.x);
			style.bottom = Val::Px(pos.y);
		}
	}
	
	fn update_team_mode_checkbox(
		checkboxes: Query<&Checkbox, (Changed<Checkbox>, With<TeamMode>)>,
		mut settings: ResMut<SetupSettings>
	) {
		let Ok(checkbox) = checkboxes.get_single() else {return;};
		settings.std_teams = checkbox.checked;
	}
	
	fn update_player_teams(
		settings: Res<SetupSettings>,
		remote_players: Query<&SetupPlayer, Without<LocalPlayer>>,
		local_players: Query<&SetupPlayer, With<LocalPlayer>>,
		mut spin_boxes: Query<(Entity, &mut SpinBox)>,
		host: Res<State<HostType>>,
		mut commands: Commands
	) {
		if settings.std_teams {
			for player in remote_players.iter().chain(local_players.iter()) {
				let (entity, mut spin_box) = spin_boxes.get_mut(player.team_spinbox).expect("Player should have a reference to a spin box");
				commands.entity(entity).remove_pseudo_state(PseudoState::Custom(String::from("ButtonsEnabled")));
				spin_box.value = player.index % 2;
				commands.entity(spin_box.value_label).update_text(format!("{}", spin_box.value));
			}
		} else {
			if *host.get() == HostType::Singleplayer {
				for player in remote_players.iter() {
					let (entity, _) = spin_boxes.get_mut(player.team_spinbox).expect("Player should have a reference to a spin box");
					commands.entity(entity).add_pseudo_state(PseudoState::Custom(String::from("ButtonsEnabled")));
				}
			}
			let local_player: &SetupPlayer = local_players.single();
			let (entity, _) = spin_boxes.get_mut(local_player.team_spinbox).expect("Player should have a reference to a spin box");
			commands.entity(entity).add_pseudo_state(PseudoState::Custom(String::from("ButtonsEnabled")));
		}
	}
}

#[derive(Component)]
struct TeamMode;

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component)]
pub struct PlayerContainer;

#[derive(Component)]
pub struct AddPlayerButton;

#[derive(Resource)]
pub struct SetupData {
	pub num_players: u8,
	pub lerp_anim_num_players: f32
}

#[derive(Resource)]
pub struct SetupSettings {
	pub std_teams: bool
}

#[derive(Component, UiContext, Default)]
struct ButtonTheme;
impl DefaultTheme for ButtonTheme {fn default_theme() -> Option<Theme<Self>> {Self::theme().into()}}
impl ButtonTheme {
	fn theme() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::primary_style)])}
	fn primary_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {
		style_builder
			.border(UiRect::all(Val::Px(5.)))
			.border_color(Color::srgb_u8(216, 216, 216))
			.width(Val::Percent(8.))
			.animated()
			.background_color(AnimatedVals {
				idle: Color::srgb_u8(0, 64, 0),
				hover: Some(Color::srgb_u8(0, 60, 0)),
				press: Some(Color::srgb_u8(0, 55, 0)),
				..default()
			})
			.copy_from(theme_data.interaction_animation);
	}
}

#[derive(Component, UiContext, Default)]
struct AddPlayerButtonTheme;
impl DefaultTheme for AddPlayerButtonTheme {fn default_theme() -> Option<Theme<Self>> {Self::theme().into()}}
impl AddPlayerButtonTheme {
	fn theme() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::primary_style)])}
	fn primary_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {
		style_builder
			.border(UiRect::all(Val::Px(5.)))
			.border_color(Color::srgb_u8(216, 216, 216))
			.height(Val::Px(40.))
			.width(Val::Percent(40.))
			.margin(UiRect::axes(Val::Px(theme_data.spacing.gaps.small), Val::Px(15.)))
			.display(Display::None)
			.animated()
			.background_color(AnimatedVals {
				idle: Color::srgb_u8(0, 64, 0),
				hover: Some(Color::srgb_u8(0, 60, 0)),
				press: Some(Color::srgb_u8(0, 55, 0)),
				..default()
			})
			.copy_from(theme_data.interaction_animation);
	}
}

impl OverrideTheme for SliderU8 {
	fn overriding_theme0() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::overriding_style0)])}
	fn overriding_style0(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
		style_builder
			.height(Val::Px(30.));
		style_builder
			.switch_target(Self::LABEL)
			.font_size(30.);
		style_builder
			.switch_target(Self::BAR)
			.height(Val::Px(7.5))
			.align_items(AlignItems::Center);
		style_builder
			.switch_target(Self::READOUT_CONTAINER)
			.height(Val::Px(30.));
		style_builder
			.switch_target(Self::READOUT)
			.font_size(30.);
	}
}