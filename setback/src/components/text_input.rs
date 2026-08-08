use dioxus::prelude::*;

const TEXT_INPUT_CSS: Asset = asset!("/assets/styling/text-input.css");

#[derive(Props, PartialEq, Clone)]
pub struct TextInputProps {
    label: String,
    placeholder: String,
    input_type: TextInputType,
    has_err: bool,
    err_msg: String,
    input_id: String,
}

#[derive(PartialEq, Clone)]
pub enum TextInputType {
    Text,
    Code,
    Password,
}

#[component]
pub fn TextInput(props: TextInputProps) -> Element {
    let has_err = if props.has_err { "has-error" } else { "" };
    let input = match props.input_type {
        TextInputType::Text => rsx! {
            input {
                class: "text-input {has_err}",
                id: "{props.input_id}",
                placeholder: "{props.placeholder}",
                r#type: "text",
            }
        },
        TextInputType::Code => rsx! {
            input {
                class: "text-input text-input--code {has_err}",
                id: "{props.input_id}",
                maxlength: "6",
                placeholder: "{props.placeholder}",
                r#type: "text",
            }
        },
        TextInputType::Password => rsx! {
            input {
                class: "text-input {has_err}",
                id: "{props.input_id}",
                r#type: "password"
            }
        }
    };
    rsx! {
        document::Link { rel: "stylesheet", href: TEXT_INPUT_CSS }
        
        div { class: "field",
            label { class: "field__label", r#for: "nick", "Nickname" }
            {input}
            if props.has_err {
                p { class: "field__hint field__hint--error", "{props.err_msg}" }
            }
        }
    }
}