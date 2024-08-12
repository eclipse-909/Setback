use {
	resources::client_items::MAX_PLAYERS,
	std::marker::PhantomData,
	bevy::prelude::*,
	sickle_ui::{
		prelude::*,
		ui_commands::UpdateTextExt
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
struct Button<T: OnClick> {
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
struct Dec;
impl OnClick for Dec {}
struct Inc;
impl OnClick for Inc {}

///Has increment and decrement buttons to set a u8 value.
#[derive(Component)]
pub struct SpinBox {
	dec_button: Entity,
	value: u8,
	value_label: Entity,
	inc_button: Entity,
	label: Entity,
	min_value: u8,
	max_value: u8,
}
impl Default for SpinBox {
	fn default() -> Self {
		Self {
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
			Self::DEC_BUTTON => Ok(self.dec_button),
			Self::VALUE_LABEL => Ok(self.value_label),
			Self::INC_BUTTON => Ok(self.inc_button),
			Self::LABEL => Ok(self.label),
			_ => Err(format!("{} doesn't exist for SpinBox. Possible contexts: {:?}", target, self.contexts()))
		}
	}
	fn contexts(&self) -> Vec<&'static str> {vec![Self::DEC_BUTTON, Self::VALUE_LABEL, Self::INC_BUTTON, Self::LABEL]}
}
impl DefaultTheme for SpinBox {
	fn default_theme() -> Option<Theme<Self>> {Theme::new(vec![PseudoTheme::deferred(None, Self::primary_style)]).into()}
}
impl SpinBox {
	const DEC_BUTTON: &'static str = "Sprite";
	const VALUE_LABEL: &'static str = "TeamSpinBox";
	const INC_BUTTON: &'static str = "HumanCheckbox";
	const LABEL: &'static str = "RemoveButton";
	
	fn primary_style(style_builder: &mut StyleBuilder, theme_data: &ThemeData) {
		todo!()
	}
	
	fn button<T: OnClick>(spin_box: Entity) -> impl Bundle {(
		ButtonBundle::default(),
		Name::new("Button"),
		Button::<T> {spin_box, p: PhantomData::default()},
		TrackedInteraction::default()
	)}
}

pub trait UiSpinBoxExt {fn spin_box(&mut self, label: &str) -> UiBuilder<Entity>;}
impl UiSpinBoxExt for UiBuilder<'_, Entity> {
	fn spin_box(&mut self, label: &str) -> UiBuilder<Entity> {
		let mut spin_box: SpinBox = SpinBox::default();
		let mut spin_box_container: UiBuilder<Entity> = self.container((Name::new("SpinBox"), NodeBundle::default()), |container| {
			let id: Entity = container.id();
			container.container(NodeBundle::default(), |interaction_container| {
				spin_box.dec_button = interaction_container
					.spawn(SpinBox::button::<Dec>(id.clone()))
					.label(LabelConfig::from("<"))
					.id();
				spin_box.value_label = interaction_container
					.label(LabelConfig::from(format!("{}", spin_box.value)))
					.id();
				spin_box.inc_button = interaction_container
					.spawn(SpinBox::button::<Inc>(id))
					.label(LabelConfig::from(">"))
					.id();
			}).style()
				.flex_direction(FlexDirection::Row);
			spin_box.label = container.label(LabelConfig::from(label)).id();
		});
		spin_box_container
			.style()
			.flex_direction(FlexDirection::Column);
		spin_box_container.insert(spin_box);
		spin_box_container
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
		if spin_box.value != spin_box.min_value && spin_box.value != spin_box.max_value {
			if inc {
				spin_box.value += 1;
			} else {
				spin_box.value -= 1;
			}
		}
	}
}

fn update_spin_box(spin_boxes: Query<&SpinBox, Changed<SpinBox>>, mut commands: Commands) {
	for spin_box in &spin_boxes {
		commands
			.entity(spin_box.label)
			.update_text(format!("{}", spin_box.value));
	}
}