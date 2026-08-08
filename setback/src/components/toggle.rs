use dioxus::prelude::*;

const TOGGLE_CSS: Asset = asset!("/assets/styling/toggle.css");

#[derive(Props, PartialEq, Clone)]
pub struct ToggleProps {
    checked: bool,
    label: String,
    description: String,
}

#[component]
pub fn Toggle(props: ToggleProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TOGGLE_CSS }
        
        label { class: "toggle",
            input { checked: "{props.checked}", r#type: "checkbox" }
            span { class: "toggle__track",
                span { class: "toggle__thumb" }
            }
            span { class: "toggle__label",
                "{props.label}"
                small { "{props.description}" }
            }
        }
    }
}