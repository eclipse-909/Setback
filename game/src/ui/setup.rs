use {
	crate::{
		SceneType, HostType, SceneChangedEvent,
		ui::scene::{SceneUI, RootUINode, UiButtonExt}
	},
	bevy::prelude::*,
	sickle_ui::prelude::*
};

///The UI associated with the Setup scene. This will automatically spawn when the [SceneType] state enters [SceneUI::SCENE] and de-spawns when it exits.
#[derive(Component)]
pub struct SetupUIPlugin;

impl Plugin for SetupUIPlugin {
	fn build(&self, app: &mut App) {
		SceneUI::build(self, app);
		app.add_plugins(ComponentThemePlugin::<ButtonTheme>::default());
	}
}

impl SceneUI for SetupUIPlugin {
	const SCENE: SceneType = SceneType::Setup;
	
	fn spawn(mut commands: Commands) {
		commands.ui_builder(UiRoot).container(
			NodeBundle {
				style: Style {
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					..default()
				},
				background_color: BackgroundColor(Color::srgb_u8(0, 92, 24)),
				..default()
			}, |ui_root| {
				ui_root
					.button(ButtonTheme, SceneChangedEvent{scene: SceneType::MainMenu, host: Some(HostType::None)})
					.label(LabelConfig::from("Back")).style().font_size(50.);
				ui_root
					.button(ButtonTheme, SceneChangedEvent{scene: SceneType::Game, host: None})
					.label(LabelConfig::from("Start")).style().font_size(50.);//TODO spawn a Ready button instead when in Multiplayer
				
				//TODO make the UI for the game settings
			}
		).insert(RootUINode)
			.style()
			.align_items(AlignItems::Center);
	}
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
			.width(Val::Percent(20.))
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