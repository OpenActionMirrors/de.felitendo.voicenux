// Starts Voicenux, or brings it to the front on a page: `voicenux open settings`.
// Voicenux runs once, so a second start only opens the page in the running app.
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const APP_ID = "io.github.voicenux.Voicenux";

function inPath(name) {
	return (process.env.PATH ?? "")
		.split(":")
		.filter(Boolean)
		.map((dir) => path.join(dir, name))
		.find((file) => {
			try {
				fs.accessSync(file, fs.constants.X_OK);
				return true;
			} catch {
				return false;
			}
		});
}

function hasFlatpak() {
	const where = [path.join(os.homedir(), ".local/share/flatpak/app", APP_ID), path.join("/var/lib/flatpak/app", APP_ID)];
	return !!inPath("flatpak") && where.some((dir) => fs.existsSync(dir));
}

// How to run Voicenux with arguments, or null when it is not installed.
export function voicenuxCommand(args = []) {
	const bin = inPath("voicenux");
	if (bin) return [bin, ...args];
	if (hasFlatpak()) return ["flatpak", "run", APP_ID, ...args];
	return null;
}

export function openVoicenux(page = "settings") {
	const command = voicenuxCommand(["open", page]);
	if (!command) throw new Error("Voicenux is not installed");
	const child = spawn(command[0], command.slice(1), { detached: true, stdio: "ignore" });
	child.on("error", () => {});
	child.unref();
}

export function installed() {
	return !!voicenuxCommand();
}
