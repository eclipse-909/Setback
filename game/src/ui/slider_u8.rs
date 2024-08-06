use {
	bevy::{
		prelude::*,
		input::mouse::MouseScrollUnit,
		ui::RelativeCursorPosition
	},
	sickle_ui::{
		prelude::*,
		widgets::inputs::slider::{SliderBar, SliderDragHandle, SliderAxis},
		ui_commands::UpdateTextExt,
		lerp::Lerp
	}
};

pub struct SliderU8Plugin;

impl Plugin for SliderU8Plugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ComponentThemePlugin::<SliderU8>::default())
			.add_systems(
				Update,
				(
					update_slider_on_scroll.after(ScrollableUpdate),
					update_slider_on_drag.after(DraggableUpdate),
					update_slider_handle,
					update_slider_readout,
				)
					.chain(),
			);
	}
}

// TODO: Add input for value (w/ read/write flags)
// TODO: Support click-on-bar value setting
fn update_slider_on_scroll(
	q_scrollables: Query<
		(AnyOf<(&SliderBar, &SliderDragHandle)>, &Scrollable),
		Changed<Scrollable>,
	>,
	mut q_slider: Query<&mut SliderU8>,
) {
	for ((slider_bar, handle), scrollable) in &q_scrollables {
		let Some((axis, diff, unit)) = scrollable.last_change() else {
			continue;
		};
		if axis == ScrollAxis::Horizontal {
			continue;
		}
		
		let slider_id = if let Some(slider_bar) = slider_bar {
			slider_bar.slider
		} else if let Some(handle) = handle {
			handle.slider
		} else {
			continue;
		};
		
		let Ok(mut slider) = q_slider.get_mut(slider_id) else {
			continue;
		};
		
		let offset = match unit {
			MouseScrollUnit::Line => -diff * 0.823,//idk why but approx. 0.823 makes it tick by 1 when scrolling
			MouseScrollUnit::Pixel => -diff,
		};
		
		let fraction = offset / 100.;
		slider.ratio = (slider.ratio + fraction as f64).clamp(0., 1.);
	}
}

fn update_slider_on_drag(
	q_draggable: Query<(&Draggable, &SliderDragHandle, &Node), Changed<Draggable>>,
	q_node: Query<&Node>,
	mut q_slider: Query<&mut SliderU8>,
) {
	for (draggable, handle, node) in &q_draggable {
		let Ok(mut slider) = q_slider.get_mut(handle.slider) else {
			continue;
		};
		
		if draggable.state == DragState::Inactive || draggable.state == DragState::MaybeDragged {
			continue;
		}
		
		if draggable.state == DragState::DragCanceled {
			if let Some(base_ratio) = slider.base_ratio {
				slider.ratio = base_ratio;
				continue;
			}
		}
		
		if draggable.state == DragState::DragStart {
			slider.base_ratio = slider.ratio.into();
		}
		
		let Ok(slider_bar) = q_node.get(slider.bar_container) else {
			continue;
		};
		let Some(diff) = draggable.diff else {
			continue;
		};
		
		let axis = &slider.config.axis;
		let fraction = match axis {
			SliderAxis::Horizontal => {
				let width = slider_bar.size().x - node.size().x;
				if diff.x == 0. || width == 0. {
					continue;
				}
				diff.x / width
			}
			SliderAxis::Vertical => {
				let height = slider_bar.size().y - node.size().y;
				if diff.y == 0. || height == 0. {
					continue;
				}
				-diff.y / height
			}
		};
		
		slider.ratio = (slider.ratio + fraction as f64).clamp(0., 1.);
	}
}

fn update_slider_handle(
	q_slider: Query<&SliderU8, Or<(Changed<SliderU8>, Changed<Node>)>>,
	q_node: Query<&Node>,
	mut q_hadle_style: Query<(&Node, &mut Style), With<SliderDragHandle>>,
) {
	for slider in &q_slider {
		let Ok(slider_bar) = q_node.get(slider.bar_container) else {
			continue;
		};
		let Ok((node, mut style)) = q_hadle_style.get_mut(slider.handle) else {
			continue;
		};
		
		let axis = &slider.config.axis;
		match axis {
			SliderAxis::Horizontal => {
				let width = slider_bar.size().x - node.size().x;
				let handle_position = width * slider.ratio as f32;
				if style.left != Val::Px(handle_position) {
					style.left = Val::Px(handle_position);
				}
			}
			SliderAxis::Vertical => {
				let height = slider_bar.size().y - node.size().y;
				let handle_position = height * (1. - slider.ratio) as f32;
				if style.top != Val::Px(handle_position) {
					style.top = Val::Px(handle_position);
				}
			}
		}
	}
}

fn update_slider_readout(q_slider: Query<&SliderU8, Changed<SliderU8>>, mut commands: Commands) {
	for slider in &q_slider {
		if !slider.config.show_current {
			continue;
		}
		
		commands
			.entity(slider.readout)
			.update_text(format!("{:.1}", slider.value()));
	}
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct SliderU8Config {
	pub label: Option<String>,
	pub min: u8,
	pub max: u8,
	pub initial_value: u8,
	pub show_current: bool,
	pub axis: SliderAxis,
}

impl SliderU8Config {
	pub fn new(
		label: impl Into<Option<String>>,
		min: u8,
		max: u8,
		initial_value: u8,
		show_current: bool,
		axis: SliderAxis,
	) -> Self {
		if max <= min || initial_value < min || initial_value > max {
			panic!(
				"Invalid slider config values! Min: {}, Max: {}, Initial: {}",
				min, max, initial_value
			);
		}
		
		SliderU8Config {
			label: label.into(),
			min,
			max,
			initial_value,
			show_current,
			axis,
		}
	}
	
	pub fn horizontal(
		label: impl Into<Option<String>>,
		min: u8,
		max: u8,
		initial_value: u8,
		show_current: bool,
	) -> Self {
		Self::new(
			label.into(),
			min,
			max,
			initial_value,
			show_current,
			SliderAxis::Horizontal,
		)
	}
	
	pub fn vertical(
		label: impl Into<Option<String>>,
		min: u8,
		max: u8,
		initial_value: u8,
		show_current: bool,
	) -> Self {
		Self::new(
			label.into(),
			min,
			max,
			initial_value,
			show_current,
			SliderAxis::Vertical,
		)
	}
	
	pub fn with_value(self, value: u8) -> Self {
		if value >= self.min && value <= self.max {
			return Self {
				initial_value: value,
				..self
			};
		}
		
		panic!("Value must be between min and max!");
	}
}

impl Default for SliderU8Config {
	fn default() -> Self {
		Self {
			label: None,
			min: 0,
			max: 255,
			initial_value: 128,
			show_current: Default::default(),
			axis: Default::default(),
		}
	}
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct SliderU8 {
	ratio: f64,
	config: SliderU8Config,
	label: Entity,
	bar_container: Entity,
	bar: Entity,
	handle: Entity,
	readout_container: Entity,
	readout: Entity,
	base_ratio: Option<f64>,
}

impl Default for SliderU8 {
	fn default() -> Self {
		Self {
			ratio: Default::default(),
			config: Default::default(),
			base_ratio: None,
			label: Entity::PLACEHOLDER,
			bar_container: Entity::PLACEHOLDER,
			bar: Entity::PLACEHOLDER,
			handle: Entity::PLACEHOLDER,
			readout_container: Entity::PLACEHOLDER,
			readout: Entity::PLACEHOLDER,
		}
	}
}

impl UiContext for SliderU8 {
	fn get(&self, target: &str) -> Result<Entity, String> {
		match target {
			Self::LABEL => Ok(self.label),
			Self::BAR_CONTAINER => Ok(self.bar_container),
			Self::BAR => Ok(self.bar),
			Self::HANDLE => Ok(self.handle),
			Self::READOUT_CONTAINER => Ok(self.readout_container),
			Self::READOUT => Ok(self.readout),
			_ => Err(format!(
				"{} doesn't exists for Slider. Possible contexts: {:?}",
				target,
				self.contexts()
			)),
		}
	}
	
	fn contexts(&self) -> Vec<&'static str> {
		vec![
			Self::LABEL,
			Self::BAR_CONTAINER,
			Self::BAR,
			Self::HANDLE,
			Self::READOUT_CONTAINER,
			Self::READOUT,
		]
	}
}

impl DefaultTheme for SliderU8 {
	fn default_theme() -> Option<Theme<Self>> {
		Self::theme().into()
	}
}

impl SliderU8 {
	pub const LABEL: &'static str = "Label";
	pub const BAR_CONTAINER: &'static str = "BarContainer";
	pub const BAR: &'static str = "Bar";
	pub const HANDLE: &'static str = "Handle";
	pub const READOUT_CONTAINER: &'static str = "ReadoutContainer";
	pub const READOUT: &'static str = "Readout";
	
	pub fn value(&self) -> u8 {
		self.config.min.lerp(self.config.max, self.ratio)
	}
	
	pub fn config(&self) -> &SliderU8Config {
		&self.config
	}
	
	pub fn set_value(&mut self, value: u8) {
		if value > self.config.max || value < self.config.min {
			warn!("Tried to set slider value outside of range");
			return;
		}
		
		self.ratio = (value - self.config.min) as f64 / (self.config.max - self.config.min) as f64
	}
	
	pub fn theme() -> Theme<Self> {
		let base_theme = PseudoTheme::deferred_context(None, Self::primary_style);
		Theme::new(vec![base_theme])
	}
	
	fn primary_style(style_builder: &mut StyleBuilder, slider: &Self, theme_data: &ThemeData) {
		let theme_spacing = theme_data.spacing;
		let colors = theme_data.colors();
		let font = theme_data
			.text
			.get(FontStyle::Body, FontScale::Medium, FontType::Regular);
		
		match slider.config().axis {
			SliderAxis::Horizontal => {
				style_builder
					.justify_content(JustifyContent::SpaceBetween)
					.align_items(AlignItems::Center)
					.width(Val::Percent(100.))
					.height(Val::Px(theme_spacing.areas.small))
					.padding(UiRect::horizontal(Val::Px(theme_spacing.gaps.medium)));
				
				style_builder
					.switch_target(Self::LABEL)
					.margin(UiRect::right(Val::Px(theme_spacing.gaps.medium)));
				
				style_builder
					.switch_target(Self::BAR_CONTAINER)
					.width(Val::Percent(100.));
				
				style_builder
					.switch_target(Self::BAR)
					.width(Val::Percent(100.))
					.height(Val::Px(theme_spacing.gaps.small))
					.margin(UiRect::vertical(Val::Px(theme_spacing.gaps.medium)));
				
				style_builder
					.switch_target(Self::READOUT)
					.min_width(Val::Px(theme_spacing.areas.medium))
					.margin(UiRect::left(Val::Px(theme_spacing.gaps.medium)));
				
				style_builder
					.switch_context(Self::HANDLE, None)
					.height(Val::Px(30.))
					.width(Val::Px(20.));
			}
			SliderAxis::Vertical => {
				style_builder
					.flex_direction(FlexDirection::ColumnReverse)
					.justify_content(JustifyContent::SpaceBetween)
					.align_items(AlignItems::Center)
					.height(Val::Percent(100.))
					.padding(UiRect::vertical(Val::Px(theme_spacing.gaps.medium)));
				
				style_builder
					.switch_target(Self::LABEL)
					.margin(UiRect::px(
						theme_spacing.gaps.small,
						theme_spacing.gaps.small,
						theme_spacing.gaps.small,
						0.,
					));
				
				style_builder
					.switch_target(Self::BAR_CONTAINER)
					.flex_direction(FlexDirection::Column)
					.height(Val::Percent(100.));
				
				style_builder
					.switch_target(Self::BAR)
					.flex_direction(FlexDirection::Column)
					.width(Val::Px(theme_spacing.gaps.small))
					.height(Val::Percent(100.))
					.margin(UiRect::horizontal(Val::Px(theme_spacing.gaps.medium)));
				
				style_builder
					.switch_target(Self::READOUT_CONTAINER)
					.justify_content(JustifyContent::Center)
					.justify_items(JustifyItems::Center)
					.width(Val::Px(theme_spacing.areas.medium))
					.overflow(Overflow::clip());
				
				style_builder
					.switch_target(Self::READOUT)
					.margin(UiRect::all(Val::Px(theme_spacing.gaps.small)));
				
				style_builder
					.switch_context(Self::HANDLE, None)
					.height(Val::Px(30.))
					.width(Val::Px(20.));
			}
		}
		
		style_builder.reset_context();
		
		style_builder
			.switch_target(Self::LABEL)
			.sized_font(font.clone())
			.font_color(colors.on(On::Surface));
		
		if slider.config().label.is_none() {
			style_builder
				.switch_target(Self::LABEL)
				.display(Display::None)
				.visibility(Visibility::Hidden);
		} else {
			style_builder
				.switch_target(Self::LABEL)
				.display(Display::Flex)
				.visibility(Visibility::Inherited);
		}
		
		if !slider.config().show_current {
			style_builder
				.switch_target(Self::READOUT_CONTAINER)
				.display(Display::None)
				.visibility(Visibility::Hidden);
		} else {
			style_builder
				.switch_target(Self::READOUT_CONTAINER)
				.display(Display::Flex)
				.visibility(Visibility::Inherited);
		}
		
		style_builder
			.switch_target(Self::READOUT)
			.sized_font(font.clone())
			.font_color(colors.on(On::Surface));
		
		style_builder
			.switch_target(Self::BAR)
			.border(UiRect::px(
				0.,
				theme_spacing.borders.extra_small,
				0.,
				theme_spacing.borders.extra_small,
			))
			.background_color(colors.surface(Surface::SurfaceVariant))
			.border_color(colors.accent(Accent::Shadow));
		
		style_builder
			.switch_context(Self::HANDLE, None)
			.border(UiRect::all(Val::Px(theme_spacing.borders.extra_small)))
			.border_color(colors.accent(Accent::Shadow))
			.border_radius(BorderRadius::all(Val::Px(theme_spacing.icons.small)))
			.animated()
			.background_color(AnimatedVals {
				idle: colors.accent(Accent::Primary),
				hover: colors.container(Container::Primary).into(),
				..default()
			})
			.copy_from(theme_data.interaction_animation);
	}
	
	fn container(name: String) -> impl Bundle {
		(Name::new(name), NodeBundle::default())
	}
	
	fn bar_container() -> impl Bundle {
		(
			Name::new("Bar Container"),
			NodeBundle::default(),
			Interaction::default(),
			Scrollable::default(),
		)
	}
	
	fn bar() -> impl Bundle {
		(Name::new("Slider Bar"), NodeBundle::default())
	}
	
	fn handle(slider: Entity) -> impl Bundle {
		(
			Name::new("Handle"),
			ButtonBundle::default(),
			TrackedInteraction::default(),
			SliderDragHandle { slider },
			Draggable::default(),
			RelativeCursorPosition::default(),
			Scrollable::default(),
		)
	}
	
	fn readout_container() -> impl Bundle {
		(Name::new("Readout"), NodeBundle::default())
	}
}

pub trait UiSliderU8Ext {
	fn slider_u8(&mut self, config: SliderU8Config) -> UiBuilder<Entity>;
}

impl UiSliderU8Ext for UiBuilder<'_, Entity> {
	fn slider_u8(&mut self, config: SliderU8Config) -> UiBuilder<Entity> {
		let mut slider_u8 = SliderU8 {
			ratio: (config.initial_value - config.min) as f64 / (config.max - config.min) as f64,
			config: config.clone(),
			..default()
		};
		
		let label = config.label.unwrap_or_else(|| "".into());
		let has_label = label.len() > 0;
		let name = match has_label {
			true => format!("Slider [{}]", label.clone()),
			false => "Slider".into(),
		};
		
		let mut input = self.container(SliderU8::container(name), |container| {
			let input_id = container.id();
			
			slider_u8.label = container.label(LabelConfig { label, ..default() }).id();
			slider_u8.bar_container = container
				.container(
					(SliderU8::bar_container(), SliderBar { slider: input_id }),
					|bar_container| {
						slider_u8.bar = bar_container
							.container(SliderU8::bar(), |bar| {
								slider_u8.handle = bar.spawn(SliderU8::handle(input_id)).id();
							}).id();
					},
				)
				.id();
			
			slider_u8.readout_container = container
				.container(SliderU8::readout_container(), |readout_container| {
					slider_u8.readout = readout_container.label(LabelConfig::default()).id();
				})
				.id();
		});
		
		input.insert(slider_u8);
		
		input
	}
}

trait LerpU8 {fn lerp(self, to: Self, t: f64) -> Self;}
impl LerpU8 for u8 {
	fn lerp(self, to: Self, t: f64) -> Self {(self as f64).lerp(to as f64, t).round() as u8}
}
