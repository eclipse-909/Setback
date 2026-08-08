use dioxus::prelude::*;

const PANEL_CSS: Asset = asset!("/assets/styling/panel.css");
const BUTTON_CSS: Asset = asset!("/assets/styling/button.css");

#[derive(Props, PartialEq, Clone)]
pub struct PanelProps {
    size: PanelSize,
    children: Element,
}

#[derive(PartialEq, Clone, Copy)]
pub enum PanelSize { Sm, Md, Lg, Wide }

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
    eyebrow: String,
    title: String,
    subtitle: String,
    body: String,
    ghost_btn_txt: String,
    primary_btn_txt: String,
}

#[component]
pub fn PanelInner(props: PanelInnerProps) -> Element {
    rsx! {
	    document::Link { rel: "stylesheet", href: PANEL_CSS }
	    document::Link { rel: "stylesheet", href: BUTTON_CSS }

        div { class: "panel__inner",
            div { class: "panel__header",
                p { class: "eyebrow", "{props.eyebrow}" }
                h2 { class: "panel__title", "{props.title}" }
                p { class: "panel__subtitle", "{props.subtitle}" }
            }
            div { class: "panel__body",
                p { style: "color: var(--cream-300);",
                    "{props.body}"
                }
            }
            div { class: "panel__footer",
                button { class: "btn btn--ghost btn--md", "{props.ghost_btn_txt}" }
                button { class: "btn btn--primary btn--md", "{props.primary_btn_txt}" }
            }
        }
	}
}