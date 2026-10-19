// Translations for the keys, the settings pages and the messages shown there.
// The code is written in English. t() looks the English text up here and falls back to it.
// The settings page imports this file too, so keep it free of Node imports.
// Action names for the OpenDeck list are in scripts/manifest.mjs.
// German words follow Voicenux: Stimmverzerrer, Mich hören, Hintergrund, Sounds.

const DE = {
	// ----- keys and dials -----
	"Voicenux off": "Voicenux aus",
	"API off": "API aus",
	"Voicenux error": "Voicenux-Fehler",
	On: "An",
	Off: "Aus",
	Muted: "Stumm",
	Live: "Live",
	Voice: "Stimme",
	Sound: "Sound",
	"In use": "Aktiv",
	"Voice changer off": "Verzerrer aus",
	"Press to use": "Drücken nimmt sie",
	Soundboard: "Soundboard",

	// ----- connection -----
	"Voicenux is not running": "Voicenux läuft nicht",
	"Other apps may not control Voicenux": "Andere Apps dürfen Voicenux nicht steuern",
	"Voicenux closed the connection": "Voicenux hat die Verbindung geschlossen",
	"Something else uses port {port}": "Ein anderes Programm belegt Port {port}",
	"Voicenux speaks API {api}, this plugin {plugin}. Update the plugin.": "Voicenux spricht API {api}, dieses Plugin {plugin}. Aktualisiere das Plugin.",
	"Voicenux is not installed": "Voicenux ist nicht installiert",

	// ----- why a key showed its alert (errors from the actions and from Voicenux) -----
	"Last error: {error}": "Letzter Fehler: {error}",
	"no voice selected": "keine Stimme gewählt",
	"no sound selected": "kein Sound gewählt",
	"This voice is gone from Voicenux": "Diese Stimme gibt es in Voicenux nicht mehr",
	"This sound is gone from Voicenux": "Diesen Sound gibt es in Voicenux nicht mehr",
	"Voicenux not connected": "Voicenux nicht verbunden",
	"Voicenux not connected ({reason})": "Voicenux nicht verbunden ({reason})",
	"Voicenux did not answer in time": "Voicenux hat nicht rechtzeitig geantwortet",
	"Voicenux said no": "Voicenux hat abgelehnt",
	"Voicenux is closing": "Voicenux wird beendet",
	"no answer": "keine Antwort",
	'no voice called "{name}"': "keine Stimme namens „{name}“",
	'no sound called "{name}"': "kein Sound namens „{name}“",

	// ----- settings page: status and help -----
	"Checking connection to Voicenux…": "Prüfe die Verbindung zu Voicenux …",
	"Connected to Voicenux {version}": "Mit Voicenux {version} verbunden",
	"Connecting to Voicenux…": "Verbinde mit Voicenux …",
	"Voicenux does not let other apps in": "Voicenux lässt keine anderen Apps zu",
	"Turn it on in the settings of Voicenux.": "Schalte das in den Einstellungen von Voicenux an.",
	"Start Voicenux, the keys connect by themselves.": "Starte Voicenux, die Tasten verbinden sich dann von selbst.",
	"Install Voicenux first.": "Installiere zuerst Voicenux.",
	"This plugin controls the Voicenux voice changer. Install it, then start it once.": "Dieses Plugin steuert den Stimmverzerrer Voicenux. Installiere ihn und starte ihn einmal.",
	"How to install": "So installierst du es",
	"How to connect Voicenux": "So verbindest du Voicenux",
	"Click **Open Voicenux settings** below. It starts Voicenux if it is not running.": "Klicke unten auf **Voicenux-Einstellungen öffnen**. Das startet Voicenux, falls es nicht läuft.",
	"Under **Other apps**, turn on **Let other apps control Voicenux**.": "Schalte unter **Andere Apps** den Punkt **Andere Apps dürfen Voicenux steuern** an.",
	"The status at the top turns green as soon as the connection works.": "Der Status oben wird grün, sobald die Verbindung steht.",
	"Open Voicenux settings": "Voicenux-Einstellungen öffnen",
	"Try again": "Nochmal versuchen",
	"Voicenux opens on your desktop…": "Voicenux öffnet sich auf deinem Desktop …",

	// ----- settings page: controls -----
	"Choose…": "Auswählen …",
	"Search…": "Suchen …",
	"Nothing found": "Nichts gefunden",
	Favorite: "Favorit",
	Less: "Weniger",
	More: "Mehr",
	"{value} (not found)": "{value} (nicht gefunden)",
	Controls: "Bedienung",
	"On the key": "Auf der Taste",
	"On the display": "Auf dem Display",
	"More options": "Weitere Optionen",
	"%": "%",

	// ----- settings page: fields (pi/schemas.js) -----
	"Press does": "Drücken",
	"Hold does": "Halten",
	"Double press": "Doppelt drücken",
	Nothing: "Nichts",
	"Keep going while held": "Weitermachen, solange gehalten",
	"With a double press set, a single press waits a moment for the second one.": "Mit Doppelklick wartet ein einfacher Druck kurz auf den zweiten.",
	Toggle: "Umschalten",
	"Turn on": "Anschalten",
	"Turn off": "Ausschalten",
	Below: "Unten",
	Above: "Oben",
	Name: "Name",
	"Name of the voice": "Name der Stimme",
	Picture: "Bild",
	Icon: "Symbol",
	"Voice in use": "Aktive Stimme",
	Frame: "Rahmen",
	"Touch screen": "Touchscreen",
	"Ignore touches": "Berührungen ignorieren",
	"On or off": "An oder aus",
	"Muted or live": "Stumm oder live",
	"Use this voice": "Diese Stimme nehmen",
	"Use it, or back to my own voice": "Nehmen, oder zurück zur eigenen Stimme",
	"Star or unstar it": "Stern setzen oder entfernen",
	"Use it while held": "Nehmen, solange gehalten",
	"Use it while held: when you let go of the key, the voice from before comes back.": "Nehmen, solange gehalten: Wenn du die Taste loslässt, kommt die Stimme von vorher zurück.",
	"On only while held": "Nur an, solange gehalten",
	"Off only while held": "Nur aus, solange gehalten",
	"On only while held: it turns off again when you let go of the key. Off only while held is the other way round.": "Nur an, solange gehalten: Es geht wieder aus, wenn du die Taste loslässt. Nur aus, solange gehalten ist es umgekehrt.",
	"Push to change voice": "Zum Verzerren drücken",
	"Push for my own voice": "Für die eigene Stimme drücken",
	"Push to change voice: others hear your own voice, and the changed one only while you hold the key. Push for my own voice is the other way round.": "Zum Verzerren drücken: Andere hören deine echte Stimme und die verzerrte nur, solange du die Taste hältst. Für die eigene Stimme drücken ist es umgekehrt.",
	"While the voice is in use": "Solange die Stimme aktiv ist",
	"The frame turns gray while the voice changer is off.": "Der Rahmen wird grau, solange der Stimmverzerrer aus ist.",
	"Play, as the soundboard plays sounds": "Abspielen, wie das Soundboard Sounds spielt",
	"Play, or stop it when it plays": "Abspielen, oder stoppen, wenn er läuft",
	"Stop it": "Stoppen",
	"How the soundboard plays sounds (from the start, play and stop, or on top) is set in Voicenux, under the sounds.": "Wie das Soundboard Sounds spielt (von vorn, abspielen und stoppen oder übereinander), stellst du in Voicenux unter den Sounds ein.",
	"While the sound plays": "Solange der Sound läuft",
	"Turns the voice changer on or off. When it is off, others hear your own voice.": "Schaltet den Stimmverzerrer an oder aus. Ist er aus, hören andere deine echte Stimme.",
	"Mutes your microphone in Voicenux. Others hear nothing, sounds of the soundboard still play.": "Schaltet dein Mikrofon in Voicenux stumm. Andere hören dich nicht, Sounds vom Soundboard laufen weiter.",
	Mute: "Stummschalten",
	Unmute: "Laut schalten",
	"Push to talk": "Zum Sprechen drücken",
	"Push to mute": "Zum Stummschalten drücken",
	"Push to talk: muted, and live only while you hold the key. Push to mute is the other way round.": "Zum Sprechen drücken: stumm, und nur live, solange du die Taste hältst. Zum Stummschalten drücken ist es umgekehrt.",
	"Hear your changed voice in your headphones.": "Hör deine verzerrte Stimme im Kopfhörer.",
	"Plays the background sounds of a voice, like rain or a crowd, if it has some.": "Spielt die Hintergrundgeräusche einer Stimme, etwa Regen oder eine Menge, falls sie welche hat.",
	"With favorites starred in Voicenux, next and previous go through them only. Random picks from your favorites when you have two or more.": "Hast du in Voicenux Favoriten mit Stern, gehen Nächste und Vorherige nur durch sie. Zufällig wählt aus den Favoriten, sobald es mindestens zwei sind.",
	"Next voice": "Nächste Stimme",
	"Previous voice": "Vorherige Stimme",
	"Random voice": "Zufällige Stimme",
	"Stops every sound of the soundboard. The key is white while a sound plays.": "Stoppt alle Sounds vom Soundboard. Die Taste ist weiß, solange ein Sound läuft.",
	"The volume of the soundboard, for everyone who hears the sounds.": "Die Lautstärke des Soundboards, für alle, die die Sounds hören.",
	Louder: "Lauter",
	Quieter: "Leiser",
	"Set to a value": "Auf einen Wert setzen",
	Step: "Schritt",
	Value: "Wert",
	Volume: "Lautstärke",
	"Hear the sounds of the soundboard yourself. Others hear them either way.": "Hör die Sounds vom Soundboard selbst. Andere hören sie so oder so.",
	"Turn to pick a voice, press to use it. Press while it shows the voice in use to turn the voice changer on or off.": "Drehen wählt eine Stimme, Drücken nimmt sie. Zeigt er die aktive Stimme, schaltet Drücken den Stimmverzerrer an oder aus.",
	Voices: "Stimmen",
	All: "Alle",
	Favorites: "Favoriten",
	"Star voices in Voicenux to make them favorites.": "Markiere Stimmen in Voicenux mit dem Stern, dann sind sie Favoriten.",
	Turning: "Drehen",
	"Picks a voice, press to use it": "Wählt eine Stimme, Drücken nimmt sie",
	"Uses the voice right away": "Nimmt die Stimme sofort",
	"Turn for the volume of the soundboard.": "Drehen ändert die Lautstärke des Soundboards.",
	"Stop all sounds": "Alle Sounds stoppen",
	"Hear sounds on or off": "Sounds selbst hören an oder aus",
};

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
