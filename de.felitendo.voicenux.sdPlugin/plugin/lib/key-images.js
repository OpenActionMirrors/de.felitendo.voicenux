// The key images from the manifest (images/keys/<name>.svg). scripts/build-icons.mjs writes them
// from this table, and keys draw them again with a title below (see Action.stateImage).
// Gray means "off". The glyph is always in the middle: OpenDeck shows these pictures before the
// plugin draws a key, and then there is no title yet.
import { COLORS } from "./icons.js";

const G = COLORS.gray;

export const KEY_IMAGES = {
	voice: { glyph: "voice" },
	sound: { glyph: "sound" },
	voicechanger: { glyph: "voicechanger", color: G },
	"voicechanger-on": { glyph: "voicechanger" },
	unmuted: { glyph: "mic" },
	muted: { glyph: "micOff", color: COLORS.red },
	hearmyself: { glyph: "headphones", color: G },
	"hearmyself-on": { glyph: "headphones" },
	background: { glyph: "background", color: G },
	"background-on": { glyph: "background" },
	random: { glyph: "random" },
	next: { glyph: "next" },
	stopsounds: { glyph: "stop", color: G },
	"stopsounds-on": { glyph: "stop" },
	soundvolume: { glyph: "volumeUp" },
	hearsounds: { glyph: "hearsounds", color: G },
	"hearsounds-on": { glyph: "hearsounds" },
};
