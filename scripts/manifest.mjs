// Builds manifest.json and the translated names (de.json). Edit the tables below, then run: node scripts/manifest.mjs
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const version = fs.readFileSync(path.join(root, "VERSION"), "utf8").trim();
const ID = "de.felitendo.voicenux";

const state = (image, extra = {}) => ({ Image: `images/keys/${image}`, TitleAlignment: "bottom", FontSize: 11, ...extra });

const dial = (icon, layout, trigger) => ({
	Controllers: ["Encoder"],
	Encoder: { layout, Icon: `images/encoder/${icon}`, TriggerDescription: trigger },
});

// [uuid, name, icon, tooltip, states, extra]. OpenDeck lists the actions in this order:
// the ones people use most come first.
const actions = [
	["voice", "Voice", "voice", "Turn a voice on or off, or use it only while you hold the key. Shows its picture, with a frame while it is in use.", [state("voice")]],
	["sound", "Sound", "sound", "Play a sound of the soundboard. Shows its picture, with a frame while it plays.", [state("sound")]],
	["voicechanger", "Voice Changer", "voicechanger", "Turn the voice changer on or off, or push to change your voice.", [state("voicechanger"), state("voicechanger-on")]],
	["mute", "Mute", "mic", "Mute or unmute your microphone, or push to talk.", [state("unmuted"), state("muted")]],
	["hearmyself", "Hear Myself", "headphones", "Hear your changed voice in your headphones, on or off.", [state("hearmyself"), state("hearmyself-on")]],
	["randomvoice", "Random Voice", "random", "Use a random voice, out of your favorites when you have two or more.", [state("random")]],
	["nextvoice", "Next Voice", "next", "Use the next or previous voice. Can show the voice in use.", [state("next")]],
	["stopsounds", "Stop Sounds", "stop", "Stop every sound of the soundboard.", [state("stopsounds"), state("stopsounds-on")]],
	["background", "Background", "background", "Background sounds of the voice, like rain or a crowd, on or off.", [state("background"), state("background-on")]],
	["soundvolume", "Soundboard Volume", "volumeUp", "Turn the soundboard louder, quieter or to a set value. Hold the key to keep going.", [state("soundvolume")]],
	["hearsounds", "Hear Sounds", "hearsounds", "Hear the sounds of the soundboard yourself, on or off.", [state("hearsounds"), state("hearsounds-on")]],
	["voicedial", "Voice (Dial)", "voice", "Turn the dial to pick a voice (with its picture), press to use it.", [{ Image: "images/encoder/voice-key" }], dial("voice", "layouts/voice-dial.json", { Rotate: "Pick a voice", Push: "Use it", Touch: "Use it" })],
	["soundvolumedial", "Soundboard Volume (Dial)", "volume", "Turn the dial for the soundboard volume, press to stop all sounds.", [{ Image: "images/encoder/volume-key" }], dial("volume", "layouts/dial.json", { Rotate: "Volume", Push: "Stop all sounds", Touch: "Stop all sounds" })],
];

// OpenDeck shows these names and tooltips when its language is German.
const de = {
	voice: ["Stimme", "Eine Stimme an- oder ausschalten, oder nur nehmen, solange du die Taste hältst. Zeigt ihr Bild, mit Rahmen, solange sie aktiv ist."],
	sound: ["Sound", "Einen Sound vom Soundboard abspielen. Zeigt sein Bild, mit Rahmen, solange er läuft."],
	voicechanger: ["Stimmverzerrer", "Den Stimmverzerrer an- oder ausschalten, oder zum Verzerren drücken."],
	mute: ["Stummschalten", "Dein Mikrofon stumm- oder lautschalten, oder zum Sprechen drücken."],
	hearmyself: ["Mich hören", "Deine verzerrte Stimme im Kopfhörer hören, an oder aus."],
	randomvoice: ["Zufällige Stimme", "Eine zufällige Stimme nehmen, aus deinen Favoriten, sobald es mindestens zwei sind."],
	nextvoice: ["Nächste Stimme", "Die nächste oder vorherige Stimme nehmen. Kann die aktive Stimme zeigen."],
	stopsounds: ["Sounds stoppen", "Alle Sounds vom Soundboard stoppen."],
	background: ["Hintergrund", "Hintergrundgeräusche der Stimme, etwa Regen oder eine Menge, an oder aus."],
	soundvolume: ["Soundboard-Lautstärke", "Das Soundboard lauter, leiser oder auf einen Wert stellen. Halten macht weiter."],
	hearsounds: ["Sounds hören", "Die Sounds vom Soundboard selbst hören, an oder aus."],
	voicedial: ["Stimme (Drehregler)", "Drehen wählt eine Stimme (mit Bild), Drücken nimmt sie."],
	soundvolumedial: ["Soundboard-Lautstärke (Drehregler)", "Drehen ändert die Lautstärke des Soundboards, Drücken stoppt alle Sounds."],
};
const missing = actions.map(([uuid]) => uuid).filter((uuid) => !de[uuid]);
if (missing.length) throw new Error(`no German name for: ${missing.join(", ")}`);

const manifest = {
	$schema: "https://schemas.elgato.com/streamdeck/plugins/manifest.json",
	UUID: ID,
	Name: "Voicenux",
	Version: version,
	Author: "Felitendo",
	Description: "Control the Voicenux voice changer from OpenDeck: voices with their pictures, the soundboard, mute, push to talk and push to change your voice.",
	URL: "https://github.com/FeloDeck/voicenux-plugin",
	Icon: "images/plugin",
	Category: "Voicenux",
	CategoryIcon: "images/category",
	CodePath: "plugin/main.js",
	Nodejs: { Version: "22" },
	PropertyInspectorPath: "pi/index.html",
	SDKVersion: 2,
	OS: [{ Platform: "linux" }],
	Actions: actions.map(([uuid, name, icon, tooltip, states, extra = {}]) => ({
		UUID: `${ID}.${uuid}`,
		Name: name,
		Icon: `images/actions/${icon}`,
		Tooltip: tooltip,
		Controllers: ["Keypad"],
		SupportedInMultiActions: true,
		DisableAutomaticStates: true,
		States: states,
		...extra,
	})),
};

const localized = {
	Name: "Voicenux",
	Description: "Steuere den Stimmverzerrer Voicenux mit OpenDeck: Stimmen mit Bild, das Soundboard, Stummschalten, zum Sprechen und zum Verzerren drücken.",
	...Object.fromEntries(actions.map(([uuid]) => [`${ID}.${uuid}`, { Name: de[uuid][0], Tooltip: de[uuid][1] }])),
};

const dir = path.join(root, "de.felitendo.voicenux.sdPlugin");
fs.writeFileSync(path.join(dir, "manifest.json"), JSON.stringify(manifest, null, "\t") + "\n");
fs.writeFileSync(path.join(dir, "de.json"), JSON.stringify(localized, null, "\t") + "\n");
console.log(`manifest.json and de.json: ${manifest.Actions.length} actions, version ${version}`);
