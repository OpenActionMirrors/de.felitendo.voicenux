import { PictureAction } from "./picture.js";
import { COLORS } from "../lib/icons.js";
import { t } from "../lib/i18n.js";
import { dialPicture, glyphKey, pictureKey } from "../lib/render.js";

const ID = "de.felitendo.voicenux";
// How long a dial shows a picked voice before it goes back to the one in use.
const PICK_TIMEOUT = 6000;

// One voice. Press uses it, and the key has a frame while it is in use.
export class Voice extends PictureAction {
	static uuid = `${ID}.voice`;
	static required = ["voice"];
	// use: this voice. toggle: this voice, or back to your own when it is in use. favorite: star it.
	// whileHeld: this voice while the key is held, then back to the voice from before.
	static pressKey = "press";
	static ops = ["use", "toggle", "favorite", "whileHeld"];
	static pressOnly = ["whileHeld"];
	static defaults = { voice: "", press: "use", hold: "none", double: "none", showName: true, nameAt: "below", showFrame: true };

	voice() {
		return this.client.voice(this.settings.voice);
	}

	renderOffline() {
		this.glyphForTitle("");
	}

	render() {
		const voice = this.voice();
		const { showName, nameAt, showFrame } = this.settings;
		if (!voice) {
			// Deleted in Voicenux, or not loaded yet.
			const title = showName ? this.settings.voice : "";
			this.ownTitle(title);
			this.setImage(glyphKey("voice", { color: COLORS.gray, title, titleAt: nameAt }));
			return;
		}
		const inUse = this.vn.voice?.id === voice.id;
		const frame = showFrame && inUse ? (this.vn.voice_changer ? "on" : "off") : null;
		const title = showName ? voice.name : "";
		this.ownTitle(title);
		this.setImage(pictureKey({ png: this.picture(voice.icon), title, titleAt: nameAt, frame }));
	}

	async onPress(args) {
		const voice = this.voice();
		if (!voice) throw new Error(this.settings.voice ? "This voice is gone from Voicenux" : "no voice selected");
		const op = this.op(args);
		if (op === "whileHeld" && !this.isInMultiAction) this.before = { id: this.vn.voice?.id, on: !!this.vn.voice_changer };
		if (op === "favorite") {
			await this.call("favorite", voice.id);
			this.showOk();
		} else if (op === "toggle" && this.vn.voice?.id === voice.id && this.vn.voice_changer && !this.isInMultiAction) {
			await this.call("voice-changer", "off");
		} else {
			await this.call("voice", voice.id);
		}
		this.render();
	}

	async onRelease() {
		const before = this.before;
		this.before = null;
		if (!before || this.gesture("press") !== "whileHeld") return;
		if (before.id && before.id !== this.vn.voice?.id) await this.call("voice", before.id);
		if (!before.on) await this.call("voice-changer", "off");
		this.render();
	}
}

const STEP_GLYPHS = { next: "next", previous: "previous", random: "random" };

// Next, previous or a random voice. Can show the voice in use.
export class NextVoice extends PictureAction {
	static uuid = `${ID}.nextvoice`;
	static pressKey = "direction";
	static ops = ["next", "previous", "random"];
	static defaults = { direction: "next", hold: "none", double: "none", look: "icon", showName: true, nameAt: "below" };

	glyphName() {
		return STEP_GLYPHS[this.gesture("press")] ?? "next";
	}

	renderOffline() {
		this.setImage(glyphKey(this.glyphName()));
	}

	render() {
		const voice = this.vn.voice;
		const { look, showName, nameAt } = this.settings;
		const title = showName && voice ? voice.name : "";
		const off = !this.vn.voice_changer;
		this.ownTitle(title);
		if (look === "voice" && voice) {
			this.setImage(pictureKey({ png: this.picture(voice.icon), title, titleAt: nameAt, dim: off }));
		} else {
			this.setImage(glyphKey(this.glyphName(), { title, titleAt: nameAt }));
		}
	}

	async onPress(args) {
		const op = this.op(args);
		await this.call(op === "random" ? "random-voice" : op === "previous" ? "previous-voice" : "next-voice");
		this.render();
	}
}

export class RandomVoice extends NextVoice {
	static uuid = `${ID}.randomvoice`;
	static pressKey = "direction";
	static ops = ["random", "next", "previous"];
	static defaults = { ...NextVoice.defaults, direction: "random" };
}

// Stream Deck + dial: turn to pick a voice (with its picture), press to use it.
// Pressed while it shows the voice in use, it turns the voice changer on or off.
export class VoiceDial extends PictureAction {
	static uuid = `${ID}.voicedial`;
	// list: "all" voices or only "favorites". mode: "pick" (press to use) or "direct" (turning uses it).
	static defaults = { list: "all", mode: "pick", disableTouch: false };

	layout() {
		return "layouts/voice-dial.json";
	}

	// The voices to turn through. Favorites only when there are some.
	list() {
		const all = this.client.voices;
		if (this.settings.list === "favorites") {
			const favorites = all.filter((v) => v.favorite);
			if (favorites.length) return favorites;
		}
		return all;
	}

	target() {
		return (this.selected && this.client.voice(this.selected)) || this.vn.voice || null;
	}

	onChange(what) {
		if (what === "voices" && this.selected && !this.client.voice(this.selected)) this.selected = null;
		this.render();
	}

	renderOffline() {
		this.setFeedback({ title: t("Voice"), name: "", info: this.dialStatus(), picture: dialPicture(null) });
	}

	render() {
		const voice = this.target();
		if (!voice) return this.renderOffline();
		const list = this.list();
		const inUse = voice.id === this.vn.voice?.id;
		const on = !!this.vn.voice_changer;
		const index = list.findIndex((v) => v.id === voice.id);
		let info = index >= 0 ? `${index + 1} / ${list.length}` : "";
		if (inUse) info = t(on ? "In use" : "Voice changer off");
		this.setFeedback({
			title: t(this.selected && !inUse ? "Press to use" : "Voice"),
			name: { value: voice.name ?? "", color: inUse && !on ? COLORS.muted : COLORS.white },
			info: { value: info, color: inUse && on ? COLORS.cyan : COLORS.muted },
			picture: dialPicture(this.picture(voice.icon)),
		});
	}

	async onDialRotate(ticks) {
		const list = this.list();
		if (!list.length) return;
		const from = list.findIndex((v) => v.id === this.target()?.id);
		const next = list[(((Math.max(0, from) + ticks) % list.length) + list.length) % list.length];
		clearTimeout(this.pickTimer);
		this.selected = next.id;
		this.render();
		if (this.settings.mode === "direct") {
			// Wait a moment, so a fast turn does not switch through every voice.
			clearTimeout(this.switchTimer);
			this.switchTimer = setTimeout(() => this.act(() => this.use(next.id)), 250);
			return;
		}
		this.pickTimer = setTimeout(() => {
			this.selected = null;
			this.render();
		}, PICK_TIMEOUT);
	}

	async use(id) {
		await this.call("voice", id);
		clearTimeout(this.pickTimer);
		this.selected = null;
		this.render();
	}

	async onDialDown() {
		const voice = this.target();
		if (!voice) return;
		if (voice.id === this.vn.voice?.id) {
			clearTimeout(this.pickTimer);
			this.selected = null;
			await this.call("voice-changer", "toggle");
			this.render();
		} else await this.use(voice.id);
	}

	onDisappear() {
		clearTimeout(this.pickTimer);
		clearTimeout(this.switchTimer);
	}
}
