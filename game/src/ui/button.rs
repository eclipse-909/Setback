use {
	crate::{
		SceneType,
		HostType,
		ui::{
			main_menu::MultiplayerPopup,
			setup::{SetupUIPlugin, SetupData, AddPlayerButton, PlayerContainer, SetupSettings, LocalPlayer},
			setup_player::{SetupPlayer, UiSetupPlayerExt},
			spinbox::SpinBox
		}
	},
	bevy::prelude::*,
	sickle_ui::{
		prelude::*,
		ui_commands::UpdateTextExt
	},
	resources::client_items::{MAX_PLAYERS, MIN_PLAYERS}
};

pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, on_button_clicked.after(FluxInteractionUpdate));
	}
}

#[derive(Component)]
pub struct Button(ButtonAction);

pub trait UiButtonExt {fn button(&mut self, action: ButtonAction) -> UiBuilder<Entity>;}
impl UiButtonExt for UiBuilder<'_, Entity> {
	fn button(&mut self, action: ButtonAction) -> UiBuilder<Entity> {
		self.spawn((
			ButtonBundle::default(),
			Name::new("Button"),
			Button(action),
			TrackedInteraction::default()
		))
	}
}

pub enum ButtonAction {
	SceneChanged(SceneType, Option<HostType>),
	AppExit(AppExit),
	MultiplayerClicked,
	MultiplayerPopupClosed,
	HostClicked,
	JoinClicked,
	ReadyClicked,
	PlayerAdded,
	///Entity references the setup player that will be removed
	PlayerRemoved(Entity)
}

///The one button-handling system to rule them all
fn on_button_clicked(
	buttons: Query<(&Button, &FluxInteraction), Changed<FluxInteraction>>,
	//SceneChanged
	curr_scene: Res<State<SceneType>>,
	mut next_scene: ResMut<NextState<SceneType>>,
	mut next_host: ResMut<NextState<HostType>>,
	//AppExit
	mut event_writer: EventWriter<AppExit>,
	//MultiplayerClicked
	//these have to be combined because you can't have two queries for the same component with mutable access
	mut styles: Query<(&mut Style, Option<&MultiplayerPopup>, Option<&AddPlayerButton>)>,
	//PlayerAdded
	mut commands: Commands,
	containers: Query<Entity, With<PlayerContainer>>,
	mut setup_data: Option<ResMut<SetupData>>,
	asset_server: Res<AssetServer>,
	host: Res<State<HostType>>,
	mut other_players: Query<(Entity, &mut SetupPlayer), Without<LocalPlayer>>,
	//PlayerRemoved
	mut local_players: Query<(Entity, &mut SetupPlayer), With<LocalPlayer>>,
	settings: Option<Res<SetupSettings>>,
	mut spin_boxes: Query<(Entity, &mut SpinBox)>
) {
	'button_loop: for (button, interaction) in buttons.iter() {
		if *interaction != FluxInteraction::Released {continue;}
		match &button.0 {
			ButtonAction::SceneChanged(scene, host) => {
				if *scene == *curr_scene.get() {return;}
				if let Some(host) = host {
					next_host.set(host.clone());
				}
				next_scene.set(scene.clone());
			}
			ButtonAction::AppExit(exit_code) => {
				event_writer.send(exit_code.clone());
			}
			ButtonAction::MultiplayerClicked => {
				for (mut style, multiplayer_popup, _) in styles.iter_mut() {
					if let Some(_) = multiplayer_popup {
						style.display = Display::Flex;
						continue 'button_loop;
					}
				}
				unreachable!("Expected multiplayer popup with style component");
			}
			ButtonAction::MultiplayerPopupClosed => {
				for (mut style, multiplayer_popup, _) in styles.iter_mut() {
					if let Some(_) = multiplayer_popup {
						style.display = Display::None;
						continue 'button_loop;
					}
				}
				unreachable!("Expected multiplayer popup with style component");
			}
			ButtonAction::HostClicked => {
				todo!("Connect to server and host a game")
			}
			ButtonAction::JoinClicked => {
				todo!("Prompt user for join code and connect to server")
			}
			ButtonAction::ReadyClicked => {
				todo!("Start ready sequence for multiplayer")
			}
			ButtonAction::PlayerAdded => {
				let setup_data: &mut ResMut<SetupData> = setup_data.as_mut().expect("Setup Data should exist");
				let pos: Vec2 = SetupUIPlugin::calc_player_pos(setup_data.num_players as f32, setup_data.lerp_anim_num_players);
				//create setup player
				commands
					.ui_builder(containers.single())
					.setup_player(setup_data.num_players, false, setup_data.num_players % 2, &asset_server, true, false, *host.get() == HostType::Host)
					.style()
					.position_type(PositionType::Absolute)
					.left(Val::Px(-pos.x))
					.bottom(Val::Px(pos.y));
				setup_data.num_players += 1;
				//don't allow adding players if at maximum allowed players
				if setup_data.num_players == MAX_PLAYERS {
					let mut found: bool = false;
					for (mut style, _, add_player_button) in styles.iter_mut() {
						if let Some(_) = add_player_button {
							style.display = Display::None;
							found = true;
							break;
						}
					}
					if !found {
						unreachable!("Expected add player button with style component");
					}
				}
				//enable the remove buttons for every player but yourself
				for (entity, _) in other_players.iter() {
					commands.entity(entity).add_pseudo_state(PseudoState::Custom(String::from("RemoveEnabled")));
				}
			}
			ButtonAction::PlayerRemoved(setup_player) => {
				
				//BUG sometimes it gives a warning: Failed to set text on entity ###: No Text component found!
				
				let setup_data: &mut ResMut<SetupData> = setup_data.as_mut().expect("Setup Data should exist");
				let settings: &Res<SetupSettings> = settings.as_ref().expect("Setup Settings should exist");
				let index: u8 = other_players.get(*setup_player).expect("Player should still exist").1.index;
				setup_data.num_players -= 1;
				for (player_entity, mut player) in other_players.iter_mut().chain(local_players.iter_mut()) {
					if player.index == index {continue;}
					//shift players down
					if player.index > index {
						player.index -= 1;
					}
					//set standard team numbers if appropriate
					if settings.std_teams {
						let (spin_box_entity, mut spin_box) = spin_boxes.get_mut(player.team_spinbox).expect("Player should have a reference to a spin box");
						commands.entity(spin_box_entity).remove_pseudo_state(PseudoState::Custom(String::from("ButtonsEnabled")));
						spin_box.value = player.index % 2;
						commands.entity(spin_box.value_label).update_text(format!("{}", spin_box.value));
					}
					//hide remove button if we are now at the minimum number of allowed players
					if setup_data.num_players == MIN_PLAYERS {
						commands.entity(player_entity).remove_pseudo_state(PseudoState::Custom(String::from("RemoveEnabled")));
					}
				}
				commands.entity(*setup_player).despawn_recursive();
				for (mut style, _, add_player_button) in styles.iter_mut() {
					if let Some(_) = add_player_button {
						style.display = Display::Flex;
						continue 'button_loop;
					}
				}
				unreachable!("Expected multiplayer popup with style component");
			}
		}
	}
}