use dioxus::prelude::*;

const MODAL_CSS: Asset = asset!("/assets/styling/modal.css");
const BUTTON_CSS: Asset = asset!("/assets/styling/button.css");
const PANEL_CSS: Asset = asset!("/assets/styling/panel.css");

#[component]
pub fn Modal(children: Element) -> Element {
	rsx! {
        document::Link { rel: "stylesheet", href: MODAL_CSS }
        document::Link { rel: "stylesheet", href: BUTTON_CSS }
        document::Link { rel: "stylesheet", href: PANEL_CSS }
        
		div { class: "modal-overlay", hidden: "false", id: "tableMenuModal",
			div {
				aria_labelledby: "tableMenuTitle",
				aria_modal: "true",
				class: "modal panel panel--sm",
				role: "dialog",
				button {
					aria_label: "Close",
					class: "modal__close",
					"data-modal-close": "false",
					svg {
						fill: "none",
						stroke: "currentColor",
						stroke_width: "2",
						view_box: "0 0 24 24",
						path { d: "M18 6L6 18M6 6l12 12" }
					}
				}
				{children}
			}
		}
	}
}