use {
	crate::{
		SceneType,
		ui::{
			button::{ButtonAction, UiButtonExt},
			spinbox::UiSpinBoxExt,
			EntityReference, OverrideTheme
		}
	},
	bevy::prelude::*,
	sickle_ui::prelude::*
};

pub struct SetupPlayerPlugin;
impl Plugin for SetupPlayerPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ComponentThemePlugin::<SetupPlayer>::default())
			.add_systems(Update, on_check_human.after(FluxInteractionUpdate).run_if(in_state(SceneType::Setup)));
	}
}

///The GUI element of the player in the Setup scene.
#[derive(Component)]
pub struct SetupPlayer {
	container: Entity,
	remove_button: Entity,
	pub team_spinbox: Entity,
	human_checkbox: Entity,
	pub index: u8,
	pub lerp_anim_index: f32
}
impl Default for SetupPlayer {
	fn default() -> Self {
		Self {
			container: Entity::PLACEHOLDER,
			remove_button: Entity::PLACEHOLDER,
			team_spinbox: Entity::PLACEHOLDER,
			human_checkbox: Entity::PLACEHOLDER,
			index: 0,
			lerp_anim_index: 0.
		}
	}
}
impl UiContext for SetupPlayer {
	fn get(&self, target: &str) -> Result<Entity, String> {
		match target {
			Self::CONTAINER => Ok(self.container),
			Self::REMOVE_BUTTON => Ok(self.remove_button),
			Self::HUMAN_CHECKBOX => Ok(self.human_checkbox),
			_ => Err(format!("{} doesn't exist for SetupPlayer. Possible contexts: {:?}", target, Vec::from_iter(self.contexts())))
		}
	}
	fn contexts(&self) -> impl Iterator<Item = &str> + '_ {[Self::CONTAINER, Self::REMOVE_BUTTON, Self::HUMAN_CHECKBOX].into_iter()}
}
impl DefaultTheme for SetupPlayer {
	fn default_theme() -> Option<Theme<Self>> {
		//No remove button or human checkbox
		let base_theme: PseudoTheme<Self> = PseudoTheme::deferred(None, Self::primary_style);
		//With remove button
		let remove_theme: PseudoTheme<Self> = PseudoTheme::deferred(vec![PseudoState::Custom(String::from("RemoveEnabled"))], Self::remove_style);
		//With human checkbox
		let human_theme: PseudoTheme<Self> = PseudoTheme::deferred(vec![PseudoState::Custom(String::from("HumanEnabled"))], Self::human_style);
		Theme::new(vec![base_theme, remove_theme, human_theme]).into()
	}
}
impl SetupPlayer {
	const CONTAINER: &'static str = "Container";
	const REMOVE_BUTTON: &'static str = "RemoveButton";
	const HUMAN_CHECKBOX: &'static str = "HumanCheckbox";
	
	fn primary_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {
		style_builder
			.flex_direction(FlexDirection::Row)
			.justify_content(JustifyContent::FlexStart)
			.align_items(AlignItems::FlexStart);
		
		style_builder
			.switch_target(Self::CONTAINER)
			.flex_direction(FlexDirection::Column)
			.justify_content(JustifyContent::FlexStart)
			.align_items(AlignItems::Stretch);
		
		style_builder
			.switch_target(Self::REMOVE_BUTTON)
			.display(Display::None)
			.margin(UiRect::bottom(Val::Px(15.)))
			.width(Val::Px(30.))
			.height(Val::Px(30.))
			.border(UiRect::all(Val::Px(2.5)))
			.border_color(Color::srgb_u8(216, 216, 216))
			.align_items(AlignItems::Center)
			.align_self(AlignSelf::FlexEnd)
			.justify_content(JustifyContent::Center)
			.animated()
			.background_color(AnimatedVals {
				idle: Color::srgb_u8(0, 64, 0),
				hover: Some(Color::srgb_u8(0, 60, 0)),
				press: Some(Color::srgb_u8(0, 55, 0)),
				..default()
			})
			.copy_from(theme_data.interaction_animation);
		
		style_builder
			.switch_target(Self::HUMAN_CHECKBOX)
			.display(Display::None);
	}
	
	fn remove_style(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
		style_builder
			.switch_target(Self::REMOVE_BUTTON)
			.display(Display::Flex);
	}
	
	fn human_style(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
		style_builder
			.switch_target(Self::HUMAN_CHECKBOX)
			.display(Display::Flex);
	}
}

#[derive(Component)]
struct HumanAITexture;

pub trait UiSetupPlayerExt {
	fn setup_player(
		&mut self,
		index: u8,
		human: bool,
		team_id: u8,
		asset_server: &Res<AssetServer>,
		remove_enabled: bool,
		team_enabled: bool,
		human_enabled: bool
	) -> UiBuilder<Entity>;
}
impl UiSetupPlayerExt for UiBuilder<'_, Entity> {
	fn setup_player(
		&mut self,
		index: u8,
		human: bool,
		team_id: u8,
		asset_server: &Res<AssetServer>,
		remove_enabled: bool,
		team_enabled: bool,
		human_enabled: bool
	) -> UiBuilder<Entity> {
		const IMAGE_WIDTH: f32 = 128.;
		let mut setup_player: SetupPlayer = SetupPlayer{index, lerp_anim_index: index as f32, ..default()};
		let mut player_container: UiBuilder<Entity> = self.container((
			NodeBundle {style: Style {
				width: Val::Px(IMAGE_WIDTH),
				height: Val::Px(IMAGE_WIDTH),
				..default()
			}, ..default()},
			UiImage::new(asset_server.load(if human {"sprites/personIcon.png"} else {"sprites/botIcon.png"})).with_color(Color::BLACK),
			HumanAITexture,
			
			//TODO do I need this?
			TrackedInteraction::default()
			
			//TODO add a drag and drop interaction
		), |player_container| {
			let container_id: Entity = player_container.id();
			let mut ctrl_container: UiBuilder<Entity> = player_container.container(NodeBundle::default(), |control_container| {
				let mut remove_buttom: UiBuilder<Entity> = control_container.button(ButtonAction::PlayerRemoved(container_id));
				remove_buttom.label(LabelConfig::from("X"))
					.style()
					.display(if remove_enabled {Display::Flex} else {Display::None})
					.font_size(30.);
				setup_player.remove_button = remove_buttom.id();
				let mut team_spinbox: UiBuilder<Entity> = control_container.spin_box("Team ID:", team_id, team_enabled);
				team_spinbox
					.style()
					.margin(UiRect::bottom(Val::Px(15.)));
				setup_player.team_spinbox = team_spinbox.id();
				let mut human_checkbox: UiBuilder<Entity> = control_container.checkbox(Some(String::from("Human")), human);
				human_checkbox.insert((EntityReference(container_id), Checkbox::overriding_theme0()));
				human_checkbox
					.style()
					.display(if human_enabled {Display::Flex} else {Display::None});
				setup_player.human_checkbox = human_checkbox.id();
			});
			ctrl_container
				.style()
				.left(Val::Px(IMAGE_WIDTH));
			setup_player.container = ctrl_container.id();
		});
		player_container.insert(setup_player)
			.style()
			.aspect_ratio(Some(1.))
			.width(Val::Px(IMAGE_WIDTH));
		if remove_enabled {
			player_container.entity_commands().add_pseudo_state(PseudoState::Custom(String::from("RemoveEnabled")));
		}
		if human_enabled {
			player_container.entity_commands().add_pseudo_state(PseudoState::Custom(String::from("HumanEnabled")));
		}
		if team_enabled {
			player_container.entity_commands().add_pseudo_state(PseudoState::Custom(String::from("TeamEnabled")));
		}
		player_container
	}
}

fn on_check_human(checkboxes: Query<(&Checkbox, &EntityReference), Changed<Checkbox>>, mut textures: Query<&mut UiImage, With<HumanAITexture>>, asset_server: Res<AssetServer>) {
	for (checkbox, text_ref) in checkboxes.iter() {
		if let Ok(mut texture) = textures.get_mut(text_ref.0) {
			texture.texture = asset_server.load(if checkbox.checked {"sprites/personIcon.png"} else {"sprites/botIcon.png"});
		}
	}
}

impl OverrideTheme for Checkbox {
	fn overriding_theme0() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::overriding_style0)])}
	fn overriding_theme1() -> Theme<Self> {Theme::new(vec![PseudoTheme::deferred(None, Self::overriding_style1)])}
	fn overriding_style0(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
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
	fn overriding_style1(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
		style_builder
			.margin(UiRect::vertical(Val::Px(15.)))
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