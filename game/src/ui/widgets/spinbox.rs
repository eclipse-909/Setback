use {
	resources::client_items::MAX_PLAYERS,
	std::marker::PhantomData,
	bevy::prelude::*,
	sickle_ui::{
		prelude::*,
		ui_commands::UpdateTextExt,
		theme::theme_colors::SchemeColors
	}
};

pub struct SpinBoxPlugin;
impl Plugin for SpinBoxPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ComponentThemePlugin::<SpinBox>::default())
			.add_systems(Update, (on_click, update_spin_box).chain());
	}
}

#[derive(Component)]
pub struct Button<T: OnClick> {
	spin_box: Entity,
	p: PhantomData<T>
}
impl<T: OnClick> Default for Button<T> {
	fn default() -> Self {
		Self {
			spin_box: Entity::PLACEHOLDER,
			p: PhantomData::default()
		}
	}
}

trait OnClick: Sized + Send + Sync + 'static {}
pub struct Dec;
impl OnClick for Dec {}
pub struct Inc;
impl OnClick for Inc {}

///Has increment and decrement buttons to set a u8 value.
#[derive(Component)]
pub struct SpinBox {
	container: Entity,
	pub dec_button: Entity,
	pub value: u8,
	pub value_label: Entity,
	pub inc_button: Entity,
	label: Entity,
	min_value: u8,
	max_value: u8,
}
impl Default for SpinBox {
	fn default() -> Self {
		Self {
			container: Entity::PLACEHOLDER,
			dec_button: Entity::PLACEHOLDER,
			value: 0,
			value_label: Entity::PLACEHOLDER,
			inc_button: Entity::PLACEHOLDER,
			label: Entity::PLACEHOLDER,
			min_value: 0,
			max_value: MAX_PLAYERS - 1//I won't use spin boxes anywhere else, so I might as well make this a default
		}
	}
}
impl UiContext for SpinBox {
	fn get(&self, target: &str) -> Result<Entity, String> {
		match target {
			Self::CONTAINER => Ok(self.container),
			Self::DEC_BUTTON => Ok(self.dec_button),
			Self::VALUE_LABEL => Ok(self.value_label),
			Self::INC_BUTTON => Ok(self.inc_button),
			Self::LABEL => Ok(self.label),
			_ => Err(format!("{} doesn't exist for SpinBox. Possible contexts: {:?}", target, Vec::from_iter(self.contexts())))
		}
	}
	fn contexts(&self) -> impl Iterator<Item = &str> + '_ {[Self::CONTAINER, Self::DEC_BUTTON, Self::VALUE_LABEL, Self::INC_BUTTON, Self::LABEL].into_iter()}
}
impl DefaultTheme for SpinBox {
	fn default_theme() -> Option<Theme<Self>> {
		let base_theme: PseudoTheme<Self> = PseudoTheme::deferred(None, Self::primary_style);
		let enabled_theme: PseudoTheme<Self> = PseudoTheme::deferred(vec![PseudoState::Custom(String::from("ButtonsEnabled"))], Self::buttons_enabled_style);
		Theme::new(vec![base_theme, enabled_theme]).into()
	}
}
impl SpinBox {
	const CONTAINER: &'static str = "Container";
	const DEC_BUTTON: &'static str = "Decrement";
	const VALUE_LABEL: &'static str = "Value";
	const INC_BUTTON: &'static str = "Increment";
	const LABEL: &'static str = "Label";
	
	fn primary_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {
		let colors: SchemeColors = theme_data.colors();
		
		style_builder
			.flex_direction(FlexDirection::Row)
			.justify_content(JustifyContent::FlexStart)
			.align_items(AlignItems::FlexStart);
		
		style_builder
			.switch_target(Self::CONTAINER)
			.flex_direction(FlexDirection::Row)
			.justify_content(JustifyContent::Center)
			.align_items(AlignItems::FlexStart);
		
		style_builder
			.switch_target(Self::LABEL)
			.margin(UiRect::right(Val::Px(5.)))
			.font_size(30.)
			.font_color(colors.on(OnColor::Surface));
		
		style_builder
			.switch_target(Self::DEC_BUTTON)
			.display(Display::None)
			.margin(UiRect::horizontal(Val::Px(5.)))
			.width(Val::Px(30.))
			.height(Val::Px(30.))
			.border(UiRect::all(Val::Px(2.5)))
			.border_color(Color::srgb_u8(216, 216, 216))
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
			.switch_target(Self::VALUE_LABEL)
			.margin(UiRect::horizontal(Val::Px(5.)))
			.font_size(30.)
			.font_color(colors.on(OnColor::Surface));
		
		style_builder
			.switch_target(Self::INC_BUTTON)
			.display(Display::None)
			.margin(UiRect::horizontal(Val::Px(5.)))
			.width(Val::Px(30.))
			.height(Val::Px(30.))
			.border(UiRect::all(Val::Px(2.5)))
			.border_color(Color::srgb_u8(216, 216, 216))
			.justify_content(JustifyContent::Center)
			.animated()
			.background_color(AnimatedVals {
				idle: Color::srgb_u8(0, 64, 0),
				hover: Some(Color::srgb_u8(0, 60, 0)),
				press: Some(Color::srgb_u8(0, 55, 0)),
				..default()
			})
			.copy_from(theme_data.interaction_animation);
	}
	
	fn buttons_enabled_style(style_builder: &mut StyleBuilder, _theme_data: &ThemeData) {
		style_builder
			.switch_target(Self::DEC_BUTTON)
			.display(Display::Flex);
		
		style_builder
			.switch_target(Self::INC_BUTTON)
			.display(Display::Flex);
	}
	
	fn button<T: OnClick>(spin_box: Entity) -> impl Bundle {(
		ButtonBundle::default(),
		Name::new("Button"),
		Button::<T> {spin_box, p: PhantomData::default()},
		TrackedInteraction::default()
	)}
}

pub trait UiSpinBoxExt {fn spin_box(&mut self, label: &str, value: u8, enabled: bool) -> UiBuilder<Entity>;}
impl UiSpinBoxExt for UiBuilder<'_, Entity> {
	// fn spin_box(&mut self, label: &str) -> UiBuilder<Entity> {self.spin_box_with_value(label, 0)}
	fn spin_box(&mut self, label: &str, value: u8, enabled: bool) -> UiBuilder<Entity> {
		let mut spin_box: SpinBox = SpinBox {value, ..default()};
		self.container((Name::new("SpinBox"), NodeBundle::default()), |container| {
			let id: Entity = container.id();
			spin_box.label = container.label(LabelConfig::from(label)).id();
			spin_box.container = container.container(NodeBundle::default(), |interaction_container| {
				let mut dec_button: UiBuilder<Entity> = interaction_container.spawn(SpinBox::button::<Dec>(id));
				spin_box.dec_button = dec_button.id();
				dec_button.label(LabelConfig::from("<"))
					.style()
					.font_size(30.);
				spin_box.value_label = interaction_container
					.label(LabelConfig::from(format!("{}", value)))
					.id();
				let mut inc_button: UiBuilder<Entity> = interaction_container.spawn(SpinBox::button::<Inc>(id));
				spin_box.inc_button = inc_button.id();
				inc_button.label(LabelConfig::from(">"))
					.style()
					.font_size(30.);
			}).id();
			container.insert(spin_box);//insert here so the buttons can access the spin box component when they're pressed
			if enabled {
				container.entity_commands().add_pseudo_state(PseudoState::Custom(String::from("ButtonsEnabled")));
			}
		})
	}
}

fn on_click(
	buttons: Query<(AnyOf<(&Button<Inc>, &Button<Dec>)>, &FluxInteraction), Changed<FluxInteraction>>,
	mut spin_boxes: Query<&mut SpinBox>
) {
	for ((inc_button, dec_button), interaction) in &buttons {
		if *interaction != FluxInteraction::Released {continue;}
		let (spin_box_id, inc) = if let Some(inc_button) = inc_button {
			(inc_button.spin_box, true)
		} else if let Some(dec_button) = dec_button {
			(dec_button.spin_box, false)
		} else {
			continue;
		};
		let Ok(mut spin_box) = spin_boxes.get_mut(spin_box_id) else {continue;};
		if inc {
			if spin_box.value < spin_box.max_value {
				spin_box.value += 1;
			}
		} else if spin_box.value > spin_box.min_value {
			spin_box.value -= 1;
		}
	}
}

fn update_spin_box(spin_boxes: Query<&SpinBox, Changed<SpinBox>>, mut commands: Commands) {
	for spin_box in &spin_boxes {
		commands
			.entity(spin_box.value_label)
			.update_text(format!("{}", spin_box.value));
	}
}