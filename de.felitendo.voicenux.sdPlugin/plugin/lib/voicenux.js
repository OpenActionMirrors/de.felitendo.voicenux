// Client for the Voicenux API (see API.md in the Voicenux repo). Uses the WebSocket built into Node 22+.
// One connection for all keys. It keeps the state, the voices, the sounds and what plays, and tells
// the keys about every change.
import { EventEmitter } from "node:events";
import fs from "node:fs";

export const PORT = 41967;
// The API version this plugin speaks. Voicenux raises it when a change breaks clients.
export const API = 1;

const RETRY_DELAYS = [1000, 2000, 3000, 5000];
const CALL_TIMEOUT = 5000;

// Is Voicenux running at all? Then a refused connection means the API is off in its settings.
// Also finds the Flatpak, its processes show up here too.
export function voicenuxRunning(proc = "/proc") {
	let dirs;
	try {
		dirs = fs.readdirSync(proc);
	} catch {
		return false;
	}
	for (const pid of dirs) {
		if (!/^\d+$/.test(pid)) continue;
		try {
			if (fs.readFileSync(`${proc}/${pid}/comm`, "utf8").trim() === "voicenux") return true;
		} catch {}
	}
	return false;
}

/**
 * Status values:
 *   idle        not started
 *   connecting  waiting for the socket
 *   connected   Voicenux said hello, actions work
 *   offline     Voicenux does not run
 *   disabled    Voicenux runs, but "Let other apps control Voicenux" is off
 *   error       anything else, see lastError
 */
export class VoicenuxClient extends EventEmitter {
	constructor({ port = PORT, host = "127.0.0.1", running = voicenuxRunning } = {}) {
		super();
		this.setMaxListeners(0);
		this.port = port;
		this.host = host;
		this.running = running;
		this.status = "idle";
		this.lastError = "";
		this.version = null;
		this.ws = null;
		this.pending = new Map();
		this.nextId = 1;
		this.retry = 0;
		this.retryTimer = null;
		this.stopped = true;
		this.reset();
	}

	reset() {
		this.state = null;
		this.voices = [];
		this.sounds = [];
		this.playing = [];
		// Pictures by name, as promises of base64 PNG. They never change while Voicenux runs.
		this.icons = new Map();
	}

	get connected() {
		return this.status === "connected";
	}

	// Ready for keys: connected and the first state is in.
	get ready() {
		return this.connected && !!this.state;
	}

	start() {
		if (!this.stopped) return;
		this.stopped = false;
		this.open();
	}

	stop() {
		this.stopped = true;
		clearTimeout(this.retryTimer);
		this.drop("stopped");
		this.setStatus("idle");
	}

	// Try again now, for example after the user turned the API on.
	reconnect() {
		clearTimeout(this.retryTimer);
		this.retry = 0;
		if (this.ws) this.drop("reconnect");
		if (!this.stopped) this.open();
	}

	drop(reason) {
		const ws = this.ws;
		this.ws = null;
		if (ws) {
			try {
				ws.close(1000);
			} catch {}
		}
		this.cleanup(reason);
	}

	setStatus(status, error = "") {
		const changed = status !== this.status || error !== this.lastError;
		this.status = status;
		this.lastError = error;
		if (changed) this.emit("status", status, error);
	}

	open() {
		if (this.stopped) return;
		// While retrying, keep showing why the last try failed.
		if (!["offline", "disabled", "error"].includes(this.status)) this.setStatus("connecting");
		let ws;
		try {
			ws = new WebSocket(`ws://${this.host}:${this.port}`);
		} catch (error) {
			this.setStatus("error", error.message);
			this.scheduleRetry();
			return;
		}
		this.ws = ws;
		let greeted = false;

		ws.addEventListener("message", (event) => {
			let message;
			try {
				message = JSON.parse(event.data);
			} catch {
				return;
			}
			if (message.event) {
				if (message.event === "hello") {
					greeted = true;
					this.retry = 0;
					this.version = message.data?.version ?? null;
					this.api = message.data?.api ?? null;
				}
				this.onEvent(message.event, message.data);
			} else if (message.id !== undefined && this.pending.has(message.id)) {
				const pending = this.pending.get(message.id);
				this.pending.delete(message.id);
				clearTimeout(pending.timer);
				if (message.ok) pending.resolve(message.result);
				else pending.reject(new Error(message.error || "Voicenux said no"));
			}
		});

		const closed = () => {
			if (this.ws !== ws) return;
			this.ws = null;
			const wasConnected = this.connected;
			this.cleanup("connection closed");
			this.reset();
			if (wasConnected) this.emit("change", "all");
			if (this.running()) {
				this.setStatus("disabled", wasConnected ? "Voicenux closed the connection" : "Other apps may not control Voicenux");
			} else {
				this.setStatus("offline", "Voicenux is not running");
			}
			this.scheduleRetry();
		};
		ws.addEventListener("close", closed);
		ws.addEventListener("error", () => {
			// Some Node versions never send "close" when the socket did not open, so the try would hang.
			if (ws.readyState === WebSocket.CONNECTING) {
				closed();
				try {
					ws.close();
				} catch {}
			}
		});
		// The hello comes right away. Without it, something else listens on the port.
		setTimeout(() => {
			if (this.ws === ws && !greeted) {
				this.drop("no hello");
				this.setStatus("error", `Something else uses port ${this.port}`);
				this.scheduleRetry();
			}
		}, 3000);
	}

	onEvent(event, data) {
		switch (event) {
			case "hello":
				// "connected" waits for the first state, so keys draw with data.
				return;
			case "state":
				this.state = data;
				this.playing = data?.playing ?? this.playing;
				if (!this.connected) {
					if (this.api !== null && this.api > API) {
						this.setStatus("connected", `Voicenux speaks API ${this.api}, this plugin ${API}. Update the plugin.`);
					} else this.setStatus("connected");
					this.emit("change", "all");
				} else this.emit("change", "state");
				return;
			case "voices":
				this.voices = Array.isArray(data) ? data : [];
				if (this.connected) this.emit("change", "voices");
				return;
			case "sounds":
				this.sounds = Array.isArray(data) ? data : [];
				if (this.connected) this.emit("change", "sounds");
				return;
			case "playing":
				this.playing = Array.isArray(data) ? data : [];
				if (this.state) this.state = { ...this.state, playing: this.playing };
				if (this.connected) this.emit("change", "playing");
				return;
		}
	}

	scheduleRetry() {
		if (this.stopped) return;
		clearTimeout(this.retryTimer);
		const delay = RETRY_DELAYS[Math.min(this.retry, RETRY_DELAYS.length - 1)];
		this.retry++;
		this.retryTimer = setTimeout(() => this.open(), delay);
	}

	cleanup(reason) {
		for (const [, pending] of this.pending) {
			clearTimeout(pending.timer);
			pending.reject(new Error(`Voicenux not connected (${reason})`));
		}
		this.pending.clear();
	}

	// An action from API.md. Actions that change something answer with the new state.
	call(action, arg) {
		return new Promise((resolve, reject) => {
			if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
				reject(new Error("Voicenux not connected"));
				return;
			}
			const id = this.nextId++;
			const timer = setTimeout(() => {
				this.pending.delete(id);
				reject(new Error("Voicenux did not answer in time"));
			}, CALL_TIMEOUT);
			this.pending.set(id, { resolve, reject, timer });
			const message = { id, action };
			if (arg !== undefined && arg !== null) message.arg = String(arg);
			this.ws.send(JSON.stringify(message));
		}).then((result) => {
			// The new state comes as an event too, but this way the key sees it before it draws.
			if (result && typeof result === "object" && "voice_changer" in result) this.onEvent("state", result);
			return result;
		});
	}

	// The picture of a voice or sound as base64 PNG, or null.
	icon(name) {
		if (!name) return Promise.resolve(null);
		if (!this.icons.has(name)) {
			const promise = this.call("icon", name)
				.then((result) => result?.png ?? null)
				.catch(() => {
					this.icons.delete(name);
					return null;
				});
			this.icons.set(name, promise);
		}
		return this.icons.get(name);
	}

	voice(id) {
		return this.voices.find((v) => v.id === id) ?? null;
	}

	sound(id) {
		return this.sounds.find((s) => s.id === id) ?? null;
	}
}
