import { Action } from "./base.js";
import { t } from "../lib/i18n.js";

const ID = "de.felitendo.voicenux";

// A switch of Voicenux: on, off or toggle. White when on, gray when off.
// holdOn: on only while the key is held, like push to talk. holdOff: off only while held.
// field: its name in the state, api: its action in the API.
class Switch extends Action {
	static field = "";
	static api = "";
	static pressKey = "mode";
	static ops = ["toggle", "on", "off", "holdOn", "holdOff"];
	static pressOnly = ["holdOn", "holdOff"];
	static defaults = { mode: "toggle", hold: "none", double: "none", showState: false, stateAt: "below" };

	isOn() {
		return !!this.vn[this.constructor.field];
	}

	renderOffline() {
		this.glyphForTitle("");
	}

	render() {
		const on = this.isOn();
		this.setState(on ? 1 : 0);
		const { showState, stateAt } = this.settings;
		this.glyphForTitle(showState ? t(on ? "On" : "Off") : "", stateAt);
	}

	async onPress(args) {
		const op = this.op(args);
		await this.call(this.constructor.api, op === "holdOn" ? "on" : op === "holdOff" ? "off" : op);
		this.render();
	}

	async onRelease() {
		if (this.isInMultiAction) return;
		const mode = this.gesture("press");
		if (mode !== "holdOn" && mode !== "holdOff") return;
		await this.call(this.constructor.api, mode === "holdOn" ? "off" : "on");
		this.render();
	}
}

export class VoiceChanger extends Switch {
	static uuid = `${ID}.voicechanger`;
	static field = "voice_changer";
	static api = "voice-changer";
}

export class HearMyself extends Switch {
	static uuid = `${ID}.hearmyself`;
	static field = "hear_myself";
	static api = "hear-myself";
}

export class Background extends Switch {
	static uuid = `${ID}.background`;
	static field = "background";
	static api = "background";
}

export class HearSounds extends Switch {
	static uuid = `${ID}.hearsounds`;
	static field = "hear_sounds";
	static api = "hear-sounds";
}

// Back to your own voice: turns the voice changer off, whatever voice is on. White while a voice
// is on, gray when you already speak with your own voice.
export class OwnVoice extends Action {
	static uuid = `${ID}.ownvoice`;
	static defaults = {};

	renderOffline() {
		this.setState(0);
	}

	render() {
		this.setState(this.vn.voice_changer ? 1 : 0);
	}

	async onPress() {
		await this.call("voice-changer", "off");
		this.render();
	}
}

// Mutes the microphone. Red when muted. talk: on only while the key is held (push to talk).
// holdMute: muted only while held.
export class Mute extends Action {
	static uuid = `${ID}.mute`;
	static pressKey = "mode";
	static ops = ["toggle", "mute", "unmute", "talk", "holdMute"];
	static pressOnly = ["talk", "holdMute"];
	static defaults = { mode: "toggle", hold: "none", double: "none", showState: false, stateAt: "below" };

	renderOffline() {
		this.glyphForTitle("");
	}

	render() {
		const muted = !!this.vn.mute;
		this.setState(muted ? 1 : 0);
		const { showState, stateAt } = this.settings;
		this.glyphForTitle(showState ? t(muted ? "Muted" : "Live") : "", stateAt);
	}

	async setMuted(muted) {
		await this.call("mute", muted ? "on" : "off");
		this.render();
	}

	async onPress(args) {
		const mode = this.op(args);
		if (mode === "toggle") await this.call("mute", "toggle");
		else if (mode === "talk") await this.setMuted(false);
		else if (mode === "holdMute") await this.setMuted(true);
		else await this.setMuted(mode === "mute");
		this.render();
	}

	async onRelease() {
		if (this.isInMultiAction) return;
		const mode = this.gesture("press");
		if (mode === "talk") await this.setMuted(true);
		else if (mode === "holdMute") await this.setMuted(false);
	}
}
