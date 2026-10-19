// Translations for the settings pages. The code is written in English. t() looks the English text
// up and falls back to it. The German texts are in src/i18n/de.rs, `cargo xtask manifest` copies them
// into shared.js. Keep this file like src/i18n.rs.
import { DE } from "./shared.js";

export const LANGUAGES = { de: DE };

let current = {};

// OpenDeck sends codes like "de" or "zh_CN".
export function setLanguage(code) {
	const lang = String(code ?? "").toLowerCase().split(/[-_]/)[0];
	current = LANGUAGES[lang] ?? {};
	return lang in LANGUAGES ? lang : "en";
}

// t("Connected to Voicenux {version}", { version: "0.1.0" }), translated, with {version} filled in.
export function t(text, vars) {
	const out = current[text] ?? text;
	return vars ? out.replace(/\{(\w+)\}/g, (match, key) => (key in vars ? String(vars[key]) : match)) : out;
}

const patterns = new Map();

// For finished English messages from the plugin, like "Something else uses port 41967":
// finds the text with {placeholders} that fits and translates it with the same values.
export function translateMessage(message) {
	message = String(message ?? "");
	if (message in current) return current[message];
	for (const [text, translated] of Object.entries(current)) {
		if (!text.includes("{")) continue;
		if (!patterns.has(text)) {
			const names = [];
			const source = text.split(/(\{\w+\})/).map((part) => {
				const name = part.match(/^\{(\w+)\}$/)?.[1];
				if (!name) return part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
				names.push(name);
				return "(.+?)";
			});
			patterns.set(text, { regex: new RegExp(`^${source.join("")}$`), names });
		}
		const { regex, names } = patterns.get(text);
		const match = message.match(regex);
		if (match && translated) return t(text, Object.fromEntries(names.map((name, i) => [name, match[i + 1]])));
	}
	return message;
}
