use {
	crate::{SceneType, HostType},
	bevy::prelude::*,
	sickle_ui::prelude::*
};

pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
	fn build(&self, app: &mut App) {
		app
			//Register the events
			.add_event::<SceneChangedEvent>()
			.add_event::<MultiplayerClickedEvent>()
			.add_event::<ReadyEvent>()
			//observers for when the events are triggered or sent
			.observe(on_scene_changed)
			.observe(on_multiplayer_clicked)
			.observe(on_ready_clicked)
			//check if the button was clicked to trigger or send the event
			.add_systems(Update, button_clicked_o::<SceneChangedEvent>)
			.add_systems(Update, button_clicked_o::<MultiplayerClickedEvent>)
			.add_systems(Update, button_clicked_o::<ReadyEvent>)
			.add_systems(Update, button_clicked_w::<AppExit>);
	}
}

///The button for the Main Menu which includes the default theme. It stores the event that will be sent to EventWriter when pressed.
#[derive(Component)]
pub struct Button<E: Event + Clone>(EventType<E>);

pub trait UiButtonExt {fn button<E: Event + Clone>(&mut self, on_press: EventType<E>) -> UiBuilder<Entity>;}
impl UiButtonExt for UiBuilder<'_, Entity> {
	fn button<E: Event + Clone>(&mut self, on_press: EventType<E>) -> UiBuilder<Entity> {
		self.spawn((
			ButtonBundle::default(),
			Name::new("Button"),
			Button::<E>(on_press),
			TrackedInteraction::default()
		))
	}
}

///Event that triggers an Observer.
fn button_clicked_o<E: Event + Clone>(mut commands: Commands, buttons: Query<(&Button<E>, &FluxInteraction), Changed<FluxInteraction>>) {
	for (button, interaction) in buttons.iter() {
		if *interaction == FluxInteraction::Released {
			if let EventType::Observer(event) = button.0.clone() {
				commands.trigger(event);
			}
		}
	}
}

///Event that writes to an EventWriter.
fn button_clicked_w<E: Event + Clone>(mut event_writer: EventWriter<E>, buttons: Query<(&Button<E>, &FluxInteraction), Changed<FluxInteraction>>) {
	for (button, interaction) in buttons.iter() {
		if *interaction == FluxInteraction::Released {
			if let EventType::Writer(event) = button.0.clone() {
				event_writer.send(event);
			}
		}
	}
}

#[derive(Clone)]
pub enum EventType<E: Event + Clone> {
	Observer(E),
	Writer(E)
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