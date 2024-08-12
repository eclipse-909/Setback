use {
	crate::{
		SceneType, HostType,
		ui::{
			OverrideTheme,
			scene::{SceneUI, RootUINode},
			button::{UiButtonExt, EventType, SceneChangedEvent, ReadyEvent},
			slider_u8::{SliderU8, UiSliderU8Ext, SliderU8Config}
		}
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
		app.add_plugins(ComponentThemePlugin::<ButtonTheme>::default())
			.add_systems(OnEnter(Self::SCENE), Self::spawn);
	}
}
impl SceneUI for SetupUIPlugin {
	const SCENE: SceneType = SceneType::Setup;
}

impl SetupUIPlugin {
	fn spawn(mut commands: Commands, host: Res<State<HostType>>, asset_server: Res<AssetServer>) {
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
				let mut back_button: UiBuilder<Entity> = ui_root.button(EventType::Observer(SceneChangedEvent(SceneType::MainMenu, Some(HostType::None))));
				back_button
					.insert(ButtonTheme)
					.label(LabelConfig::from("Back")).style().font_size(50.);
				back_button.style()
					.position_type(PositionType::Absolute)
					.top(Val::Px(15.))
					.left(Val::Px(15.));
				
				match host.get() {
					HostType::None => {unreachable!("Must have a HostType to be in the Setup scene");}
					HostType::Singleplayer => {
						let mut start_button: UiBuilder<Entity> = ui_root.button(EventType::Observer(SceneChangedEvent(SceneType::Game, None)));
						start_button
							.insert(ButtonTheme)
							.label(LabelConfig::from("Start")).style().font_size(50.);
						start_button
					}
					HostType::Host | HostType::Client => {
						let mut start_button: UiBuilder<Entity> = ui_root.button(EventType::Observer(ReadyEvent));
						start_button
							.insert(ButtonTheme)
							.label(LabelConfig::from("Ready")).style().font_size(50.);
						start_button
					}
				}.style()
					.position_type(PositionType::Absolute)
					.bottom(Val::Px(15.))
					.right(Val::Px(15.));
				
				ui_root.panel(String::from("Settings"), |settings_container| {
					settings_container
						.checkbox(Some(String::from("Standard Team Mode")), true)
						.insert(Checkbox::overriding_theme());
					settings_container
						.slider_u8(SliderU8Config::horizontal(Some(String::from("Max Score")), 2, 124, 21, true))
						.insert(SliderU8::overriding_theme());
					settings_container
						.checkbox(Some(String::from("Win By 2")), false)
						.insert(Checkbox::overriding_theme());
					settings_container
						.checkbox(Some(String::from("Negative Scores")), true)
						.insert(Checkbox::overriding_theme());
				}).style()
					.position_type(PositionType::Absolute)
					.top(Val::Px(15.))
					.right(Val::Px(15.))
					.background_color(Color::srgba_u8(0, 0, 0, 92))
					.flex_direction(FlexDirection::Column)
					.width(Val::Percent(25.))
					.height(Val::Percent(25.))
					.justify_content(JustifyContent::SpaceEvenly);
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

impl OverrideTheme for Checkbox {
	fn overriding_theme() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::overriding_style)])}
	fn overriding_style(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
		style_builder
			.height(Val::Px(30.));
		style_builder
			.switch_target(Self::CHECKMARK_BACKGROUND)
			.width(Val::Px(30.))
			.height(Val::Px(30.));
		style_builder
			.switch_target(Self::CHECKMARK)
			.width(Val::Px(30.))
			.height(Val::Px(30.))
			.font_size(30.);
		style_builder
			.switch_target(Self::LABEL)
			.font_size(30.);
	}
}

impl OverrideTheme for SliderU8 {
	fn overriding_theme() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::overriding_style)])}
	fn overriding_style(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
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