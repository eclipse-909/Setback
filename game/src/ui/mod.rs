mod main_menu;
mod setup;

pub use {
	main_menu::MainMenuUIPlugin,
	setup::SetupUIPlugin
};

mod scene {
	use {
		crate::SceneType,
		bevy::prelude::*,
		sickle_ui::prelude::*
	};
	
	///Mark the root node of the UI hierarchy with this to despawn it when changing scenes.
	#[derive(Component)]
	pub struct RootUINode;
	
	///Spawns and de-spawns the implementing struct based on the given [Self::SCENE].
	pub trait SceneUI: Plugin {
		///The scene with which this UI will spawn on enter and de-spawn on exit.
		const SCENE: SceneType;
		///Call this method when implementing Plugin for the scene.
		fn build(&self, app: &mut App) {
			app.add_systems(OnEnter(Self::SCENE), Self::spawn)
				.add_systems(OnExit(Self::SCENE), Self::de_spawn)
				//TODO fix bug: I can't add system because of generic params
				.add_systems(Update, Self::press_button.run_if(in_state(Self::SCENE)));//this assumes every scene will have a button
		}
		fn spawn(commands: Commands);
		fn de_spawn(mut commands: Commands, root: Query<Entity, With<RootUINode>>) {commands.entity(root.single()).despawn_recursive();}
		fn press_button(mut commands: Commands, mut buttons: Query<(&Button<impl Event + Clone>, &FluxInteraction), Changed<FluxInteraction>>) {
			for (button, interaction) in &mut buttons {
				if *interaction == FluxInteraction::Released {
					commands.trigger(button.0.clone());
				}
			}
		}
	}
	
	///The button for the Main Menu which includes the default theme. It stores the event that will be sent to EventWriter when pressed.
	#[derive(Component)]
	pub struct Button<E: Event>(E);
	
	pub trait UiButtonExt {fn button(&mut self, theme: impl Component, on_press: impl Event) -> UiBuilder<Entity>;}
	
	impl UiButtonExt for UiBuilder<'_, Entity> {
		fn button(&mut self, theme: impl Component, on_press: impl Event) -> UiBuilder<Entity> {
			self.spawn((
				ButtonBundle::default(),
				Name::new("Button"),
				Button(on_press),
				theme,
				TrackedInteraction::default()
			))
		}
	}
}