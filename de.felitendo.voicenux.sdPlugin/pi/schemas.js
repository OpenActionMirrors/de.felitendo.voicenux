// What each settings page shows. Field types: select, number, checkbox, segmented, tiles, note.
// section: where a field goes. Without one it is at the top: what the key is about (voice, sound).
// CONTROLS (what press, hold and double press do) and LOOK ("On the key", dials: lookTitle) are open.
// Any other name is a section of its own that is closed until opened.
// tiles: for choices that change how the key looks. Like segmented, with a small picture of the
// key above each choice: preview(value, settings, page) is its SVG. value(settings) and apply(value)
// can map the choice onto more than one setting. page has the lists and pictureUrl().
// The texts are English. src/i18n/de.rs has the German ones.
import { t } from "./lib/i18n.js";
import { COLORS, VOLUME_GLYPHS } from "./lib/icons.js";
import { glyphKeySvg, pictureKeySvg } from "./lib/render.js";

export const CONTROLS = "Controls";
export const LOOK = "look";
const MORE = "More options";
// Dials have no key: their look is on the touch strip.
const DIAL = { lookTitle: "On the display" };

// Press, hold and double press offer the same choices (the plugin side: Action.gesture).
// pressOnly: choices only for press, they use the key going down and up (push to talk).
// repeat: hold can also repeat the press while the key is held.
const gestures = (key, options, { pressOnly = [], repeat = false } = {}) => {
	const more = [["none", "Nothing"], ...(repeat ? [["repeat", "Keep going while held"]] : []), ...options.filter(([v]) => v !== "none" && !pressOnly.includes(v))];
	const free = (s) => !pressOnly.includes(s[key] ?? options[0][0]);
	return [
		{ key, label: "Press does", type: "select", options, section: CONTROLS },
		{ key: "hold", label: "Hold does", type: "select", options: more, showIf: free, section: CONTROLS },
		{ key: "double", label: "Double press", type: "select", options: more.filter(([v]) => v !== "repeat"), showIf: free, section: CONTROLS },
		{ type: "note", text: "With a double press set, a single press waits a moment for the second one.", showIf: (s) => free(s) && (s.double ?? "none") !== "none", section: CONTROLS },
	];
};

// Does press, hold or double press use one of these choices? first: the press default.
const uses = (s, key, choices, first) => [s[key] ?? first, s.hold, s.double].some((v) => choices.includes(v));

const TOGGLE = [
	["toggle", "Toggle"],
	["on", "Turn on"],
	["off", "Turn off"],
];

const TILE_OPTIONS = [
	["off", "Off"],
	["below", "Below"],
	["above", "Above"],
];

// Where a key shows its title: not at all, or below or above the glyph. on and at are the
// settings behind it. key(settings, page) gives the SVG of the key with a title at "below" or "above".
const titleTiles = (label, on, at, key) => ({
	key: at,
	label,
	type: "tiles",
	section: LOOK,
	options: TILE_OPTIONS,
	value: (s) => (s[on] === false ? "off" : (s[at] ?? "below")),
	apply: (v) => (v === "off" ? { [on]: false } : { [on]: true, [at]: v }),
	preview: (v, s, page) => key(v === "off" ? "" : v, s, page),
});

// The same tiles for keys whose title is off unless turned on (the switches).
const stateTiles = (label, glyph, text) => ({
	...titleTiles(label, "showState", "stateAt", (at, s) => glyphKeySvg(glyph(s), { title: at ? t(text) : "", titleAt: at || "below" })),
	value: (s) => (s.showState ? (s.stateAt ?? "below") : "off"),
});

// The voice or sound a key is set to, as it looks on the key.
const itemKey = (list, fallback, frame) => (at, s, page) => {
	const item = page.find(list, s[list === "voices" ? "voice" : "sound"]);
	const title = at ? (item?.label ?? t(list === "voices" ? "Voice" : "Sound")) : "";
	return pictureKeySvg({ src: item ? page.pictureUrl(item.icon) : null, fallback, title, titleAt: at || "below", frame: frame(s) });
};

const ID = "de.felitendo.voicenux";

// Every dial: a touch does what a press does, unless touches are off.
const TOUCH = { key: "disableTouch", label: "Touch screen", type: "checkbox", text: "Ignore touches", section: MORE };

// Keys that listen: the language you speak, for the speech recognition of Voicenux.
const LANGUAGE = {
	key: "language",
	label: "You speak",
	type: "select",
	options: [
		["auto", "Automatic"],
		["en", "English"],
		["de", "German"],
	],
	section: CONTROLS,
};

// held: the names of the two choices that hold the switch only while the key is held, and the note.
const HELD = ["On only while held", "Off only while held", "On only while held: it turns off again when you let go of the key. Off only while held is the other way round."];
const switchSchema = (glyph, help, [holdOn, holdOff, note] = HELD) => ({
	help,
	fields: [
		...gestures("mode", [...TOGGLE, ["holdOn", holdOn], ["holdOff", holdOff]], { pressOnly: ["holdOn", "holdOff"] }),
		{ type: "note", text: note, showIf: (s) => ["holdOn", "holdOff"].includes(s.mode), section: CONTROLS },
		stateTiles("On or off", () => glyph, "On"),
	],
});

const stepSchema = (first) => ({
	help: "With favorites starred in Voicenux, next and previous go through them only. Random picks from your favorites when you have two or more.",
	fields: [
		...gestures("direction", [
			["next", "Next voice"],
			["previous", "Previous voice"],
			["random", "Random voice"],
		].sort(([a], [b]) => (a === first ? -1 : b === first ? 1 : 0))),
		{
			key: "look",
			label: "Picture",
			type: "tiles",
			section: LOOK,
			options: [
				["icon", "Icon"],
				["voice", "Voice in use"],
			],
			preview: (v, s, page) => {
				const voice = page.currentVoice();
				const title = s.showName === false ? "" : (voice?.label ?? t("Voice"));
				const glyph = { next: "next", previous: "previous", random: "random" }[s.direction ?? first];
				if (v === "icon" || !voice) return glyphKeySvg(glyph, { title, titleAt: s.nameAt });
				return pictureKeySvg({ src: page.pictureUrl(voice.icon), title, titleAt: s.nameAt });
			},
		},
		titleTiles("Name of the voice", "showName", "nameAt", (at, s, page) => {
			const voice = page.currentVoice();
			const title = at ? (voice?.label ?? t("Voice")) : "";
			const glyph = { next: "next", previous: "previous", random: "random" }[s.direction ?? first];
			if (s.look !== "voice" || !voice) return glyphKeySvg(glyph, { title, titleAt: at || "below" });
			return pictureKeySvg({ src: page.pictureUrl(voice.icon), title, titleAt: at || "below" });
		}),
	],
});

export const SCHEMAS = {
	[`${ID}.voice`]: {
		fields: [
			{ key: "voice", label: "Voice", type: "select", list: "voices" },
			...gestures(
				"press",
				[
					["toggle", "This voice on or off"],
					["use", "Use this voice"],
					["whileHeld", "Use it while held"],
					["favorite", "Star or unstar it"],
				],
				{ pressOnly: ["whileHeld"] },
			),
			{ type: "note", text: "Off means your own voice: the voice changer turns off.", showIf: (s) => uses(s, "press", ["toggle"], "toggle"), section: CONTROLS },
			{ type: "note", text: "Use it while held: when you let go of the key, the voice from before comes back.", showIf: (s) => s.press === "whileHeld", section: CONTROLS },
			titleTiles("Name", "showName", "nameAt", itemKey("voices", "voice", () => null)),
			{ key: "showFrame", section: LOOK, label: "Frame", type: "checkbox", text: "While the voice is in use" },
			{ type: "note", text: "The frame turns gray while the voice changer is off.", section: LOOK, showIf: (s) => s.showFrame !== false },
		],
	},
	[`${ID}.sound`]: {
		fields: [
			{ key: "sound", label: "Sound", type: "select", list: "sounds" },
			...gestures("press", [
				["restart", "Play from the start"],
				["toggle", "Play and stop"],
				["overlap", "Play on top"],
				["stop", "Stop"],
			]),
			{ type: "note", text: "Play from the start: a press while it plays starts it over.", showIf: (s) => (s.press ?? "restart") === "restart", section: CONTROLS },
			{ type: "note", text: "Play and stop: a press while it plays stops it.", showIf: (s) => s.press === "toggle", section: CONTROLS },
			{ type: "note", text: "Play on top: every press starts it once more, over the one that still plays.", showIf: (s) => s.press === "overlap", section: CONTROLS },
			titleTiles("Name", "showName", "nameAt", itemKey("sounds", "sound", () => null)),
			{ key: "showPlaying", section: LOOK, label: "Frame", type: "checkbox", text: "While the sound plays" },
		],
	},
	[`${ID}.voicechanger`]: switchSchema("voicechanger", "Turns the voice changer on or off. When it is off, others hear your own voice.", [
		"Push to change voice",
		"Push for my own voice",
		"Push to change voice: others hear your own voice, and the changed one only while you hold the key. Push for my own voice is the other way round.",
	]),
	[`${ID}.mute`]: {
		help: "Mutes your microphone in Voicenux. Others hear nothing, sounds of the soundboard still play.",
		fields: [
			...gestures(
				"mode",
				[
					["toggle", "Toggle"],
					["mute", "Mute"],
					["unmute", "Unmute"],
					["talk", "Push to talk"],
					["holdMute", "Push to mute"],
				],
				{ pressOnly: ["talk", "holdMute"] },
			),
			{ type: "note", text: "Push to talk: muted, and live only while you hold the key. Push to mute is the other way round.", showIf: (s) => ["talk", "holdMute"].includes(s.mode), section: CONTROLS },
			{
				...stateTiles("Muted or live", () => "mic", "Live"),
				preview: (v) => glyphKeySvg("micOff", { color: COLORS.red, title: v === "off" ? "" : t("Muted"), titleAt: v === "off" ? "below" : v }),
			},
		],
	},
	[`${ID}.hearmyself`]: switchSchema("headphones", "Hear your changed voice in your headphones."),
	[`${ID}.background`]: switchSchema("background", "Plays the background sounds of a voice, like rain or a crowd, if it has some."),
	[`${ID}.randomvoice`]: stepSchema("random"),
	[`${ID}.nextvoice`]: stepSchema("next"),
	[`${ID}.stopsounds`]: {
		help: "Stops all sounds that play. The key is white while one plays.",
		fields: [],
	},
	[`${ID}.randomsound`]: {
		help: "Plays a random sound, never the same one twice in a row.",
		fields: [
			{
				key: "look",
				label: "Picture",
				type: "tiles",
				section: LOOK,
				options: [
					["dice", "Dice"],
					["next", "Next sound"],
				],
				preview: (v, s, page) => {
					const sound = page.list("sounds")[0];
					if (v === "dice" || !sound) return glyphKeySvg("random");
					return pictureKeySvg({ src: page.pictureUrl(sound.icon), title: s.showName === false ? "" : sound.label, titleAt: s.nameAt });
				},
			},
			{ type: "note", text: "Next sound: the key shows the sound that plays when you press it.", showIf: (s) => s.look === "next", section: LOOK },
			{
				...titleTiles("Name of the sound", "showName", "nameAt", (at, s, page) => {
					const sound = page.list("sounds")[0];
					return pictureKeySvg({ src: sound ? page.pictureUrl(sound.icon) : null, fallback: "sound", title: at ? (sound?.label ?? t("Sound")) : "", titleAt: at || "below" });
				}),
				showIf: (s) => s.look === "next",
			},
		],
	},
	[`${ID}.saysound`]: {
		help: "Hold the key and say a sound, like vine boom. Voicenux finds the one most people like on MyInstants and puts it on this key. Your soundboard stays as it is. Press the key to play it.",
		fields: [
			{
				key: "play",
				label: "Press does",
				type: "select",
				options: [
					["restart", "Play from the start"],
					["toggle", "Play and stop"],
					["overlap", "Play on top"],
				],
				section: CONTROLS,
			},
			{ type: "note", text: "Not the right one? Press twice quickly: the key takes the sound with the next most likes. It does not play it, press again to hear it.", section: CONTROLS },
			LANGUAGE,
			{ type: "note", text: "While you hold the key, others do not hear you. The first time, Voicenux downloads its speech recognition (290 MB).", section: CONTROLS },
			// The sound it found, with the lens. Voicenux keeps it off the soundboard, so it is not in the lists.
			titleTiles("Name", "showName", "nameAt", (at, s, page) => {
				if (!s.sound) return glyphKeySvg("saysound", { title: at ? t("Sound") : "", titleAt: at || "below" });
				const item = page.find("sounds", s.sound) ?? { label: s.soundName || t("Sound"), icon: s.soundIcon };
				return pictureKeySvg({ src: page.pictureUrl(item.icon), fallback: "sound", title: at ? item.label : "", titleAt: at || "below", badge: "saysound" });
			}),
			{ key: "showPlaying", section: LOOK, label: "Frame", type: "checkbox", text: "While the sound plays" },
		],
	},
	[`${ID}.sayvoice`]: {
		help: "Hold the key and say one of your voices, like Darth Vader. It only picks out of the voices you have in Voicenux, nothing is searched online. Press the key to turn the voice on or off.",
		fields: [
			{
				key: "press",
				label: "Press does",
				type: "select",
				options: [
					["toggle", "This voice on or off"],
					["use", "Use this voice"],
				],
				section: CONTROLS,
			},
			LANGUAGE,
			{ type: "note", text: "While you hold the key, others do not hear you. New sounds from the internet: that is Say a Sound.", section: CONTROLS },
			titleTiles("Name", "showName", "nameAt", (at, s, page) => {
				const voice = s.voice ? page.find("voices", s.voice) : null;
				if (!voice) return glyphKeySvg("sayvoice", { title: at ? t("Voice") : "", titleAt: at || "below" });
				return pictureKeySvg({ src: page.pictureUrl(voice.icon), title: at ? voice.label : "", titleAt: at || "below", badge: "saysound" });
			}),
			{ key: "showFrame", section: LOOK, label: "Frame", type: "checkbox", text: "While the voice is in use" },
		],
	},
	[`${ID}.soundvolume`]: {
		help: "How loud the sounds play for everyone: for others in the Voicenux Microphone, and for you when Hear sounds myself is on.",
		fields: [
			...gestures(
				"mode",
				[
					["up", "Louder"],
					["down", "Quieter"],
					["set", "Set to a value"],
				],
				{ repeat: true },
			),
			{ key: "step", section: CONTROLS, label: "Step", type: "number", min: 1, max: 50, step: 1, unit: "%", showIf: (s) => uses(s, "mode", ["up", "down"], "up") },
			{ key: "value", section: CONTROLS, label: "Value", type: "number", min: 0, max: 100, step: 5, unit: "%", showIf: (s) => uses(s, "mode", ["set"], "up") },
			titleTiles("Volume", "showValue", "valueAt", (at, s, page) => glyphKeySvg(VOLUME_GLYPHS[s.mode ?? "up"], { title: at ? `${page.state?.sound_volume ?? 80}%` : "", titleAt: at || "below" })),
		],
	},
	[`${ID}.hearsounds`]: switchSchema("hearsounds", "Hear the sounds yourself too, in your headphones. Others always hear them."),
	[`${ID}.voicedial`]: {
		...DIAL,
		help: "Turn to pick a voice, press to use it. Press while it shows the voice in use to turn the voice changer on or off.",
		fields: [
			{
				key: "list",
				label: "Voices",
				type: "segmented",
				options: [
					["all", "All"],
					["favorites", "Favorites"],
				],
			},
			{ type: "note", text: "Star voices in Voicenux to make them favorites.", showIf: (s) => s.list === "favorites" },
			{
				key: "mode",
				label: "Turning",
				type: "select",
				options: [
					["pick", "Picks a voice, press to use it"],
					["direct", "Uses the voice right away"],
				],
				section: CONTROLS,
			},
			TOUCH,
		],
	},
	[`${ID}.sounddial`]: {
		...DIAL,
		help: "Turn to pick a sound of the soundboard, press or touch to play it.",
		fields: [
			{
				key: "play",
				label: "Press does",
				type: "select",
				options: [
					["restart", "Play from the start"],
					["toggle", "Play and stop"],
				],
				section: CONTROLS,
			},
			TOUCH,
		],
	},
	[`${ID}.soundvolumedial`]: {
		...DIAL,
		help: "Turn for the volume of the soundboard.",
		fields: [
			{ key: "step", label: "Step", type: "number", min: 1, max: 25, step: 1, unit: "%", section: CONTROLS },
			{
				key: "pressAction",
				label: "Press does",
				type: "select",
				options: [
					["stop", "Stop all sounds"],
					["hear", "Hear sounds on or off"],
					["nothing", "Nothing"],
				],
				section: CONTROLS,
			},
			TOUCH,
		],
	},
};
