use {crate::components::*, dioxus::prelude::*};

#[derive(Props, PartialEq, Clone)]
pub struct PanelProps {
	#[props(default)]
	size: PanelSize,
	children: Element,
}

#[derive(PartialEq, Clone, Copy, Default)]
pub enum PanelSize {
	Sm,
	#[default]
	Md,
	Lg,
	Wide,
}

impl PanelSize {
	const fn as_class(self) -> &'static str {
		match self {
			PanelSize::Sm => "panel--sm",
			PanelSize::Md => "panel--md",
			PanelSize::Lg => "panel--lg",
			PanelSize::Wide => "panel--wide",
		}
	}
}

#[component]
pub fn Panel(props: PanelProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: PANEL_CSS }

		div { class: "panel {props.size.as_class()}",
			{props.children}
		}
	}
}

#[derive(Props, PartialEq, Clone)]
pub struct PanelInnerProps {
	#[props(default)]
	eyebrow: String,
	#[props(default)]
	title: String,
	#[props(default)]
	subtitle: String,
	ghost_btn_txt: String,
	primary_btn_txt: String,
	children: Element,
}

#[component]
pub fn PanelInner(props: PanelInnerProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: PANEL_CSS }
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		div { class: "panel__inner",
			div { class: "panel__header",
				if !props.eyebrow.is_empty() { p { class: "eyebrow", "{props.eyebrow}" } }
				if !props.title.is_empty() { h2 { class: "panel__title", "{props.title}" } }
				if !props.subtitle.is_empty() { p { class: "panel__subtitle", "{props.subtitle}" } }
			}
			div { class: "panel__body",
				{props.children}
			}
			div { class: "panel__footer",
				Btn { variant: BtnVariant::Ghost, "{props.ghost_btn_txt}" }
				Btn { variant: BtnVariant::Primary, "{props.primary_btn_txt}" }
			}
		}
	}
}
