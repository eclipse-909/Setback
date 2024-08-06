mod main_menu;
mod setup;
mod slider_u8;

use sickle_ui::prelude::{DefaultTheme, StyleBuilder, Theme, ThemeData, UiContext};
use {
	main_menu::MainMenuUIPlugin,
	setup::SetupUIPlugin,
	scene::{SceneChangedEvent, MultiplayerClickedEvent, press_button, on_scene_changed, on_multiplayer_clicked},
	bevy::prelude::*
};
use crate::ui::scene::on_ready_clicked;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((MainMenuUIPlugin, SetupUIPlugin))
			.add_event::<SceneChangedEvent>()
			.add_event::<MultiplayerClickedEvent>()
			.add_systems(Update, press_button)
			.observe(on_scene_changed)
			.observe(on_multiplayer_clicked)
			.observe(on_ready_clicked);
	}
}

mod scene {
	use {
		crate::{SceneType, HostType},
		bevy::prelude::*,
		sickle_ui::prelude::*
	};
	
	///Mark the root node of the UI hierarchy with this to despawn it when changing scenes.
	#[derive(Component)]
	pub struct RootUINode;
	
	///Spawns and de-spawns the implementing struct based on the given [Self::SCENE].
	///Implementers must make their own spawn function like this:
	///```
	///fn spawn(params: impl SystemParam) {/* ... */}
	///```
	///And it must be added to the plugin in the build function like this:
	///```
	///app.add_systems(OnEnter(Self::SCENE), Self::spawn);
	///```
	pub trait SceneUI: Plugin {
		///The scene with which this UI will spawn on enter and de-spawn on exit.
		const SCENE: SceneType;
		///Call this method when implementing Plugin for the scene.
		fn build(&self, app: &mut App) {app.add_systems(OnExit(Self::SCENE), Self::de_spawn);}
		fn de_spawn(mut commands: Commands, root: Query<Entity, With<RootUINode>>) {commands.entity(root.single()).despawn_recursive();}
	}
	
	///The button for the Main Menu which includes the default theme. It stores the event that will be sent to EventWriter when pressed.
	#[derive(Component)]
	pub struct Button(ButtonPressedEvent);
	
	pub trait UiButtonExt {fn button(&mut self, theme: impl DefaultTheme, on_press: ButtonPressedEvent) -> UiBuilder<Entity>;}
	impl UiButtonExt for UiBuilder<'_, Entity> {
		fn button(&mut self, theme: impl Component, on_press: ButtonPressedEvent) -> UiBuilder<Entity> {
			self.spawn((
				ButtonBundle::default(),
				Name::new("Button"),
				Button(on_press),
				theme,
				TrackedInteraction::default()
			))
		}
	}
	
	#[derive(Clone)]
	pub enum ButtonPressedEvent {
		SceneChanged(SceneChangedEvent),
		MultiplayerClicked(MultiplayerClickedEvent),
		AppExit,
		Ready
	}
	
	#[derive(Event, Clone)]
	pub struct SceneChangedEvent(pub SceneType, pub Option<HostType>);
	pub fn on_scene_changed(
		trigger: Trigger<SceneChangedEvent>,
		curr_scene: Res<State<SceneType>>,
		mut next_scene: ResMut<NextState<SceneType>>,
		mut next_host: ResMut<NextState<HostType>>
	) {
		let event = trigger.event();
		if event.0 == *curr_scene.get() {return;}
		if let Some(host) = event.1.clone() {
			next_host.set(host);
		}
		next_scene.set(event.0.clone());
	}
	
	#[derive(Event, Clone)]
	pub struct MultiplayerClickedEvent;
	pub fn on_multiplayer_clicked(trigger: Trigger<MultiplayerClickedEvent>) {
		todo!()
	}
	
	#[derive(Event, Clone)]
	pub struct ReadyEvent;
	pub fn on_ready_clicked(trigger: Trigger<ReadyEvent>) {
		todo!()
	}
	
	///Runs every frame to check if a button was pressed. Triggers the observer based on which event variant it holds.
	pub fn press_button(mut commands: Commands, mut buttons: Query<(&Button, &FluxInteraction), Changed<FluxInteraction>>, mut exit: EventWriter<AppExit>) {
		for (button, interaction) in &mut buttons {
			if *interaction == FluxInteraction::Released {
				match button.0.clone() {
					ButtonPressedEvent::SceneChanged(event) => {commands.trigger(event);}
					ButtonPressedEvent::MultiplayerClicked(event) => {commands.trigger(event);}
					ButtonPressedEvent::AppExit => {exit.send(AppExit::Success);}
					ButtonPressedEvent::Ready => {commands.trigger(ReadyEvent);}
				}
			}
		}
	}
}

trait OverrideTheme: Sized + Component + UiContext + DefaultTheme {
	fn overriding_theme() -> Theme<Self>;
	fn overriding_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData);
}