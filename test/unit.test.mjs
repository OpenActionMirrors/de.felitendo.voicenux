import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

import { ACTIONS } from "../de.felitendo.voicenux.sdPlugin/plugin/actions/index.js";
import { Mute, VoiceChanger } from "../de.felitendo.voicenux.sdPlugin/plugin/actions/switches.js";
import { Sound, SoundVolume } from "../de.felitendo.voicenux.sdPlugin/plugin/actions/sounds.js";
import { NextVoice, RandomVoice, Voice, VoiceDial } from "../de.felitendo.voicenux.sdPlugin/plugin/actions/voices.js";
import { LANGUAGES, setLanguage, t, translateMessage } from "../de.felitendo.voicenux.sdPlugin/plugin/lib/i18n.js";
import { pictureKeySvg, statusKey, titleLine } from "../de.felitendo.voicenux.sdPlugin/plugin/lib/render.js";
import { VoicenuxClient, voicenuxRunning } from "../de.felitendo.voicenux.sdPlugin/plugin/lib/voicenux.js";
import { SCHEMAS } from "../de.felitendo.voicenux.sdPlugin/pi/schemas.js";

const sdPlugin = path.join(import.meta.dirname, "../de.felitendo.voicenux.sdPlugin");
const manifest = JSON.parse(fs.readFileSync(path.join(sdPlugin, "manifest.json"), "utf8"));

const VOICES = [
	{ id: "robot", name: "Robot", icon: "robot", category: "scifi", favorite: false, custom: false },
	{ id: "deep", name: "Deep", icon: "bear", category: "characters", favorite: true, custom: false },
	{ id: "chipmunk", name: "Chipmunk", icon: "chipmunk", category: "funny", favorite: false, custom: false },
];
const SOUNDS = [{ id: "airhorn", name: "Air Horn", icon: "postal-horn", mode: "restart", volume: 100 }];

// A Voicenux that answers like the real API and writes down every call.
class FakeClient extends EventEmitter {
	constructor() {
		super();
		this.status = "connected";
		this.calls = [];
		this.voices = VOICES;
		this.sounds = SOUNDS;
		this.playing = [];
		this.state = { voice: { id: "robot", name: "Robot", icon: "robot", category: "scifi" }, voice_changer: true, hear_myself: false, background: true, mute: false, sound_volume: 80, hear_sounds: true, playing: [], apps: [] };
	}
	get ready() {
		return true;
	}
	voice(id) {
		return this.voices.find((v) => v.id === id) ?? null;
	}
	sound(id) {
		return this.sounds.find((s) => s.id === id) ?? null;
	}
	icon() {
		return Promise.resolve(null);
	}
	async call(action, arg) {
		this.calls.push(arg === undefined ? [action] : [action, String(arg)]);
		const s = this.state;
		const flip = (now) => (arg === "on" ? true : arg === "off" ? false : !now);
		if (action === "voice") Object.assign(s, { voice: this.voice(arg), voice_changer: true });
		else if (action === "voice-changer") s.voice_changer = flip(s.voice_changer);
		else if (action === "mute") s.mute = flip(s.mute);
		else if (action === "sound-volume") s.sound_volume = Number(arg);
		return s;
	}
}

const sent = [];
function key(Class, settings = {}, client = new FakeClient()) {
	const sd = new EventEmitter();
	for (const name of ["log", "warn", "setTitle", "setImage", "setState", "setFeedback", "setFeedbackLayout", "showOk", "showAlert", "setSettings", "sendToPropertyInspector"]) {
		sd[name] = (...args) => sent.push([name, ...args]);
	}
	const plugin = { sd, client, actionInfo: new Map() };
	const action = new Class(plugin, { context: "ctx", action: Class.uuid, payload: { settings } });
	action.appear();
	return { action, client };
}

test("every action has a manifest entry and a settings page", () => {
	const uuids = manifest.Actions.map((a) => a.UUID);
	assert.deepEqual(ACTIONS.map((a) => a.uuid).sort(), [...uuids].sort());
	for (const uuid of uuids) assert.ok(SCHEMAS[uuid], `no settings page for ${uuid}`);
});

// The choices on the settings page must be ones the key knows.
test("press, hold and double press offer only known choices", () => {
	for (const Class of ACTIONS) {
		const fields = SCHEMAS[Class.uuid].fields;
		for (const field of fields.filter((f) => ["hold", "double", Class.pressKey].includes(f.key) && f.options)) {
			for (const [value] of field.options) {
				const known = Class.ops.includes(value) || value === "none" || (value === "repeat" && Class.canRepeat);
				assert.ok(known, `${Class.name}: ${field.key} offers ${value}`);
			}
		}
	}
});

test("gestures fall back to the first choice", () => {
	assert.equal(key(Voice).action.gesture("press"), "toggle");
	assert.equal(key(Voice, { press: "nope" }).action.gesture("press"), "toggle");
	assert.equal(key(Voice, { hold: "favorite" }).action.gesture("hold"), "favorite");
	assert.equal(key(RandomVoice).action.gesture("press"), "random");
	assert.equal(key(SoundVolume).action.gesture("hold"), "repeat");
	// Push to talk uses key up, so it has no hold or double press.
	assert.equal(key(Mute, { mode: "talk", hold: "mute" }).action.gesture("hold"), "none");
});

test("a voice key uses its voice, or goes back to your own voice", async () => {
	const { action, client } = key(Voice, { voice: "deep", press: "toggle" });
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["voice", "deep"]);
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["voice-changer", "off"]);
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["voice", "deep"]);
	await action.onPress({ op: "favorite" });
	assert.deepEqual(client.calls.at(-1), ["favorite", "deep"]);
});

test("a key without its voice or sound says why", async () => {
	await assert.rejects(key(Voice, { voice: "gone" }).action.onPress(), /gone from Voicenux/);
	await assert.rejects(key(Sound, { sound: "" }).action.onPress(), /no sound selected/);
});

test("a sound key plays in its own mode, also with an older Voicenux", async () => {
	const { action, client } = key(Sound, { sound: "airhorn", press: "overlap" });
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["sound-overlap", "airhorn"]);
	await action.onPress({ op: "stop" });
	assert.deepEqual(client.calls.at(-1), ["stop-sound", "airhorn"]);
	// Voicenux 0.1.0 knows only "sound": toggle and restart still work, on top can not.
	const old = key(Sound, { sound: "airhorn", press: "toggle" });
	const call = old.client.call.bind(old.client);
	old.client.call = (name, arg) => (name.startsWith("sound-") ? Promise.reject(new Error(`unknown action "${name}", see API.md`)) : call(name, arg));
	await old.action.onPress();
	assert.deepEqual(old.client.calls.at(-1), ["sound", "airhorn"]);
	old.client.playing = ["airhorn"];
	await old.action.onPress();
	assert.deepEqual(old.client.calls.at(-1), ["stop-sound", "airhorn"]);
	await assert.rejects(old.action.onPress({ op: "overlap" }), /needs a newer Voicenux/);
});

test("soundboard volume stays between 0 and 100", async () => {
	const { action, client } = key(SoundVolume, { step: 15 });
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["sound-volume", "95"]);
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["sound-volume", "100"]);
	const calls = client.calls.length;
	await action.onPress();
	assert.equal(client.calls.length, calls, "no call when it is at 100 already");
	await action.onPress({ op: "set" });
	assert.deepEqual(client.calls.at(-1), ["sound-volume", "50"]);
});

test("switches and mute send on, off or toggle", async () => {
	const changer = key(VoiceChanger, { mode: "off" });
	await changer.action.onPress();
	assert.deepEqual(changer.client.calls.at(-1), ["voice-changer", "off"]);
	const mute = key(Mute, { mode: "talk" });
	await mute.action.onPress();
	assert.deepEqual(mute.client.calls.at(-1), ["mute", "off"]);
	await mute.action.onRelease();
	assert.deepEqual(mute.client.calls.at(-1), ["mute", "on"]);
	await key(NextVoice, { direction: "previous" }).action.onPress();
});

test("push to change voice: on while held, off when let go", async () => {
	const { action, client } = key(VoiceChanger, { mode: "holdOn", hold: "off" });
	client.state.voice_changer = false;
	// It uses key down and up, so there is no hold or double press.
	assert.equal(action.gesture("hold"), "none");
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["voice-changer", "on"]);
	await action.onRelease();
	assert.deepEqual(client.calls.at(-1), ["voice-changer", "off"]);
	const own = key(VoiceChanger, { mode: "holdOff" });
	await own.action.onPress();
	await own.action.onRelease();
	assert.deepEqual(own.client.calls, [["voice-changer", "off"], ["voice-changer", "on"]]);
});

test("a voice used while held gives the voice from before back", async () => {
	const { action, client } = key(Voice, { voice: "deep", press: "whileHeld" });
	client.state.voice_changer = false;
	await action.onPress();
	assert.deepEqual(client.calls.at(-1), ["voice", "deep"]);
	await action.onRelease();
	assert.deepEqual(client.calls.slice(-2), [["voice", "robot"], ["voice-changer", "off"]]);
	// A short press of another kind of voice key leaves the voice as it is.
	const use = key(Voice, { voice: "deep", press: "use" });
	await use.action.onPress();
	await use.action.onRelease();
	assert.deepEqual(use.client.calls, [["voice", "deep"]]);
});

test("the voice dial picks first and uses on press", async () => {
	const { action, client } = key(VoiceDial, { list: "all" });
	await action.onDialRotate(2);
	assert.equal(action.selected, "chipmunk");
	assert.equal(client.calls.length, 0);
	await action.onDialDown();
	assert.deepEqual(client.calls.at(-1), ["voice", "chipmunk"]);
	// Pressed on the voice in use: the voice changer goes off.
	await action.onDialDown();
	assert.deepEqual(client.calls.at(-1), ["voice-changer", "toggle"]);
	// Only favorites: there is just one.
	const favorites = key(VoiceDial, { list: "favorites" });
	await favorites.action.onDialRotate(1);
	assert.equal(favorites.action.selected, "deep");
	action.onDisappear();
	favorites.action.onDisappear();
});

test("the client is connected once the first state is in", () => {
	const client = new VoicenuxClient({ running: () => false });
	const changes = [];
	client.on("change", (what) => changes.push(what));
	client.onEvent("hello", { app: "voicenux", version: "0.1.0", api: 1 });
	assert.equal(client.connected, false);
	client.onEvent("state", { voice: { id: "robot" }, playing: [] });
	assert.equal(client.status, "connected");
	client.onEvent("voices", VOICES);
	client.onEvent("playing", ["airhorn"]);
	assert.deepEqual(client.state.playing, ["airhorn"]);
	assert.equal(client.voice("deep").name, "Deep");
	assert.deepEqual(changes, ["all", "voices", "playing"]);
});

test("finds a running Voicenux by its process name", () => {
	const proc = fs.mkdtempSync(path.join(os.tmpdir(), "proc-"));
	try {
		fs.mkdirSync(path.join(proc, "12"));
		fs.writeFileSync(path.join(proc, "12", "comm"), "bash\n");
		assert.equal(voicenuxRunning(proc), false);
		fs.mkdirSync(path.join(proc, "34"));
		fs.writeFileSync(path.join(proc, "34", "comm"), "voicenux\n");
		assert.equal(voicenuxRunning(proc), true);
	} finally {
		fs.rmSync(proc, { recursive: true });
	}
});

test("pictures get a frame in the colors of Voicenux, gray while it is off", () => {
	const on = pictureKeySvg({ png: "AAAA", title: "Robot", frame: "on" });
	assert.match(on, /linearGradient/);
	assert.match(on, /data:image\/png;base64,AAAA/);
	assert.match(on, />Robot</);
	assert.match(pictureKeySvg({ png: "AAAA", frame: "off" }), /stroke="#6B6B6B"/);
	assert.doesNotMatch(pictureKeySvg({ png: "AAAA" }), /<rect/);
	// Without a picture the glyph stands in.
	assert.doesNotMatch(pictureKeySvg({ png: null, title: "Robot" }), /<image/);
	assert.ok(statusKey("disabled").startsWith("data:image/svg+xml;base64,"));
	assert.ok(titleLine("A very very very long voice name").line.endsWith("…"));
});

// Every text a user can see needs a German version, with the same {placeholders}.
test("every text has a German translation", () => {
	const texts = new Set();
	const add = (value) => typeof value === "string" && value && texts.add(value);
	for (const schema of Object.values(SCHEMAS)) {
		add(schema.help);
		add(schema.lookTitle);
		for (const field of schema.fields ?? []) {
			for (const key of ["label", "text", "hint", "unit"]) add(field[key]);
			// Section titles. "look" is a name in the code, the page shows "On the key" or lookTitle.
			if (field.section !== "look") add(field.section);
			for (const [, label] of field.options ?? []) add(label);
		}
	}
	// t("...") in the code, and the errors it throws: the settings page shows them translated.
	const files = ["pi/pi.js", "pi/schemas.js", ...["plugin/actions", "plugin/lib"].flatMap((dir) => fs.readdirSync(path.join(sdPlugin, dir)).map((f) => `${dir}/${f}`))];
	const errors = new Set();
	for (const file of files) {
		const code = fs.readFileSync(path.join(sdPlugin, file), "utf8");
		for (const [, text] of code.matchAll(/\bt\("((?:[^"\\]|\\.)+)"/g)) add(text);
		// Messages are the texts with a space. ${...} can be any value, like {name} in the table.
		for (const [, args] of code.matchAll(/new Error\((.*)\)/g)) {
			for (const [, plain, template] of args.matchAll(/"((?:[^"\\]|\\.)+)"|`([^`]+)`/g)) {
				const text = plain ?? template.replace(/\$\{[^}]*\}/g, "{x}");
				if (text.includes(" ")) errors.add(text);
			}
		}
		// What the client sets as the reason of a status.
		for (const [, text] of code.matchAll(/setStatus\("\w+", "([^"]+)"/g)) errors.add(text);
		for (const [, text] of code.matchAll(/setStatus\("\w+", `([^`]+)`/g)) errors.add(text.replace(/\$\{[^}]*\}/g, "{x}"));
	}
	const de = LANGUAGES.de;
	const missing = [...texts].filter((text) => !(text in de));
	assert.deepEqual(missing, [], "missing German texts");
	const known = new Set(Object.keys(de).map((text) => text.replace(/\{\w+\}/g, "{x}")));
	assert.deepEqual([...errors].filter((text) => !known.has(text.replace(/\{\w+\}/g, "{x}"))), [], "errors without a German text");
	// A text listed twice would quietly lose its first translation.
	const table = fs.readFileSync(path.join(sdPlugin, "plugin/lib/i18n.js"), "utf8");
	const keys = [...table.matchAll(/^\t("(?:[^"\\]|\\.)*"|'[^']*'|\w+):/gm)].map((m) => m[1].replace(/^["']|["']$/g, ""));
	assert.deepEqual(keys.filter((k, i) => keys.indexOf(k) !== i), [], "texts listed twice");
	const holes = (text) => [...text.matchAll(/\{\w+\}/g)].map((m) => m[0]).sort().join();
	for (const [en, translated] of Object.entries(de)) assert.equal(holes(translated), holes(en), `placeholders differ: ${en}`);
});

test("plugin messages are translated with their values", () => {
	setLanguage("de");
	try {
		assert.equal(translateMessage("Something else uses port 41967"), "Ein anderes Programm belegt Port 41967");
		assert.equal(translateMessage('no voice called "robo"'), "keine Stimme namens „robo“");
		assert.equal(translateMessage("something we do not know"), "something we do not know");
		assert.equal(t("Connected to Voicenux {version}", { version: "0.1.0" }), "Mit Voicenux 0.1.0 verbunden");
	} finally {
		setLanguage("en");
	}
});
