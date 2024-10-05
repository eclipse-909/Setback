use {
	crate::{
		SceneType,
		HostType,
		ui::{
			scenes::{SceneUI, RootUINode},
			widgets::{UiBuilder, UiBuilderExt}
		}
	},
	bevy::prelude::*
};

///The UI associated with the Main Menu scene. This will automatically spawn when the [SceneType] state enters [SceneUI::SCENE] and de-spawns when it exits.
#[derive(Component)]
pub struct MainMenuUIPlugin;

impl Plugin for MainMenuUIPlugin {
	fn build(&self, app: &mut App) {
		SceneUI::build(self, app);
		app.add_systems(OnEnter(Self::SCENE), Self::spawn);
	}
}
impl SceneUI for MainMenuUIPlugin {
	const SCENE: SceneType = SceneType::MainMenu;
}

impl MainMenuUIPlugin {
	fn spawn(mut commands: Commands) {
		//UI scene root node
		let mut ui_root: UiBuilder<Entity> = commands.ui_builder(RootUINode).spawn(NodeBundle {
			style: Style {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::Center,
				..default()
			},
			background_color: BackgroundColor(Color::srgb_u8(0, 92, 24)),
			..default()
		});
		//Title
		ui_root.spawn(TextBundle::from_section("Setback", TextStyle{
			font_size: 200., ..default()
		}).with_style(Style{
			margin: UiRect::vertical(Val::Px(20.)), ..default()
		}));
		//Singleplayer button
		ui_root.main_menu_button("Singleplayer")
			.observe(|
				_trigger: Trigger<Pointer<Click>>,
				curr_scene: Res<State<SceneType>>,
				mut next_scene: ResMut<NextState<SceneType>>,
				mut next_host: ResMut<NextState<HostType>>
			| {
				if SceneType::Setup == *curr_scene.get() {return;}
				next_host.set(HostType::Singleplayer);
				next_scene.set(SceneType::Setup);
			});
		//Multiplayer button
		ui_root.main_menu_button("Multiplayer")
			.observe(|trigger: Trigger<Pointer<Click>>, mut mul_popup: Query<&mut Style, With<MultiplayerPopup>>| {
				mul_popup.single_mut().display = Display::Flex;
			});
		//Settings button
		ui_root.main_menu_button("Settings")
			.observe(|
				_trigger: Trigger<Pointer<Click>>,
				curr_scene: Res<State<SceneType>>,
				mut next_scene: ResMut<NextState<SceneType>>,
				mut next_host: ResMut<NextState<HostType>>
			| {
				if SceneType::Settings == *curr_scene.get() {return;}
				next_host.set(HostType::None);
				next_scene.set(SceneType::Settings);
			});
		//Tutorial button
		ui_root.main_menu_button("Tutorial")
			.observe(|trigger: Trigger<Pointer<Click>>| {
				todo!("Prompt user if they want to take the tutorial")
			});
		//Exit button
		ui_root.main_menu_button("Exit")
			.observe(|trigger: Trigger<Pointer<Click>>, mut event_writer: EventWriter<AppExit>| {
				//TODO I might want to prompt the user asking if they're sure they want to exit
				event_writer.send(AppExit::Success);
			});
		//Multiplayer popup
		let mut mul_popup: UiBuilder<Entity> = ui_root.spawn((NodeBundle {
			style: Style {
				position_type: PositionType::Absolute,
				display: Display::None,
				justify_content: JustifyContent::SpaceEvenly,
				align_items: AlignItems::Center,
				..default()
			},
			background_color: BackgroundColor(Color::srgba_u8(0x18, 0x18, 0x18, 0x40)),
			..default()
		}, MultiplayerPopup));
		//Host button
		mul_popup.main_menu_button("Host")
			.observe(|trigger: Trigger<Pointer<Click>>| {
				todo!("Connect to server, send a HostGame message, wait for response, change scene to game")
			});
		//Join button
		mul_popup.main_menu_button("Join")
			.observe(|trigger: Trigger<Pointer<Click>>| {
				todo!("Prompt user for join code")
			});
		//Cancel button
		mul_popup.main_menu_button("Cancel")
			.observe(|trigger: Trigger<Pointer<Click>>, mut mul_popup: Query<&mut Style, With<MultiplayerPopup>>| {
				mul_popup.single_mut().display = Display::None;
			});
	}
}

trait MainMenuButtonStyle {
	fn main_menu_button(&mut self, label: Option<impl Into<String>>) -> UiBuilder<Entity>;
}
impl MainMenuButtonStyle for UiBuilder<Entity> {
	fn main_menu_button(&mut self, label: impl Into<Option<String>>) -> UiBuilder<Entity> {
		let mut button: UiBuilder<Entity> = self.spawn(ButtonBundle {
			style: Style {
				border: UiRect::all(Val::Px(10.)),
				margin: UiRect::all(Val::Px(20.)),
				padding: UiRect::all(Val::Px(10.)),
				width: Val::Percent(26.),
				..default()
			},
			border_color: Color::srgb_u8(216, 216, 216).into(),
			background_color: Color::srgb_u8(0, 64, 0).into(),
			..default()
		});
		if let Some(label) = label {
			button.spawn(TextBundle::from_section(label, TextStyle {font_size: 75., ..default()}));
		}
		button
			.observe(|trigger: Trigger<Pointer<Out>>, mut bg_colors: Query<&mut BackgroundColor>| {
				*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(216, 216, 216).into();
			})
			.observe(|trigger: Trigger<Pointer<Over>>, mut bg_colors: Query<&mut BackgroundColor>| {
				*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 60, 0).into();
			})
			// .observe(|trigger: Trigger<Pointer<Move>>, mut bg_colors: Query<&mut BackgroundColor>| {
			// 	*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 60, 0).into();
			// })
			// .observe(|trigger: Trigger<Pointer<Up>>, mut bg_colors: Query<&mut BackgroundColor>| {
			// 	*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 60, 0).into();
			// })
			.observe(|trigger: Trigger<Pointer<Click>>, mut bg_colors: Query<&mut BackgroundColor>| {
				*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 55, 0).into();
			});
		button
	}
}

#[derive(Component)]
pub struct MultiplayerPopup;