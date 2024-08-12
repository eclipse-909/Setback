use {
	bevy::prelude::*,
	sickle_ui::prelude::*
};

pub struct SetupPlayerPlugin;
impl Plugin for SetupPlayerPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ComponentThemePlugin::<SetupPlayer>::default());
	}
}

///The GUI element of the player in the Setup scene.
#[derive(Component)]
pub struct SetupPlayer {
	sprite: Entity,
	team_spin_box: Entity,
	human_checkbox: Entity,
	remove_button: Entity
}
impl Default for SetupPlayer {
	fn default() -> Self {
		Self {
			sprite: Entity::PLACEHOLDER,
			team_spin_box: Entity::PLACEHOLDER,
			human_checkbox: Entity::PLACEHOLDER,
			remove_button: Entity::PLACEHOLDER
		}
	}
}
impl UiContext for SetupPlayer {
	fn get(&self, target: &str) -> Result<Entity, String> {
		match target {
			Self::SPRITE => Ok(self.sprite),
			Self::TEAM_SPIN_BOX => Ok(self.team_spin_box),
			Self::HUMAN_CHECKBOX => Ok(self.human_checkbox),
			Self::REMOVE_BUTTON => Ok(self.remove_button),
			_ => Err(format!("{} doesn't exist for SetupPlayer. Possible contexts: {:?}", target, self.contexts()))
		}
	}
	fn contexts(&self) -> Vec<&'static str> {vec![Self::SPRITE, Self::TEAM_SPIN_BOX, Self::HUMAN_CHECKBOX, Self::REMOVE_BUTTON]}
}
impl DefaultTheme for SetupPlayer {
	fn default_theme() -> Option<Theme<Self>> {Theme::new(vec![PseudoTheme::deferred(None, Self::primary_style)]).into()}
}
impl SetupPlayer {
	const SPRITE: &'static str = "Sprite";
	const TEAM_SPIN_BOX: &'static str = "TeamSpinBox";
	const HUMAN_CHECKBOX: &'static str = "HumanCheckbox";
	const REMOVE_BUTTON: &'static str = "RemoveButton";
	
	fn primary_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {
		todo!()
	}
}

pub trait UiSetupPlayerExt {fn setup_player(&mut self, human: bool, team_id: u8, asset_server: &Res<AssetServer>) -> UiBuilder<Entity>;}
impl UiSetupPlayerExt for UiBuilder<'_, Entity> {
	fn setup_player(&mut self, human: bool, team_id: u8, asset_server: &Res<AssetServer>) -> UiBuilder<Entity> {
		self.container(NodeBundle::default(), |data_container| {
			todo!()
		});
		todo!()
	}
}