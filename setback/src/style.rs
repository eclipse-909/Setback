use {
	keyframe::{num_traits::Float, AnimationSequence, CanTween},
	std::{cell::UnsafeCell, fmt::Debug, mem::MaybeUninit, ops::BitOrAssign},
	taffy::{
		style_helpers::TaffyAuto, AlignContent, AlignItems, AlignSelf, BoxGenerationMode, BoxSizing,
		CheapCloneStr, CompactLength, CoreStyle, Dimension, Display, FlexDirection,
		FlexWrap, FlexboxContainerStyle, FlexboxItemStyle, JustifyContent,
		LengthPercentage, LengthPercentageAuto, Overflow, Point, Position, Rect, Size,
	},
	vello::{
		kurbo::{self, RoundedRectRadii},
		peniko::{
			self, color::{ColorSpaceTag, DynamicColor, HueDirection}, Brush, Color, ColorStop, ColorStops, Extend, Gradient, GradientKind,
			Image,
		},
	},
};

/// A UI node's style properties (in addition to Taffy layout)
pub struct Style<S: CheapCloneStr = winit::keyboard::SmolStr> {
	pub dummy: core::marker::PhantomData<S>,
	pub display: Display,
	pub item_is_table: bool,
	pub item_is_replaced: bool,
	pub box_sizing: BoxSizing,
	pub overflow: Point<Overflow>,
	pub scrollbar_width: f32, //Animated<f32>
	pub position: Position,
	pub inset: Rect<LengthPercentageAuto>, //Rect<Animated<AnimCompactLength>>
	pub size: Size<Dimension>,             //Size<Animated<AnimCompactLength>>
	pub min_size: Size<Dimension>,         //Size<Animated<AnimCompactLength>>
	pub max_size: Size<Dimension>,         //Size<Animated<AnimCompactLength>>
	pub aspect_ratio: Option<f32>,         //Option<Animated<f32>>
	pub margin: Rect<LengthPercentageAuto>, //Rect<Animated<AnimCompactLength>>
	pub padding: Rect<LengthPercentage>,   //Rect<Animated<AnimCompactLength>>
	pub border: Rect<LengthPercentage>,    //Rect<Animated<AnimCompactLength>>
	pub align_items: Option<AlignItems>,
	pub align_self: Option<AlignSelf>,
	pub align_content: Option<AlignContent>,
	pub justify_content: Option<JustifyContent>,
	pub gap: Size<LengthPercentage>, //Size<Animated<AnimCompactLength>>
	pub flex_direction: FlexDirection,
	pub flex_wrap: FlexWrap,
	pub flex_basis: Dimension, //Animated<AnimCompactLength>
	pub flex_grow: f32,        //Animated<f32>
	pub flex_shrink: f32,      //Animated<f32>

	pub background: Brush,               //AnimBrush
	pub border_color: Color,             //AnimColor
	pub border_radius: RoundedRectRadii, //AnimRectRadii
	pub box_shadow: Option<BoxShadow>,
	pub opacity: f32, //Animated<f32>
	pub z_index: i32, //Animated<I32>
	pub clip: bool,
}

impl<S: CheapCloneStr> Style<S> {
	pub const DEFAULT: Self = Self {
		dummy: core::marker::PhantomData,
		display: Display::DEFAULT,
		item_is_table: false,
		item_is_replaced: false,
		box_sizing: BoxSizing::BorderBox,
		overflow: Point {
			x: Overflow::Visible,
			y: Overflow::Visible,
		},
		scrollbar_width: 0.,
		position: Position::Relative,
		inset: Rect::auto(),
		margin: Rect::zero(),
		padding: Rect::zero(),
		border: Rect::zero(),
		size: Size::auto(),
		min_size: Size::auto(),
		max_size: Size::auto(),
		aspect_ratio: None,
		gap: Size::zero(),
		align_items: None,
		align_self: None,
		align_content: None,
		justify_content: None,
		flex_direction: FlexDirection::Row,
		flex_wrap: FlexWrap::NoWrap,
		flex_grow: 0.,
		flex_shrink: 1.,
		flex_basis: Dimension::AUTO,

		background: Brush::Solid(Color::TRANSPARENT),
		border_color: Color::TRANSPARENT,
		border_radius: RoundedRectRadii {
			top_left: 0.,
			top_right: 0.,
			bottom_left: 0.,
			bottom_right: 0.,
		},
		box_shadow: None,
		opacity: 1.,
		z_index: 0,
		clip: false,
	};

	const fn rect_auto() -> Rect<Animated<AnimCompactLength>> {
		Rect {
			top: Animated::new_static(AnimCompactLength(CompactLength::auto())),
			bottom: Animated::new_static(AnimCompactLength(CompactLength::auto())),
			left: Animated::new_static(AnimCompactLength(CompactLength::auto())),
			right: Animated::new_static(AnimCompactLength(CompactLength::auto())),
		}
	}

	const fn rect_zero() -> Rect<Animated<AnimCompactLength>> {
		Rect {
			top: Animated::new_static(AnimCompactLength(CompactLength::length(0.))),
			bottom: Animated::new_static(AnimCompactLength(CompactLength::length(0.))),
			left: Animated::new_static(AnimCompactLength(CompactLength::length(0.))),
			right: Animated::new_static(AnimCompactLength(CompactLength::length(0.))),
		}
	}

	const fn size_auto() -> Size<Animated<AnimCompactLength>> {
		Size {
			width: Animated::new_static(AnimCompactLength(CompactLength::auto())),
			height: Animated::new_static(AnimCompactLength(CompactLength::auto())),
		}
	}

	const fn size_zero() -> Size<Animated<AnimCompactLength>> {
		Size {
			width: Animated::new_static(AnimCompactLength(CompactLength::length(0.))),
			height: Animated::new_static(AnimCompactLength(CompactLength::length(0.))),
		}
	}

	pub fn advance(&mut self, seconds: f64) -> bool {
		let mut r = 0;
		// let ret: *mut u8 = &mut r;
		// unsafe {
		// *ret += self.scrollbar_width.advance(seconds) as u8;
		// self.inset.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.margin.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.padding.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.border.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.size.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.min_size.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.max_size.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.aspect_ratio.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// self.gap.clone().map(|mut a| *ret += a.advance(seconds) as u8);
		// *ret += self.flex_grow.advance(seconds) as u8;
		// *ret += self.flex_shrink.advance(seconds) as u8;
		// *ret += self.flex_basis.advance(seconds) as u8;
		// *ret += self.background.advance(seconds) as u8;
		// *ret += self.border_color.advance(seconds) as u8;
		// *ret += self.border_radius.top_left.advance(seconds) as u8;
		// *ret += self.border_radius.top_right.advance(seconds) as u8;
		// *ret += self.border_radius.bottom_left.advance(seconds) as u8;
		// *ret += self.border_radius.bottom_right.advance(seconds) as u8;
		// self.box_shadow.clone().map(|mut a| {
		// 	*ret += a.blur.advance(seconds) as u8;
		// 	*ret += a.offset_x.advance(seconds) as u8;
		// 	*ret += a.offset_y.advance(seconds) as u8;
		// 	*ret += a.spread.advance(seconds) as u8;
		// 	*ret += a.color.advance(seconds) as u8;
		// });
		// *ret += self.opacity.advance(seconds) as u8;
		// *ret += self.z_index.advance(seconds) as u8;
		// }
		r > 0
	}
}

impl<S: CheapCloneStr> Default for Style<S> {
	fn default() -> Self {
		Self::DEFAULT
	}
}

impl<S: CheapCloneStr> CoreStyle for Style<S> {
	type CustomIdent = S;

	#[inline(always)]
	fn box_generation_mode(&self) -> BoxGenerationMode {
		match self.display {
			Display::None => BoxGenerationMode::None,
			_ => BoxGenerationMode::Normal,
		}
	}
	#[inline(always)]
	fn is_compressible_replaced(&self) -> bool {
		self.item_is_replaced
	}
	#[inline(always)]
	fn box_sizing(&self) -> BoxSizing {
		self.box_sizing
	}
	#[inline(always)]
	fn overflow(&self) -> Point<Overflow> {
		self.overflow
	}
	#[inline(always)]
	fn scrollbar_width(&self) -> f32 {
		self.scrollbar_width
	}
	#[inline(always)]
	fn position(&self) -> Position {
		self.position
	}
	#[inline(always)]
	fn inset(&self) -> Rect<LengthPercentageAuto> {
		self.inset
	}
	#[inline(always)]
	fn size(&self) -> Size<Dimension> {
		self.size
	}
	#[inline(always)]
	fn min_size(&self) -> Size<Dimension> {
		self.min_size
	}
	#[inline(always)]
	fn max_size(&self) -> Size<Dimension> {
		self.max_size
	}
	#[inline(always)]
	fn aspect_ratio(&self) -> Option<f32> {
		self.aspect_ratio
	}
	#[inline(always)]
	fn margin(&self) -> Rect<LengthPercentageAuto> {
		self.margin
	}
	#[inline(always)]
	fn padding(&self) -> Rect<LengthPercentage> {
		self.padding
	}
	#[inline(always)]
	fn border(&self) -> Rect<LengthPercentage> {
		self.border
	}
}

impl<S: CheapCloneStr> FlexboxContainerStyle for Style<S> {
	#[inline(always)]
	fn flex_direction(&self) -> FlexDirection {
		self.flex_direction
	}
	#[inline(always)]
	fn flex_wrap(&self) -> FlexWrap {
		self.flex_wrap
	}
	#[inline(always)]
	fn gap(&self) -> Size<LengthPercentage> {
		self.gap
	}
	#[inline(always)]
	fn align_content(&self) -> Option<AlignContent> {
		self.align_content
	}
	#[inline(always)]
	fn align_items(&self) -> Option<AlignItems> {
		self.align_items
	}
	#[inline(always)]
	fn justify_content(&self) -> Option<JustifyContent> {
		self.justify_content
	}
}

impl<S: CheapCloneStr> FlexboxItemStyle for Style<S> {
	#[inline(always)]
	fn flex_basis(&self) -> Dimension {
		self.flex_basis
	}
	#[inline(always)]
	fn flex_grow(&self) -> f32 {
		self.flex_grow
	}
	#[inline(always)]
	fn flex_shrink(&self) -> f32 {
		self.flex_shrink
	}
	#[inline(always)]
	fn align_self(&self) -> Option<AlignSelf> {
		self.align_self
	}
}

#[derive(Clone)]
pub struct BoxShadow {
	pub offset_x: Animated<f32>,
	pub offset_y: Animated<f32>,
	pub blur: Animated<f32>,
	pub spread: Animated<f64>,
	pub color: AnimColor,
}

pub trait Animatable: CanTween + Clone + Default {
	type Layout: Clone + Debug;
	fn as_layout(&self) -> Self::Layout;
}

pub struct Animated<A: Animatable> {
	tag: UnsafeCell<AnimatedTag<A>>,
	value: UnsafeCell<A>,
}

#[derive(Clone)]
pub enum AnimatedTag<A: Animatable> {
	Static,
	Animation(AnimationSequence<A>),
	Cached(AnimationSequence<A>),
}

impl<A: Animatable> Animated<A> {
	pub const fn new_static(a: A) -> Self {
		Self {
			tag: UnsafeCell::new(AnimatedTag::Static),
			value: UnsafeCell::new(a),
		}
	}
	pub fn get(&self) -> A::Layout {
		match unsafe { &*self.tag.get() } {
			AnimatedTag::Static | AnimatedTag::Cached(_) => unsafe {
				(*self.value.get()).as_layout()
			},
			AnimatedTag::Animation(sequence) => {
				let value = sequence.now();
				unsafe { *self.value.get() = value.clone() };
				self.swap();
				value.as_layout()
			}
		}
	}
	pub fn advance(&mut self, seconds: f64) -> bool {
		match self.tag.get_mut() {
			AnimatedTag::Static => return false,
			AnimatedTag::Animation(sequence) => {
				sequence.advance_and_maybe_wrap(seconds);
			}
			AnimatedTag::Cached(sequence) => {
				sequence.advance_and_maybe_wrap(seconds);
				self.swap();
			}
		}
		true
	}
	fn swap(&self) {
		let uninit: &mut MaybeUninit<UnsafeCell<AnimatedTag<A>>> =
			unsafe { std::mem::transmute(&mut *self.tag.get()) };
		let prev = std::mem::replace(uninit, MaybeUninit::uninit());
		let next = match unsafe { prev.assume_init().into_inner() } {
			AnimatedTag::Animation(s) => UnsafeCell::new(AnimatedTag::Cached(s)),
			AnimatedTag::Cached(s) => UnsafeCell::new(AnimatedTag::Animation(s)),
			_ => unreachable!(),
		};
		let _ = std::mem::replace(uninit, MaybeUninit::new(next));
	}
	pub fn dimension(f: Dimension) -> Animated<AnimCompactLength> {
		Animated::new_static(AnimCompactLength(f.into_raw()))
	}
	pub fn length_percentage_auto(f: LengthPercentageAuto) -> Animated<AnimCompactLength> {
		Animated::new_static(AnimCompactLength(f.into_raw()))
	}
	pub fn length_percentage(f: LengthPercentage) -> Animated<AnimCompactLength> {
		Animated::new_static(AnimCompactLength(f.into_raw()))
	}
}
impl<A: Animatable + Default> Default for Animated<A> {
	#[inline(always)]
	fn default() -> Self {
		Self::new_static(A::default())
	}
}
impl<T: Animatable> Debug for Animated<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Animated")
	}
}
impl<T: Animatable> Clone for Animated<T> {
	fn clone(&self) -> Self {
		Self {
			tag: UnsafeCell::new(unsafe { (*self.tag.get()).clone() }),
			value: UnsafeCell::new(unsafe { (*self.value.get()).clone() }),
		}
	}
}
impl<A: Animatable> From<A> for Animated<A> {
	fn from(value: A) -> Self {
		Animated::new_static(value)
	}
}

impl Animatable for f32 {
	type Layout = Self;
	#[inline(always)]
	fn as_layout(&self) -> Self::Layout {
		*self
	}
}

impl Animatable for f64 {
	type Layout = Self;
	#[inline(always)]
	fn as_layout(&self) -> Self::Layout {
		*self
	}
}

#[derive(Clone, Copy, Default)]
pub struct I32(pub i32);
impl Animatable for I32 {
	type Layout = i32;
	#[inline(always)]
	fn as_layout(&self) -> Self::Layout {
		self.0
	}
}
impl CanTween for I32 {
	fn ease(from: Self, to: Self, time: impl Float) -> Self {
		Self((from.0 as f64 + (to.0 - from.0) as f64 * time.to_f64().unwrap()) as i32)
	}
}

#[derive(Clone)]
pub struct AnimCompactLength(pub CompactLength);
impl Animatable for AnimCompactLength {
	type Layout = CompactLength;
	#[inline(always)]
	fn as_layout(&self) -> Self::Layout {
		self.0
	}
}
impl Default for AnimCompactLength {
	#[inline(always)]
	fn default() -> Self {
		Self(CompactLength::auto())
	}
}
impl CanTween for AnimCompactLength {
	fn ease(from: Self, to: Self, time: impl Float) -> Self {
		let tag = from.0.tag();
		if tag != to.0.tag() {
			return from;
		}
		let val = CanTween::ease(from.0.value(), to.0.value(), time);
		match tag {
			CompactLength::CALC_TAG
			| CompactLength::AUTO_TAG
			| CompactLength::MIN_CONTENT_TAG
			| CompactLength::MAX_CONTENT_TAG => to,
			CompactLength::LENGTH_TAG => Self(CompactLength::length(val)),
			CompactLength::PERCENT_TAG => Self(CompactLength::percent(val)),
			CompactLength::FR_TAG => Self(CompactLength::fr(val)),
			CompactLength::FIT_CONTENT_PX_TAG => Self(CompactLength::fit_content_px(val)),
			CompactLength::FIT_CONTENT_PERCENT_TAG => Self(CompactLength::fit_content_percent(val)),
			_ => unreachable!("Invalid CompactLength tag: {}", tag),
		}
	}
}

pub enum AnimBrush {
	StaticSolid(Color),
	StaticGradient(Gradient),
	StaticImage(Image),
	AnimSolid(AnimColor),
	AnimGradient(AnimGradient),
}
impl AnimBrush {
	pub fn get(&self) -> Brush {
		match self {
			AnimBrush::StaticSolid(s) => Brush::Solid(*s),
			AnimBrush::StaticGradient(g) => Brush::Gradient(g.clone()),
			AnimBrush::StaticImage(i) => Brush::Image(i.clone()),
			AnimBrush::AnimSolid(s) => Brush::Solid(s.get()),
			AnimBrush::AnimGradient(g) => Brush::Gradient(g.get()),
		}
	}
	fn advance(&mut self, seconds: f64) -> bool {
		match self {
			AnimBrush::AnimSolid(s) => s.advance(seconds),
			AnimBrush::AnimGradient(g) => g.advance(seconds),
			_ => false,
		}
	}
}
impl Default for AnimBrush {
	#[inline(always)]
	fn default() -> Self {
		Self::StaticSolid(Color::TRANSPARENT)
	}
}
impl From<Brush> for AnimBrush {
	fn from(brush: Brush) -> Self {
		match brush {
			Brush::Solid(c) => Self::StaticSolid(c),
			Brush::Gradient(g) => Self::StaticGradient(g),
			Brush::Image(i) => Self::StaticImage(i),
		}
	}
}
struct AnimGradient {
	pub kind: AnimGradientKind,
	pub extend: Extend,
	pub hue_direction: HueDirection,
	pub stops: AnimColorStops,
}
impl AnimGradient {
	fn get(&self) -> Gradient {
		Gradient {
			kind: self.kind.get(),
			extend: self.extend,
			interpolation_cs: ColorSpaceTag::Srgb,
			hue_direction: self.hue_direction,
			stops: self.stops.get(),
		}
	}
	fn advance(&mut self, seconds: f64) -> bool {
		let mut ret = false;
		ret.bitor_assign(self.kind.advance(seconds));
		ret.bitor_assign(self.stops.advance(seconds));
		ret
	}
}
enum AnimGradientKind {
	Linear {
		start: AnimPoint,
		end: AnimPoint,
	},
	Radial {
		start_center: AnimPoint,
		start_radius: Animated<f32>,
		end_center: AnimPoint,
		end_radius: Animated<f32>,
	},
	Sweep {
		center: AnimPoint,
		start_angle: Animated<f32>,
		end_angle: Animated<f32>,
	},
}
impl AnimGradientKind {
	fn get(&self) -> GradientKind {
		match self {
			AnimGradientKind::Linear { start, end } => GradientKind::Linear {
				start: start.get(),
				end: end.get(),
			},
			AnimGradientKind::Radial {
				start_center,
				start_radius,
				end_center,
				end_radius,
			} => GradientKind::Radial {
				start_center: start_center.get(),
				start_radius: start_radius.get(),
				end_center: end_center.get(),
				end_radius: end_radius.get(),
			},
			AnimGradientKind::Sweep {
				center,
				start_angle,
				end_angle,
			} => GradientKind::Sweep {
				center: center.get(),
				start_angle: start_angle.get(),
				end_angle: end_angle.get(),
			},
		}
	}
	fn advance(&mut self, seconds: f64) -> bool {
		let mut ret = false;
		match self {
			AnimGradientKind::Linear { start, end } => {
				ret.bitor_assign(start.advance(seconds));
				ret.bitor_assign(end.advance(seconds));
			}
			AnimGradientKind::Radial {
				start_center,
				start_radius,
				end_center,
				end_radius,
			} => {
				ret.bitor_assign(start_center.advance(seconds));
				ret.bitor_assign(start_radius.advance(seconds));
				ret.bitor_assign(end_center.advance(seconds));
				ret.bitor_assign(end_radius.advance(seconds));
			}
			AnimGradientKind::Sweep {
				center,
				start_angle,
				end_angle,
			} => {
				ret.bitor_assign(center.advance(seconds));
				ret.bitor_assign(start_angle.advance(seconds));
				ret.bitor_assign(end_angle.advance(seconds));
			}
		}
		ret
	}
}
pub struct AnimPoint {
	pub x: Animated<f64>,
	pub y: Animated<f64>,
}
impl AnimPoint {
	fn get(&self) -> kurbo::Point {
		kurbo::Point {
			x: self.x.get(),
			y: self.y.get(),
		}
	}
	fn advance(&mut self, seconds: f64) -> bool {
		let mut ret = false;
		ret.bitor_assign(self.x.advance(seconds));
		ret.bitor_assign(self.y.advance(seconds));
		ret
	}
}
struct AnimColorStops(pub smallvec::SmallVec<[AnimColorStop; 4]>);
impl AnimColorStops {
	fn get(&self) -> ColorStops {
		let mut stops = ColorStops::new();
		for stop in self.0.iter() {
			stops.push(stop.get());
		}
		stops
	}
	fn advance(&mut self, seconds: f64) -> bool {
		let mut ret = false;
		for stop in self.0.iter_mut() {
			ret.bitor_assign(stop.advance(seconds));
		}
		ret
	}
}
struct AnimColorStop {
	pub offset: Animated<f32>,
	pub color: AnimColor,
}
impl AnimColorStop {
	fn get(&self) -> ColorStop {
		ColorStop {
			offset: self.offset.get(),
			color: DynamicColor::from_alpha_color(self.color.get()),
		}
	}
	fn advance(&mut self, seconds: f64) -> bool {
		let mut ret = false;
		ret.bitor_assign(self.offset.advance(seconds));
		ret.bitor_assign(self.color.advance(seconds));
		ret
	}
}

#[derive(Default)]
pub struct AnimRectRadii {
	pub top_left: Animated<f64>,
	pub top_right: Animated<f64>,
	pub bottom_right: Animated<f64>,
	pub bottom_left: Animated<f64>,
}
impl From<RoundedRectRadii> for AnimRectRadii {
	fn from(radii: RoundedRectRadii) -> Self {
		Self {
			top_left: Animated::new_static(radii.top_left),
			top_right: Animated::new_static(radii.top_right),
			bottom_right: Animated::new_static(radii.bottom_right),
			bottom_left: Animated::new_static(radii.bottom_left),
		}
	}
}

#[derive(Clone, Copy, Default)]
struct LabColor {
	l: f32,
	a: f32,
	b: f32,
	alpha: u8,
}
impl LabColor {
	const fn midpoint(self, other: Self) -> Self {
		Self {
			l: self.l.midpoint(other.l),
			a: self.a.midpoint(other.a),
			b: self.b.midpoint(other.b),
			alpha: self.alpha.midpoint(other.alpha),
		}
	}
}
impl From<Color> for LabColor {
	fn from(c: Color) -> Self {
		let peniko::color::Rgba8 { r, g, b, a } = c.to_rgba8();
		let lab = lab::Lab::from_rgb(&[r, g, b]);
		LabColor {
			l: lab.l,
			a: lab.a,
			b: lab.b,
			alpha: a,
		}
		.into()
	}
}
impl From<LabColor> for Color {
	fn from(c: LabColor) -> Self {
		let rgb = lab::Lab {
			l: c.l,
			a: c.a,
			b: c.a,
		}
		.to_rgb();
		Color::from_rgba8(rgb[0], rgb[1], rgb[2], c.alpha)
	}
}
impl CanTween for LabColor {
	fn ease(from: Self, to: Self, time: impl Float) -> Self {
		Self {
			l: from.l + (to.l - from.l) * time.to_f32().unwrap(),
			a: from.a + (to.a - from.a) * time.to_f32().unwrap(),
			b: from.b + (to.b - from.b) * time.to_f32().unwrap(),
			alpha: (from.alpha as f32 + (to.alpha - from.alpha) as f32 * time.to_f32().unwrap())
				as u8,
		}
	}
}
#[derive(Clone)]
pub enum AnimColor {
	Static(Color),
	Animation(AnimationSequence<LabColor>),
}

impl AnimColor {
	pub fn get(&self) -> Color {
		match self {
			Self::Static(a) => *a,
			Self::Animation(s) => s.now().into(),
		}
	}

	fn advance(&mut self, seconds: f64) -> bool {
		let Self::Animation(sequence) = self else {
			return false;
		};
		sequence.advance_and_maybe_wrap(seconds);
		true
	}
}

impl Debug for AnimColor {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "AnimColor")
	}
}
