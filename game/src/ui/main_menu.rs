use {
	crate::{
		SceneType, HostType,
		ui::{
			scene::{SceneUI, RootUINode},
			button::{ButtonAction, UiButtonExt}
		}
	},
	bevy::prelude::*,
	sickle_ui::prelude::*
};

///The UI associated with the Main Menu scene. This will automatically spawn when the [SceneType] state enters [SceneUI::SCENE] and de-spawns when it exits.
#[derive(Component)]
pub struct MainMenuUIPlugin;

impl Plugin for MainMenuUIPlugin {
	fn build(&self, app: &mut App) {
		SceneUI::build(self, app);
		app.add_plugins(ComponentThemePlugin::<ButtonTheme>::default())
			.add_systems(OnEnter(Self::SCENE), Self::spawn);
	}
}
impl SceneUI for MainMenuUIPlugin {
	const SCENE: SceneType = SceneType::MainMenu;
}

impl MainMenuUIPlugin {
	fn spawn(mut commands: Commands) {
		commands.ui_builder(UiRoot).container(
			NodeBundle {
				style: Style {
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					flex_direction: FlexDirection::Column,
					justify_content: JustifyContent::FlexStart,
					..default()
				},
				background_color: BackgroundColor(Color::srgb_u8(0, 92, 24)),
				..default()
			}, |ui_root| {
				ui_root
					.label(LabelConfig::from("Setback"))
					.style()
					.font_size(200.)
					.margin(UiRect::vertical(Val::Px(20.)));
				ui_root
					.button(ButtonAction::SceneChanged(SceneType::Setup, Some(HostType::Singleplayer)))
					.insert(ButtonTheme)
					.label(LabelConfig::from("Singleplayer"))
					.style()
					.font_size(75.);
				ui_root
					.button(ButtonAction::MultiplayerClicked)
					.insert(ButtonTheme)
					.label(LabelConfig::from("Multiplayer"))
					.style()
					.font_size(75.);
				ui_root
					.button(ButtonAction::SceneChanged(SceneType::Settings, Some(HostType::None)))
					.insert(ButtonTheme)
					.label(LabelConfig::from("Settings"))
					.style()
					.font_size(75.);
				ui_root
					.button(ButtonAction::SceneChanged(SceneType::Game, Some(HostType::Singleplayer)))
					.insert(ButtonTheme)
					.label(LabelConfig::from("Tutorial"))
					.style()
					.font_size(75.);
				ui_root
					.button(ButtonAction::AppExit(AppExit::Success))
					.insert(ButtonTheme)
					.label(LabelConfig::from("Exit"))
					.style()
					.font_size(75.);
				ui_root.container((NodeBundle::default(), MultiplayerPopup), |container| {
					container
						.button(ButtonAction::HostClicked)
						.insert(ButtonTheme)
						.label(LabelConfig::from("Host"))
						.style()
						.font_size(75.);
					container
						.button(ButtonAction::JoinClicked)
						.insert(ButtonTheme)
						.label(LabelConfig::from("Host"))
						.style()
						.font_size(75.);
					container
						.button(ButtonAction::MultiplayerPopupClosed)
						.insert(ButtonTheme)
						.label(LabelConfig::from("Host"))
						.style()
						.font_size(75.);
				}).style()
					.position_type(PositionType::Absolute)
					.display(Display::None)
					.justify_content(JustifyContent::SpaceEvenly)
					.align_items(AlignItems::Center);
			}
		).insert(RootUINode)
			.style()
			.align_items(AlignItems::Center);
	}
}

#[derive(Component)]
pub struct MultiplayerPopup;

#[derive(Component, UiContext, Default)]
struct ButtonTheme;

impl DefaultTheme for ButtonTheme {fn default_theme() -> Option<Theme<Self>> {Self::theme().into()}}
impl ButtonTheme {
	fn theme() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::primary_style)])}
	fn primary_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {
		style_builder
			.border(UiRect::all(Val::Px(10.)))
			.border_color(Color::srgb_u8(216, 216, 216))
			.margin(UiRect::all(Val::Px(20.)))
			.padding(UiRect::all(Val::Px(10.)))
			.width(Val::Percent(26.))
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