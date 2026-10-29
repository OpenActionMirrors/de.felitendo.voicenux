// The actions as OpenDeck lists them. `cargo xtask manifest` writes manifest.json and de.json from
// this table, and the plugin takes the icons of its keys from it. OpenDeck lists the actions in this
// order: the ones people use most come first.
pub const ID: &str = "de.felitendo.voicenux";

pub struct Dial {
	pub icon: &'static str,
	pub layout: &'static str,
	pub rotate: &'static str,
	pub push: &'static str,
	pub touch: &'static str,
}

pub struct ActionDef {
	/// After "de.felitendo.voicenux."
	pub id: &'static str,
	pub name: &'static str,
	/// Glyph of the icon in the action list (images/actions/<icon>.svg).
	pub icon: &'static str,
	pub tooltip: &'static str,
	/// Key images, images/keys/<name>.svg. Dials have one image, images/encoder/<icon>-key.svg.
	pub states: &'static [&'static str],
	pub dial: Option<Dial>,
	/// What OpenDeck shows when its language is German.
	pub de_name: &'static str,
	pub de_tooltip: &'static str,
}

const fn key(
	id: &'static str,
	name: &'static str,
	icon: &'static str,
	tooltip: &'static str,
	states: &'static [&'static str],
	de: [&'static str; 2],
) -> ActionDef {
	ActionDef { id, name, icon, tooltip, states, dial: None, de_name: de[0], de_tooltip: de[1] }
}

const fn dial(
	id: &'static str,
	name: &'static str,
	icon: &'static str,
	tooltip: &'static str,
	layout: &'static str,
	trigger: [&'static str; 3],
	de: [&'static str; 2],
) -> ActionDef {
	ActionDef {
		id,
		name,
		icon,
		tooltip,
		states: &[],
		dial: Some(Dial { icon, layout, rotate: trigger[0], push: trigger[1], touch: trigger[2] }),
		de_name: de[0],
		de_tooltip: de[1],
	}
}

pub static ACTIONS: &[ActionDef] = &[
	key(
		"voice",
		"Voice",
		"voice",
		"Turn a voice on or off, or use it only while you hold the key. Shows its picture, with a frame while it is in use.",
		&["voice"],
		[
			"Stimme",
			"Eine Stimme an- oder ausschalten, oder nur nehmen, solange du die Taste hältst. Zeigt ihr Bild, mit Rahmen, solange sie aktiv ist.",
		],
	),
	key(
		"sound",
		"Sound",
		"sound",
		"Play a sound of the soundboard: from the start, on top, or play and stop. Shows its picture, with a frame while it plays.",
		&["sound"],
		[
			"Sound",
			"Einen Sound vom Soundboard abspielen: von vorne, übereinander oder mit Start/Stopp. Zeigt sein Bild, mit Rahmen, solange er läuft.",
		],
	),
	key(
		"voicechanger",
		"Voice Changer",
		"voicechanger",
		"Turn the voice changer on or off, or push to change your voice.",
		&["voicechanger", "voicechanger-on"],
		["Stimmverzerrer", "Den Stimmverzerrer an- oder ausschalten, oder zum Verzerren drücken."],
	),
	key(
		"mute",
		"Mute",
		"mic",
		"Mute or unmute your microphone, or push to talk.",
		&["unmuted", "muted"],
		["Stummschalten", "Dein Mikrofon stumm- oder lautschalten, oder zum Sprechen drücken."],
	),
	key(
		"hearmyself",
		"Hear Myself",
		"headphones",
		"Hear your changed voice in your headphones, on or off.",
		&["hearmyself", "hearmyself-on"],
		["Mich hören", "Deine verzerrte Stimme im Kopfhörer hören, an oder aus."],
	),
	key(
		"randomvoice",
		"Random Voice",
		"random",
		"Use a random voice, out of your favorites when you have two or more.",
		&["random"],
		["Zufällige Stimme", "Eine zufällige Stimme nehmen, aus deinen Favoriten, sobald es mindestens zwei sind."],
	),
	key(
		"nextvoice",
		"Next Voice",
		"next",
		"Use the next or previous voice. Can show the voice in use.",
		&["next"],
		["Nächste Stimme", "Die nächste oder vorherige Stimme nehmen. Kann die aktive Stimme zeigen."],
	),
	key(
		"stopsounds",
		"Stop Sounds",
		"stop",
		"Stop all sounds that play.",
		&["stopsounds", "stopsounds-on"],
		["Sounds stoppen", "Alle Sounds stoppen, die gerade laufen."],
	),
	key(
		"randomsound",
		"Random Sound",
		"random",
		"Play a random sound of the soundboard. Shows a dice, or already the sound that plays next.",
		&["random"],
		[
			"Zufälliger Sound",
			"Einen zufälligen Sound vom Soundboard abspielen. Zeigt einen Würfel oder schon den Sound, der als Nächstes läuft.",
		],
	),
	key(
		"saysound",
		"Say a Sound",
		"saysound",
		"Hold the key and say a sound. It finds the one people like most on MyInstants and keeps it. Press to play it.",
		&["saysound"],
		[
			"Sound ansagen",
			"Taste halten und einen Sound sagen. Sie sucht den beliebtesten auf MyInstants und behält ihn. Drücken spielt ihn ab.",
		],
	),
	key(
		"background",
		"Background",
		"background",
		"Background sounds of the voice, like rain or a crowd, on or off.",
		&["background", "background-on"],
		["Hintergrund", "Hintergrundgeräusche der Stimme, etwa Regen oder eine Menge, an oder aus."],
	),
	key(
		"soundvolume",
		"Soundboard Volume",
		"volumeUp",
		"Make the sounds louder, quieter or set their volume. Hold the key to keep going.",
		&["soundvolume"],
		["Soundboard-Lautstärke", "Die Sounds lauter oder leiser machen oder ihre Lautstärke festlegen. Gedrückt halten macht weiter."],
	),
	key(
		"hearsounds",
		"Hear Sounds",
		"hearsounds",
		"Hear the sounds yourself too, on or off.",
		&["hearsounds", "hearsounds-on"],
		["Sounds selbst hören", "Die Sounds auch selbst hören, an oder aus."],
	),
	dial(
		"voicedial",
		"Voice (Dial)",
		"voice",
		"Turn the dial to pick a voice (with its picture), press to use it.",
		"layouts/voice-dial.json",
		["Pick a voice", "Use it", "Use it"],
		["Stimme (Drehregler)", "Drehen wählt eine Stimme (mit Bild), Drücken nimmt sie."],
	),
	dial(
		"sounddial",
		"Sound (Dial)",
		"sound",
		"Turn the dial to pick a sound (with its picture), press to play it.",
		"layouts/voice-dial.json",
		["Pick a sound", "Play it", "Play it"],
		["Sound (Drehregler)", "Drehen wählt einen Sound (mit Bild), Drücken spielt ihn ab."],
	),
	dial(
		"soundvolumedial",
		"Soundboard Volume (Dial)",
		"volume",
		"Turn the dial for the soundboard volume, press to stop all sounds.",
		"layouts/dial.json",
		["Volume", "Stop all sounds", "Stop all sounds"],
		["Soundboard-Lautstärke (Drehregler)", "Drehen ändert die Lautstärke des Soundboards, Drücken stoppt alle Sounds."],
	),
];

/// Icon and state images of an action, by its full UUID.
pub fn action_def(uuid: &str) -> Option<&'static ActionDef> {
	let id = uuid.strip_prefix(ID)?.strip_prefix('.')?;
	ACTIONS.iter().find(|a| a.id == id)
}

impl ActionDef {
	/// The names of the state images, like the plugin read them from manifest.json.
	pub fn state_images(&self) -> Vec<String> {
		match &self.dial {
			Some(dial) => vec![format!("{}-key", dial.icon)],
			None => self.states.iter().map(|s| s.to_string()).collect(),
		}
	}
}
