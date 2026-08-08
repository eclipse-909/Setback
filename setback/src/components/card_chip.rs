use {
	dioxus::prelude::*,
	libsetback::*,
};

const CARD_CHIP_CSS: Asset = asset!("/assets/styling/card-chip.css");

#[derive(Props, PartialEq, Clone)]
pub struct CardChipProps {
	num: u8,
	suit: Suit,
	face_up: bool,
	size: CardChipSize,
	interactive: bool,
}

impl Default for CardChipProps {
	fn default() -> Self {
		Self {
			num: 2,
			suit: Suit::Hearts,
			face_up: true,
			size: CardChipSize::Md,
			interactive: false,
		}
	}
}

#[derive(PartialEq, Clone, Copy)]
pub enum CardChipSize { Sm, Md, Lg }

impl CardChipSize {
	const fn as_class(self) -> &'static str {
		match self {
			CardChipSize::Sm => "card-chip--sm",
			CardChipSize::Md => "card-chip--md",
			CardChipSize::Lg => "card-chip--lg",
		}
	}
}

#[component]
pub fn CardChip(props: CardChipProps) -> Element {
	let interactive = if props.interactive { "card-chip--interactive" } else { "" };
	let red = if props.suit.is_red() { "card-chip--red" } else { "" };
	rsx! {
		document::Link { rel: "stylesheet", href: CARD_CHIP_CSS }
		
		div { class: "card-chip {props.size.as_class()} {interactive} {red}",
			if props.face_up {
				span { class: "card-chip__corner",
					"{Card::num_small(props.num)}"
					br {}
					"{props.suit.as_icon()}"
				}
				span { class: "card-chip__pip", "{props.suit.as_icon()}" }
			}
		}
	}
}