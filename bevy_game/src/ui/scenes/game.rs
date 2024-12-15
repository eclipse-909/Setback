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

#[derive(Component)]
pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
	fn build(&self, app: &mut App) {
		SceneUI::build(self, app);
		app.add_systems(OnEnter(Self::SCENE), Self::spawn);
	}
}

impl SceneUI for GameUIPlugin {
	const SCENE: SceneType = SceneType::Game;
}

impl GameUIPlugin {
	fn spawn(mut commands: Commands) {
		commands.ui_builder(UiRoot).container(NodeBundle {
			style: Style {
				width: Val::Percent(100.),
				height: Val::Percent(100.),
				..default()
			},
			background_color: BackgroundColor(Color::srgb_u8(0, 92, 24)),
			..default()
		}, |ui_root| {
			//TODO - make UI
			
			//Settings button - opens settings menu
			
			//Score panel
				//Label for each team and their score of the match
				//Indicators for which team has high, low, jack, and/or bevy_game for the current bevy_game
			
			//Bid panel - only show the bids that are higher than the current highest bid
				//pass button
				//bid 2 button
				//bid 3 button
				//bid 4 button
			
			//Settings menu
				//Leave bevy_game button
		});
	}
}