use {
	crate::{Route, components::*},
	dioxus::prelude::*,
};

#[derive(Props, PartialEq, Clone)]
pub struct BtnProps {
	#[props(default)]
	variant: BtnVariant,
	#[props(default)]
	size: BtnSize,
	#[props(default)]
	full: bool,
	#[props(default)]
	onclick: Option<EventHandler<MouseEvent>>,
	children: Element,
}

#[derive(PartialEq, Clone, Copy, Default)]
pub enum BtnVariant {
	#[default]
	None,
	Primary,
	Ghost,
	Danger,
	Link,
}

impl BtnVariant {
	const fn as_class(self) -> &'static str {
		match self {
			BtnVariant::None => "",
			BtnVariant::Primary => "btn--primary",
			BtnVariant::Ghost => "btn--ghost",
			BtnVariant::Danger => "btn--danger",
			BtnVariant::Link => "btn--link",
		}
	}
}

#[derive(PartialEq, Clone, Copy, Default)]
pub enum BtnSize {
	Lg,
	#[default]
	Md,
	Sm,
	Icon,
}

impl BtnSize {
	const fn as_class(self) -> &'static str {
		match self {
			BtnSize::Lg => "btn--lg",
			BtnSize::Md => "btn--md",
			BtnSize::Sm => "btn--sm",
			BtnSize::Icon => "btn--icon",
		}
	}
}

#[component]
pub fn Btn(props: BtnProps) -> Element {
	let full = if props.full { "btn--full" } else { "" };
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		button {
			class: "btn {props.variant.as_class()} {props.size.as_class()} {full}",
			onclick: move |evt| {props.onclick.inspect(|callback| {callback.call(evt);});},
			r#type: "button",
			{props.children}
		}
	}
}

#[derive(Props, PartialEq, Clone)]
pub struct LinkBtnProps {
	to: Route,
	#[props(default)]
	variant: BtnVariant,
	#[props(default)]
	size: BtnSize,
	#[props(default)]
	full: bool,
	#[props(default)]
	onclick: Option<EventHandler<MouseEvent>>,
	children: Element,
}

#[component]
pub fn LinkBtn(props: LinkBtnProps) -> Element {
	let full = if props.full { "btn--full" } else { "" };
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		Link {
			to: props.to,
			class: "btn {props.variant.as_class()} {props.size.as_class()} {full}",
			onclick: props.onclick,
			{props.children}
		}
	}
}

#[derive(Props, PartialEq, Clone)]
pub struct ModalBtnProps {
	modal: String,
	#[props(default)]
	variant: BtnVariant,
	#[props(default)]
	size: BtnSize,
	#[props(default)]
	full: bool,
	#[props(default)]
	onclick: Option<EventHandler<MouseEvent>>,
	children: Element,
}

/// Button that opens a modal
#[component]
pub fn ModalBtn(props: ModalBtnProps) -> Element {
	let full = if props.full { "btn--full" } else { "" };
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		button {
			class: "btn {props.variant.as_class()} {props.size.as_class()} {full}",
			"data-modal-open": props.modal,
			onclick: move |evt| {props.onclick.inspect(|callback| {callback.call(evt);});},
			r#type: "button",
			{props.children}
		}
	}
}

#[derive(Props, PartialEq, Clone)]
pub struct BackBtnProps {
	to: Route,
	#[props(default)]
	onclick: Option<EventHandler<MouseEvent>>,
}

#[component]
pub fn BackBtn(props: BackBtnProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		Link {
			to: props.to,
			class: "btn btn--ghost btn--icon back-button",
			onclick: props.onclick,
			aria_label: "Back",

			svg {
				fill: "none",
				stroke: "currentColor",
				stroke_width: "2",
				view_box: "0 0 24 24",
				path { d: "M15 18l-6-6 6-6" }
			}
		}
	}
}
