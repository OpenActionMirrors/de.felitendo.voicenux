// What each settings page shows. Field types: select, number, checkbox, segmented, tiles, note.
// section: where a field goes. Without one it is at the top: what the key is about (voice, sound).
// CONTROLS (what press, hold and double press do) and LOOK ("On the key", dials: lookTitle) are open.
// Any other name is a section of its own that is closed until opened.
// tiles: for choices that change how the key looks. Like segmented, with a small picture of the
// key above each choice: preview(value, settings, page) is its SVG. value(settings) and apply(value)
// can map the choice onto more than one setting. page has the lists and pictureUrl().
// The texts are English. i18n.js has the other languages.
import { t } from "../plugin/lib/i18n.js";
import { COLORS, VOLUME_GLYPHS } from "../plugin/lib/icons.js";
import { glyphKeySvg, pictureKeySvg } from "../plugin/lib/render.js";

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

const switchSchema = (glyph, help) => ({
	help,
	fields: [...gestures("mode", TOGGLE), stateTiles("On or off", () => glyph, "On")],
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
			...gestures("press", [
				["use", "Use this voice"],
				["toggle", "Use it, or back to my own voice"],
				["favorite", "Star or unstar it"],
			]),
			titleTiles("Name", "showName", "nameAt", itemKey("voices", "voice", () => null)),
			{ key: "showFrame", section: LOOK, label: "Frame", type: "checkbox", text: "While the voice is in use" },
			{ type: "note", text: "The frame turns gray while the voice changer is off.", section: LOOK, showIf: (s) => s.showFrame !== false },
		],
	},
	[`${ID}.sound`]: {
		fields: [
			{ key: "sound", label: "Sound", type: "select", list: "sounds" },
			...gestures("press", [
				["play", "Play, as the soundboard plays sounds"],
				["toggle", "Play, or stop it when it plays"],
				["stop", "Stop it"],
			]),
			{ type: "note", text: "How the soundboard plays sounds (from the start, play and stop, or on top) is set in Voicenux, under the sounds.", showIf: (s) => uses(s, "press", ["play"], "play"), section: CONTROLS },
			titleTiles("Name", "showName", "nameAt", itemKey("sounds", "sound", () => null)),
			{ key: "showPlaying", section: LOOK, label: "Frame", type: "checkbox", text: "While the sound plays" },
		],
	},
	[`${ID}.voicechanger`]: switchSchema("voicechanger", "Turns the voice changer on or off. When it is off, others hear your own voice."),
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
		help: "Stops every sound of the soundboard. The key is white while a sound plays.",
		fields: [],
	},
	[`${ID}.soundvolume`]: {
		help: "The volume of the soundboard, for everyone who hears the sounds.",
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
	[`${ID}.hearsounds`]: switchSchema("hearsounds", "Hear the sounds of the soundboard yourself. Others hear them either way."),
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
