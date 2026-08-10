mod btn;
mod card_chip;
mod dropdown;
mod game_list_item;
mod info_tooltip;
mod keybind_row;
mod modal;
mod option_card;
mod panel;
mod player_node;
mod settings;
mod slider;
mod tabs;
mod text_input;
mod toast;
mod toggle;

use dioxus::prelude::*;

pub use {
	btn::*, card_chip::*, dropdown::*, game_list_item::*, info_tooltip::*, keybind_row::*,
	modal::*, option_card::*, panel::*, player_node::*, settings::*, slider::*, tabs::*,
	text_input::*, toast::*, toggle::*,
};

pub const BUTTON_CSS: Asset = asset!("/assets/styling/button.css");
pub const CARD_CHIP_CSS: Asset = asset!("/assets/styling/card-chip.css");
pub const DROPDOWN_CSS: Asset = asset!("/assets/styling/dropdown.css");
pub const GAME_LIST_ITEM_CSS: Asset = asset!("/assets/styling/game-list-item.css");
pub const INFO_TOOLTIP_CSS: Asset = asset!("/assets/styling/info-tooltip.css");
pub const KEYBIND_ROW_CSS: Asset = asset!("/assets/styling/keybind-row.css");
pub const MODAL_CSS: Asset = asset!("/assets/styling/modal.css");
pub const OPTION_CARD_CSS: Asset = asset!("/assets/styling/option-card.css");
pub const PANEL_CSS: Asset = asset!("/assets/styling/panel.css");
pub const PLAYER_NODE_CSS: Asset = asset!("/assets/styling/player-node.css");
pub const SETTINGS_CSS: Asset = asset!("/assets/styling/settings.css");
pub const SLIDER_CSS: Asset = asset!("/assets/styling/slider.css");
pub const TABS_CSS: Asset = asset!("/assets/styling/tabs.css");
pub const TEXT_INPUT_CSS: Asset = asset!("/assets/styling/text-input.css");
pub const TOAST_CSS: Asset = asset!("/assets/styling/toast.css");
pub const TOGGLE_CSS: Asset = asset!("/assets/styling/toggle.css");
