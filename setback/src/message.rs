use {
	crate::{
		state::*,
		keyboard::*
	},
	libsetback::*
};

#[derive(Debug, Clone)]
pub enum Message {
	SetScene(Scene),
	KeyEvent(Keybind),
	ServerEvent(ServerEvent),
	ApiResLogin(Result<LoginResponse, Error>),
	ApiResLogout(Result<(), Error>),
	ApiResId(Result<u32, Error>),
	ApiResUpdate(Result<Version, Error>),

	//main menu
	ClickSingleplayer,
	ClickMultiplayer,
	PromptLogin,
	RepromptLogin,
	SubmitCredentials {username: String, password: String},
	ClickCreatePrivateGame,
	CreatePrivateGame {password: String},
	ClickCreatePublicGame,
	ClickJoinPrivateGame,
	JoinPrivateGame {code: String, password: String},
	ClickBrowsePublicGames,
	Back,
	Exit,

	//settings
	ApplyVsync(bool),
	ApplyFullscreen(bool),
	ApplyAntialiasing(bool),
	ApplyMasterVolume(f64),
	ApplyMusicVolume(f64),
	ApplyEffectsVolume(f64),
	ApplyKeybindBid0(Keybind),
	ApplyKeybindBid2(Keybind),
	ApplyKeybindBid3(Keybind),
	ApplyKeybindBid4(Keybind),
	ApplyKeybindPlayCard1(Keybind),
	ApplyKeybindPlayCard2(Keybind),
	ApplyKeybindPlayCard3(Keybind),
	ApplyKeybindPlayCard4(Keybind),
	ApplyKeybindPlayCard5(Keybind),
	ApplyKeybindPlayCard6(Keybind),
	ApplySaveLoginCredentials(bool),
	Logout,
	Rebind,
	CancelRebind,

	//multiplayer

	//setup

	//game
}