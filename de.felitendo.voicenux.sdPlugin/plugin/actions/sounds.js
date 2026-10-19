import { Action } from "./base.js";
import { PictureAction } from "./picture.js";
import { COLORS, VOLUME_GLYPHS } from "../lib/icons.js";
import { t } from "../lib/i18n.js";
import { dialIcon, glyphKey, pictureKey } from "../lib/render.js";

const ID = "de.felitendo.voicenux";

const clamp = (value) => Math.max(0, Math.min(100, Math.round(Number(value) || 0)));

// One sound of the soundboard. The key has a frame while it plays.
export class Sound extends PictureAction {
	static uuid = `${ID}.sound`;
	static required = ["sound"];
	// The play modes of Voicenux, for this key alone: restart (from the start), toggle (play and
	// stop), overlap (another copy on top). stop: stop it.
	static pressKey = "press";
	static ops = ["restart", "toggle", "overlap", "stop"];
	static defaults = { sound: "", press: "restart", hold: "none", double: "none", showName: true, nameAt: "below", showPlaying: true };

	sound() {
		return this.client.sound(this.settings.sound);
	}

	playing() {
		return this.client.playing.includes(this.settings.sound);
	}

	renderOffline() {
		this.glyphForTitle("");
	}

	render() {
		const sound = this.sound();
		const { showName, nameAt, showPlaying } = this.settings;
		if (!sound) {
			const title = showName ? this.settings.sound : "";
			this.ownTitle(title);
			this.setImage(glyphKey("sound", { color: COLORS.gray, title, titleAt: nameAt }));
			return;
		}
		const title = showName ? sound.name : "";
		this.ownTitle(title);
		this.setImage(pictureKey({ png: this.picture(sound.icon), fallback: "sound", title, titleAt: nameAt, frame: showPlaying && this.playing() ? "on" : null }));
	}

	async onPress(args) {
		const sound = this.sound();
		if (!sound) throw new Error(this.settings.sound ? "This sound is gone from Voicenux" : "no sound selected");
		const op = this.op(args);
		if (op === "stop") await this.call("stop-sound", sound.id);
		else await this.playAs(op, sound.id);
	}

	// Voicenux 0.1.0 does not know sound-restart, sound-toggle and sound-overlap yet. There a key
	// plays as the soundboard is set, and only on top can not work.
	async playAs(mode, id) {
		try {
			await this.call(`sound-${mode}`, id);
		} catch (error) {
			if (!/unknown action/.test(error.message)) throw error;
			if (mode === "overlap") throw new Error("Play on top needs a newer Voicenux");
			if (mode === "toggle" && this.playing()) await this.call("stop-sound", id);
			else await this.call("sound", id);
		}
	}
}

// Stops every sound. White while something plays.
export class StopSounds extends Action {
	static uuid = `${ID}.stopsounds`;
	static defaults = {};

	renderOffline() {
		this.setState(0);
	}

	render() {
		this.setState(this.client.playing.length ? 1 : 0);
	}

	async onPress() {
		await this.call("stop-sounds");
	}
}

// Volume of the soundboard: up, down or to a value. Hold keeps going by default.
export class SoundVolume extends Action {
	static uuid = `${ID}.soundvolume`;
	static pressKey = "mode";
	static ops = ["up", "down", "set"];
	static canRepeat = true;
	static defaults = { mode: "up", hold: "repeat", double: "none", step: 10, value: 50, showValue: true, valueAt: "below" };

	glyphName() {
		return VOLUME_GLYPHS[this.gesture("press")] ?? "volume";
	}

	onChange(what) {
		if (what === "state") this.render();
	}

	renderOffline() {
		this.setImage(glyphKey(this.glyphName()));
	}

	render() {
		const { showValue, valueAt } = this.settings;
		const volume = this.vn.sound_volume;
		const title = showValue && volume !== undefined ? `${volume}%` : "";
		this.ownTitle(title);
		this.setImage(glyphKey(this.glyphName(), { title, titleAt: valueAt }));
	}

	async onPress(args) {
		const mode = this.op(args);
		const now = this.vn.sound_volume ?? 0;
		if (mode === "set") {
			if (!args?.repeat) await this.call("sound-volume", clamp(this.settings.value));
		} else {
			const step = Math.abs(Number(this.settings.step) || 1);
			const next = clamp(now + (mode === "down" ? -step : step));
			if (next !== now) await this.call("sound-volume", next);
		}
		this.render();
	}
}

// Stream Deck + dial: turn for the soundboard volume, press or touch to stop all sounds.
export class SoundVolumeDial extends Action {
	static uuid = `${ID}.soundvolumedial`;
	// pressAction: "stop" all sounds, "hear" (hear the sounds yourself, on or off), or "nothing".
	static defaults = { step: 5, pressAction: "stop", disableTouch: false };

	layout() {
		return "layouts/dial.json";
	}

	renderOffline() {
		this.setFeedback({ title: t("Soundboard"), icon: dialIcon("volume"), value: this.dialStatus(), indicator: { value: 0 } });
	}

	render() {
		const volume = this.vn.sound_volume ?? 0;
		const playing = this.client.playing.length > 0;
		this.setFeedback({
			title: t("Soundboard"),
			icon: dialIcon(playing ? "sound" : "volume", { color: playing ? COLORS.cyan : COLORS.white }),
			// With its color: OpenDeck keeps the one of the "Voicenux off" text otherwise.
			value: { value: `${volume}%`, color: COLORS.white },
			indicator: { value: volume, bar_fill_c: COLORS.sky },
		});
	}

	async onDialRotate(ticks) {
		const now = this.vn.sound_volume ?? 0;
		const next = clamp(now + ticks * (Math.abs(Number(this.settings.step)) || 1));
		if (next !== now) await this.call("sound-volume", next);
		this.render();
	}

	async onDialDown() {
		if (this.settings.pressAction === "hear") await this.call("hear-sounds", "toggle");
		else if (this.settings.pressAction !== "nothing") await this.call("stop-sounds");
	}
}
