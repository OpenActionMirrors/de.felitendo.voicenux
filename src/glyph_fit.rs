// Written by `cargo xtask fit-glyphs`: center and size of every glyph, in grid units.
use crate::icons::Fit;

pub static FIT: &[(&str, Fit)] = &[
	("plugin", Fit { cx: 12.0, cy: 12.499, w: 20.0, h: 12.0 }),
	("voicechanger", Fit { cx: 12.0, cy: 12.0, w: 20.0, h: 17.0 }),
	("voice", Fit { cx: 12.0, cy: 11.68, w: 19.2, h: 19.6 }),
	("sound", Fit { cx: 12.465, cy: 11.385, w: 18.3, h: 18.1 }),
	("mic", Fit { cx: 12.0, cy: 12.03, w: 17.2, h: 21.2 }),
	("micOff", Fit { cx: 11.912, cy: 12.044, w: 20.4, h: 21.2 }),
	("headphones", Fit { cx: 12.0, cy: 13.411, w: 19.4, h: 18.6 }),
	("hearsounds", Fit { cx: 12.027, cy: 13.607, w: 19.4, h: 18.6 }),
	("background", Fit { cx: 11.9, cy: 12.5, w: 21.2, h: 14.2 }),
	("random", Fit { cx: 12.0, cy: 12.0, w: 19.0, h: 19.0 }),
	("next", Fit { cx: 11.344, cy: 12.0, w: 15.7, h: 14.4 }),
	("previous", Fit { cx: 12.656, cy: 12.0, w: 15.7, h: 14.4 }),
	("stop", Fit { cx: 12.0, cy: 12.0, w: 13.0, h: 13.0 }),
	("volume", Fit { cx: 11.782, cy: 12.0, w: 20.3, h: 15.6 }),
	("volumeUp", Fit { cx: 11.328, cy: 12.0, w: 21.0, h: 15.6 }),
	("volumeDown", Fit { cx: 10.898, cy: 12.0, w: 21.0, h: 15.6 }),
	("volumeSet", Fit { cx: 11.492, cy: 12.0, w: 21.0, h: 15.6 }),
	("power", Fit { cx: 12.0, cy: 13.103, w: 20.4, h: 21.35 }),
	("plug", Fit { cx: 12.0, cy: 11.505, w: 14.6, h: 21.4 }),
	("star", Fit { cx: 12.0, cy: 11.965, w: 19.8, h: 19.0 }),
];
