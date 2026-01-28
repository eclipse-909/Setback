use {
	crate::{
		game::GameUi, main_menu::MainMenuUi, multiplayer::MultiplayerUi, settings::SettingsUi,
		setup::SetupUi, state::Scene,
	},
	gpui::{Context, IntoElement, Render, Window},
};

#[derive(Default)]
pub struct RootUi {
	scene: Scene,
}

impl RootUi {
	pub fn set_scene(&mut self, scene: Scene, cx: &mut Context<Self>) {
		self.scene = scene;
		cx.notify();
	}
}

impl Render for RootUi {
	fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
		match self.scene {
			Scene::MainMenu => MainMenuUi,
			Scene::Multiplayer => MultiplayerUi,
			Scene::Settings => SettingsUi,
			Scene::Setup => SetupUi,
			Scene::Game => GameUi,
		}
	}
}
