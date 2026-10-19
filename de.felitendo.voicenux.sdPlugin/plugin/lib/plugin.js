// Routes OpenDeck events to the action instances and answers the settings pages.
import fs from "node:fs";
import path from "node:path";

import { installed, openVoicenux } from "./desktop.js";
import { PORT, VoicenuxClient } from "./voicenux.js";

export class Plugin {
	constructor(sd, actionClasses, client = new VoicenuxClient()) {
		this.sd = sd;
		this.classes = new Map(actionClasses.map((c) => [c.uuid, c]));
		this.instances = new Map();
		this.openInspectors = new Set();
		// Icon and state images of every action, by UUID (from manifest.json).
		const manifest = JSON.parse(fs.readFileSync(new URL("../../manifest.json", import.meta.url), "utf8"));
		this.actionInfo = new Map(manifest.Actions.map((a) => [a.UUID, { icon: path.basename(a.Icon), states: (a.States ?? []).map((st) => path.basename(st.Image ?? "")) }]));
		// One connection to Voicenux for all keys.
		this.client = client;
		this.client.on("status", (status, error) => {
			sd.log(`Voicenux ${status}${error ? `: ${error}` : ""}`);
			this.broadcastState();
		});
		// Lists on the settings pages follow Voicenux.
		this.client.on("change", (what) => {
			if (what !== "playing") this.broadcastState();
		});
		this.client.start();

		sd.on("message", (message) => this.onMessage(message));
	}

	instanceFor(message) {
		let instance = this.instances.get(message.context);
		if (!instance) {
			const Class = this.classes.get(message.action);
			if (!Class) return null;
			instance = new Class(this, message.event === "sendToPlugin" ? { ...message, payload: {} } : message);
			this.instances.set(message.context, instance);
			// Multi action children never get willAppear, they only press.
			if (message.event !== "willAppear") instance.attach();
		}
		return instance;
	}

	// Press events carry the settings, which matters for multi action children.
	withSettings(message) {
		const instance = this.instanceFor(message);
		const settings = message.payload?.settings;
		if (instance && settings && JSON.stringify(instance.withDefaults(settings)) !== JSON.stringify(instance.settings)) {
			instance.updateSettings(settings);
		}
		return instance;
	}

	async onMessage(message) {
		const { event, context } = message;
		try {
			switch (event) {
				case "willAppear":
					this.instanceFor(message)?.appear(message);
					break;
				case "willDisappear": {
					const instance = this.instances.get(context);
					instance?.disappear();
					this.instances.delete(context);
					break;
				}
				case "keyDown":
					await this.withSettings(message)?.keyDown(message);
					break;
				case "keyUp":
					await this.withSettings(message)?.keyUp(message);
					break;
				case "dialRotate": {
					const instance = this.withSettings(message);
					if (instance && (await instance.ensureOnline())) await instance.act(() => instance.onDialRotate(message.payload.ticks, message.payload.pressed));
					break;
				}
				case "dialDown":
				case "dialUp":
				case "touchTap": {
					const instance = this.withSettings(message);
					if (!instance || !(await instance.ensureOnline())) break;
					const hook = { dialDown: "onDialDown", dialUp: "onDialUp", touchTap: "onTouchTap" }[event];
					await instance.act(() => instance[hook](message.payload));
					break;
				}
				case "didReceiveSettings":
					this.instanceFor(message)?.updateSettings(message.payload?.settings);
					break;
				case "propertyInspectorDidAppear":
					this.openInspectors.add(context);
					this.sendState(context);
					break;
				case "propertyInspectorDidDisappear":
					this.openInspectors.delete(context);
					break;
				case "sendToPlugin":
					await this.onInspectorMessage(message);
					break;
				case "systemDidWakeUp":
					this.client.reconnect();
					break;
			}
		} catch (error) {
			this.sd.warn(`${event}: ${error?.stack ?? error}`);
		}
	}

	// State shown at the top of every settings page, with the lists for the drop downs.
	stateFor(context) {
		const instance = this.instances.get(context);
		const client = this.client;
		return {
			type: "state",
			connection: {
				status: client.status,
				error: client.lastError,
				version: client.version,
				port: PORT,
				installed: installed(),
			},
			// The voice in use and the soundboard volume, for the pictures of the key.
			voicenux: client.state ? { voice: client.state.voice, sound_volume: client.state.sound_volume } : null,
			lists: {
				voices: client.voices.map((v) => ({ value: v.id, label: v.name, icon: v.icon, favorite: v.favorite })),
				sounds: client.sounds.map((s) => ({ value: s.id, label: s.name, icon: s.icon })),
			},
			settings: instance?.settings ?? null,
			keyError: instance?.lastError ?? null,
		};
	}

	// Something on the settings page of this key changed, like its last error.
	stateChanged(context) {
		if (this.openInspectors.has(context)) this.sendState(context);
	}

	sendState(context) {
		this.sd.sendToPropertyInspector(context, this.stateFor(context));
	}

	broadcastState() {
		for (const context of this.openInspectors) this.sendState(context);
	}

	async onInspectorMessage(message) {
		const { context, payload = {} } = message;
		const instance = this.instanceFor(message);
		const reply = (data) => this.sd.sendToPropertyInspector(context, { ...data, requestId: payload.requestId });
		this.openInspectors.add(context);

		switch (payload.type) {
			case "hello":
				this.sendState(context);
				break;
			// Starts Voicenux on its settings, where "Let other apps control Voicenux" is.
			case "openVoicenux":
				try {
					openVoicenux(payload.page ?? "settings");
					reply({ type: "opened" });
				} catch (error) {
					reply({ type: "opened", error: error.message });
				}
				break;
			case "retry":
				this.client.reconnect();
				break;
			case "openUrl":
				if (/^https?:\/\//.test(payload.url ?? "")) this.sd.openUrl(payload.url);
				break;
			default:
				if (!instance) break;
				try {
					await instance.onPiMessage(payload, reply);
				} catch (error) {
					instance.log(`settings page: ${error.message}`);
					reply({ type: "error", for: payload.type, message: error.message });
				}
		}
	}
}
