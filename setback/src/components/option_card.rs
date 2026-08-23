use {
	crate::{Route, components::*},
	dioxus::prelude::*,
};

#[component]
pub fn OptionCardGrid(children: Element) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: OPTION_CARD_CSS }

		div {
			class: "option-grid",
			"data-option-group": "false",
			id: "visibilityGroup",
			{children}
		}
	}
}

#[derive(Props, PartialEq, Clone)]
pub struct OptionCardProps {
	#[props(default)]
	selected: bool,
	#[props(default)]
	to: Option<Route>,
	#[props(default)]
	onclick: Option<EventHandler<MouseEvent>>,
	title: String,
	description: String,
	/// svg icon
	children: Element,
}

#[component]
pub fn OptionCard(props: OptionCardProps) -> Element {
	let inner = rsx! {
		span { class: "option-card__icon",
			{props.children}
		}
		span { class: "option-card__title", "{props.title}" }
		span { class: "option-card__desc", "{props.description}" }
	};
	let is_selected = if props.selected { "is-selected" } else { "" };
	rsx! {
		document::Link { rel: "stylesheet", href: OPTION_CARD_CSS }

		if let Some(route) = props.to {
			Link {
				class: "option-card {is_selected}",
				to: route,
				{inner}
			}
		} else if let Some(callback) = props.onclick {
			button {
				class: "option-card {is_selected}",
				r#type: "button",
				onclick: move |evt| {callback.call(evt);},
				{inner}
			}
		} else {
			div {
				class: "option-card {is_selected}",
				"data-visibility": "{props.title}",
				{inner}
			}
		}
	}
}
