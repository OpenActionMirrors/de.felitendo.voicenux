// The key images from the manifest (images/keys/<name>.svg). `cargo xtask icons` writes them from
// this table, and keys draw them again with a title below (see Key::state_image).
// Gray means "off". The glyph is always in the middle: OpenDeck shows these pictures before the
// plugin draws a key, and then there is no title yet.
use crate::icons::colors::{GRAY as G, RED};

pub struct KeyImage {
	pub glyph: &'static str,
	pub color: Option<&'static str>,
}

const fn k(glyph: &'static str) -> KeyImage {
	KeyImage { glyph, color: None }
}

const fn c(glyph: &'static str, color: &'static str) -> KeyImage {
	KeyImage { glyph, color: Some(color) }
}

pub static KEY_IMAGES: &[(&str, KeyImage)] = &[
	("voice", k("voice")),
	("sound", k("sound")),
	("voicechanger", c("voicechanger", G)),
	("voicechanger-on", k("voicechanger")),
	("unmuted", k("mic")),
	("muted", c("micOff", RED)),
	("hearmyself", c("headphones", G)),
	("hearmyself-on", k("headphones")),
	("background", c("background", G)),
	("background-on", k("background")),
	("random", k("random")),
	("next", k("next")),
	("stopsounds", c("stop", G)),
	("stopsounds-on", k("stop")),
	("soundvolume", k("volumeUp")),
	("hearsounds", c("hearsounds", G)),
	("hearsounds-on", k("hearsounds")),
];

pub fn key_image(name: &str) -> Option<&'static KeyImage> {
	KEY_IMAGES.iter().find(|(n, _)| *n == name).map(|(_, image)| image)
}
