// Sounds: one sound of the soundboard, a random one, stop all, and the soundboard volume.
use std::hash::{BuildHasher, Hasher};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::time::Instant;

use super::{Behavior, Gesture, KeyRc, Kind, Press, defaults};
use crate::i18n::t;
use crate::icons::{colors, volume_glyph};
use crate::js::{self, fmt};
use crate::render::{Frame, GlyphKey, PictureKey, dial_icon, glyph_key, picture_key};
use crate::settings::or;
use crate::timers::Timer;
use crate::voicenux::{Change, Sound as SoundInfo};
use crate::{Error, Res};

// 0 to 100, whole numbers. NaN is 0.
fn clamp(value: f64) -> f64 {
	js::max(0.0, js::min(100.0, js::round(or(value, 0.0))))
}

// Like Math.random(): a number from 0 up to 1. Every RandomState starts from new random keys.
fn random() -> f64 {
	let bits = std::collections::hash_map::RandomState::new().build_hasher().finish();
	(bits >> 11) as f64 / (1u64 << 53) as f64
}

/// Plays a sound in a mode of its own. Voicenux 0.1.0 does not know sound-restart, sound-toggle
/// and sound-overlap yet: there a key plays as the soundboard is set, and only on top can not work.
async fn play_as(k: &KeyRc, mode: &str, id: &str) -> Res<()> {
	let Err(error) = k.call(&format!("sound-{mode}"), Some(id)).await else { return Ok(()) };
	if !error.message().contains("unknown action") {
		return Err(error);
	}
	if mode == "overlap" {
		return Err(Error::from("Play on top needs a newer Voicenux"));
	}
	if mode == "toggle" && k.client.playing().iter().any(|p| p == id) {
		k.call("stop-sound", Some(id)).await?;
	} else {
		k.call("sound", Some(id)).await?;
	}
	Ok(())
}

// ----- One sound of the soundboard. The key has a frame while it plays. -----
// The play modes of Voicenux, for this key alone: restart (from the start), toggle (play and stop),
// overlap (another copy on top). stop: stop it.

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SoundSettings {
	/// The id of the sound.
	sound: String,
	show_name: bool,
	/// "below" or "above".
	name_at: String,
	/// A frame while the sound plays.
	show_playing: bool,
}

struct Sound;

#[async_trait(?Send)]
impl Behavior for Sound {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let s: SoundSettings = k.settings();
		let Some(sound) = k.client.sound(&s.sound) else {
			let title = if s.show_name { s.sound.as_str() } else { "" };
			k.own_title(title);
			k.set_image(&glyph_key("sound", &GlyphKey { color: colors::GRAY, title, title_at: &s.name_at, ..Default::default() }));
			return;
		};
		let title = if s.show_name { sound.name.as_str() } else { "" };
		k.own_title(title);
		let playing = k.client.playing().contains(&s.sound);
		let png = k.picture(&sound.icon);
		k.set_image(&picture_key(&PictureKey {
			png: png.as_deref(),
			fallback: "sound",
			title,
			title_at: &s.name_at,
			frame: (s.show_playing && playing).then_some(Frame::On),
			..Default::default()
		}));
	}

	async fn on_press(&self, k: &KeyRc, press: Press) -> Res<()> {
		let id = k.settings::<SoundSettings>().sound;
		let Some(sound) = k.client.sound(&id) else {
			return Err(Error::from(if id.is_empty() { "no sound selected" } else { "This sound is gone from Voicenux" }));
		};
		match press.op.as_str() {
			"stop" => k.call("stop-sound", Some(&sound.id)).await.map(|_| ()),
			op => play_as(k, op, &sound.id).await,
		}
	}
}

pub static SOUND: Kind = Kind {
	uuid: "de.felitendo.voicenux.sound",
	name: "Sound",
	defaults: || {
		defaults(
			json!({ "sound": "", "press": "restart", "hold": "none", "double": "none", "showName": true, "nameAt": "below", "showPlaying": true }),
		)
	},
	press_key: "press",
	ops: &["restart", "toggle", "overlap", "stop"],
	required: &["sound"],
	behavior: &Sound,
	..Kind::BASE
};

// ----- A random sound. It picks the next one ahead, so the key can show it ("next"), or shows a -----
// dice. Never the same sound twice in a row. look: "dice", or "next" (the picture of the sound the
// next press plays).

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RandomSoundSettings {
	look: String,
	show_name: bool,
	name_at: String,
}

#[derive(Default)]
struct RandomPick {
	next: Option<String>,
	last: Option<String>,
}

struct RandomSound;

impl RandomSound {
	// The sound the next press plays.
	fn next(&self, k: &KeyRc) -> Option<SoundInfo> {
		let next = k.own::<RandomPick>().next.clone();
		if let Some(sound) = next.and_then(|id| k.client.sound(&id)) {
			return Some(sound);
		}
		self.pick(k);
		let next = k.own::<RandomPick>().next.clone();
		next.and_then(|id| k.client.sound(&id))
	}

	fn pick(&self, k: &KeyRc) {
		let sounds = k.client.sounds();
		let mut x = k.own::<RandomPick>();
		let others: Vec<&SoundInfo> =
			if sounds.len() > 1 { sounds.iter().filter(|s| Some(&s.id) != x.last.as_ref()).collect() } else { sounds.iter().collect() };
		x.next = if others.is_empty() { None } else { Some(others[(random() * others.len() as f64) as usize % others.len()].id.clone()) };
	}
}

#[async_trait(?Send)]
impl Behavior for RandomSound {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let s: RandomSoundSettings = k.settings();
		let sound = self.next(k);
		let Some(sound) = sound.filter(|_| s.look == "next") else { return k.glyph_for_title("", "below") };
		let title = if s.show_name { sound.name.as_str() } else { "" };
		k.own_title(title);
		let png = k.picture(&sound.icon);
		k.set_image(&picture_key(&PictureKey {
			png: png.as_deref(),
			fallback: "sound",
			title,
			title_at: &s.name_at,
			..Default::default()
		}));
	}

	async fn on_press(&self, k: &KeyRc, _press: Press) -> Res<()> {
		let sound = self.next(k).ok_or("The soundboard has no sounds")?;
		k.own::<RandomPick>().last = Some(sound.id.clone());
		self.pick(k);
		play_as(k, "restart", &sound.id).await?;
		self.render(k);
		Ok(())
	}
}

pub static RANDOM_SOUND: Kind = Kind {
	uuid: "de.felitendo.voicenux.randomsound",
	name: "RandomSound",
	defaults: || defaults(json!({ "look": "dice", "showName": true, "nameAt": "below" })),
	behavior: &RandomSound,
	new_state: || Box::new(RandomPick::default()),
	..Kind::BASE
};

// ----- Say a sound: hold the key and say what you want to hear. Voicenux finds the sound most -----
// people liked on MyInstants and keeps it for the key, off the soundboard. A press plays it. While
// you hold the key, Voicenux listens, and others do not hear you.

/// Shorter is a press. Voicenux listens from the start anyway, so no word is lost meanwhile.
const SAY_AFTER_MS: f64 = 400.0;
/// The first time, Voicenux downloads its speech recognition (about 290 MB).
const TRANSCRIBE_MS: f64 = 600_000.0;
const FIND_MS: f64 = 120_000.0;

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SaySoundSettings {
	/// The id of the sound it found. Not in the list of sounds: Voicenux keeps it off the
	/// soundboard, unless it was there already. So the key keeps its name and picture too.
	sound: String,
	sound_name: String,
	sound_icon: String,
	/// How a press plays it: "restart", "toggle" or "overlap".
	play: String,
	/// What you speak: "auto" (English or the language of Voicenux), "en" or "de".
	language: String,
	show_name: bool,
	name_at: String,
	show_playing: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
	#[default]
	Idle,
	Listening,
	Searching,
}

#[derive(Default)]
struct Saying {
	/// When the key went down. None while it is up.
	down: Option<Instant>,
	/// Why Voicenux does not listen to this press, like an older Voicenux.
	deaf: Option<Error>,
	phase: Phase,
	/// What you said, on the key while it searches.
	said: String,
	look: Option<Timer>,
}

struct SaySound;

impl SaySound {
	async fn play(&self, k: &KeyRc) -> Res<()> {
		let s: SaySoundSettings = k.settings();
		if s.sound.is_empty() {
			return Err(Error::from("Hold the key and say a sound"));
		}
		let mode = if matches!(s.play.as_str(), "toggle" | "overlap") { s.play.as_str() } else { "restart" };
		play_as(k, mode, &s.sound).await.map_err(|error| {
			// Voicenux keeps the last 50 sounds it found, one this old is gone.
			if error.message().starts_with("no sound called") {
				Error::from("This sound is gone. Hold the key to find a new one.")
			} else {
				error
			}
		})
	}

	fn show(&self, k: &KeyRc, phase: Phase) {
		{
			let mut x = k.own::<Saying>();
			x.phase = phase;
			if phase != Phase::Searching {
				x.said.clear();
			}
		}
		k.draw();
	}

	/// What you said, then the sound for it.
	async fn find(&self, k: &KeyRc) -> Res<()> {
		let s: SaySoundSettings = k.settings();
		let language = Some(s.language.as_str()).filter(|l| !l.is_empty() && *l != "auto");
		let heard = k.client.call_waiting("transcribe", language, TRANSCRIBE_MS).await?;
		let said = heard["text"].as_str().unwrap_or_default().trim().to_string();
		if said.is_empty() {
			return Err(Error::from("Nothing heard. Hold the key and say a sound."));
		}
		k.own::<Saying>().said = said.clone();
		k.draw();
		let found = k.client.call_waiting("find-sound", Some(&said), FIND_MS).await?;
		let id = found["id"].as_str().filter(|id| !id.is_empty()).ok_or("Voicenux found no sound")?;
		k.save_settings(&[("sound", id.into()), ("soundName", found["name"].clone()), ("soundIcon", found["icon"].clone())]);
		Ok(())
	}
}

/// Voicenux 0.3.0 and older cannot listen.
fn newer(error: Error) -> Error {
	if error.message().contains("unknown action") { Error::from("Saying a sound needs a newer Voicenux") } else { error }
}

#[async_trait(?Send)]
impl Behavior for SaySound {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let s: SaySoundSettings = k.settings();
		let (phase, said) = {
			let x = k.own::<Saying>();
			(x.phase, x.said.clone())
		};
		let busy = match phase {
			Phase::Listening => Some(("mic", colors::RED, t("Listening"))),
			Phase::Searching => Some(("saysound", colors::WHITE, if said.is_empty() { t("Thinking") } else { said })),
			Phase::Idle => None,
		};
		if let Some((glyph, color, title)) = busy {
			k.own_title(&title);
			return k.set_image(&glyph_key(glyph, &GlyphKey { color, title: &title, title_at: &s.name_at, ..Default::default() }));
		}
		// Nothing found yet: the lens.
		if s.sound.is_empty() {
			return k.glyph_for_title("", "below");
		}
		// A sound of the soundboard may have a new name by now.
		let (name, icon) = match k.client.sound(&s.sound) {
			Some(sound) => (sound.name, sound.icon),
			None => (s.sound_name, s.sound_icon),
		};
		let title = if s.show_name { name.as_str() } else { "" };
		k.own_title(title);
		let playing = k.client.playing().contains(&s.sound);
		let png = k.picture(&icon);
		k.set_image(&picture_key(&PictureKey {
			png: png.as_deref(),
			fallback: "sound",
			title,
			title_at: &s.name_at,
			frame: (s.show_playing && playing).then_some(Frame::On),
			// Hold for another sound.
			badge: "saysound",
			..Default::default()
		}));
	}

	fn on_disappear(&self, k: &KeyRc) {
		let listening = {
			let mut x = k.own::<Saying>();
			if let Some(look) = x.look.take() {
				look.clear();
			}
			x.down.take().is_some() && x.deaf.take().is_none()
		};
		if listening {
			// Sent right away, no need to wait for the answer.
			drop(k.client.call("listen", Some("off")));
		}
	}

	async fn on_press(&self, k: &KeyRc, _press: Press) -> Res<()> {
		if k.is_in_multi_action() {
			return self.play(k).await;
		}
		let now = Instant::now();
		let busy = {
			let mut x = k.own::<Saying>();
			x.down = Some(now);
			x.deaf = None;
			x.phase != Phase::Idle
		};
		if busy {
			k.own::<Saying>().deaf = Some(Error::from("Still looking for the last sound"));
			return Ok(());
		}
		if let Err(error) = k.call("listen", Some("on")).await {
			// A press still plays.
			k.own::<Saying>().deaf = Some(newer(error));
			return Ok(());
		}
		let look = k.after(SAY_AFTER_MS, move |k| {
			let held = {
				let x = k.own::<Saying>();
				x.down == Some(now) && x.phase == Phase::Idle
			};
			if held {
				SaySound.show(&k, Phase::Listening);
			}
		});
		k.own::<Saying>().look = Some(look);
		Ok(())
	}

	async fn on_release(&self, k: &KeyRc) -> Res<()> {
		let (down, deaf) = {
			let mut x = k.own::<Saying>();
			if let Some(look) = x.look.take() {
				look.clear();
			}
			(x.down.take(), x.deaf.take())
		};
		// In a multi action the press played already.
		let Some(down) = down else { return Ok(()) };
		if down.elapsed().as_secs_f64() * 1000.0 < SAY_AFTER_MS {
			if deaf.is_none() {
				let _ = k.call("listen", Some("off")).await;
			}
			return self.play(k).await;
		}
		if let Some(error) = deaf {
			return Err(error);
		}
		self.show(k, Phase::Searching);
		let found = self.find(k).await;
		self.show(k, Phase::Idle);
		found.map_err(newer)?;
		k.clear_error();
		k.show_ok();
		Ok(())
	}
}

pub static SAY_SOUND: Kind = Kind {
	uuid: "de.felitendo.voicenux.saysound",
	name: "SaySound",
	defaults: || {
		defaults(json!({
			"sound": "",
			"soundName": "",
			"soundIcon": "",
			"play": "restart",
			"language": "auto",
			"showName": true,
			"nameAt": "below",
			"showPlaying": true,
		}))
	},
	behavior: &SaySound,
	new_state: || Box::new(Saying::default()),
	..Kind::BASE
};

// ----- Stops every sound. White while something plays. -----

struct StopSounds;

#[async_trait(?Send)]
impl Behavior for StopSounds {
	fn render_offline(&self, k: &KeyRc) {
		k.set_state(0);
	}

	fn render(&self, k: &KeyRc) {
		k.set_state(u16::from(!k.client.playing().is_empty()));
		// The images of the manifest, not one from before.
		k.glyph_for_title("", "below");
	}

	async fn on_press(&self, k: &KeyRc, _press: Press) -> Res<()> {
		k.call("stop-sounds", None).await.map(|_| ())
	}
}

pub static STOP_SOUNDS: Kind = Kind {
	uuid: "de.felitendo.voicenux.stopsounds",
	name: "StopSounds",
	defaults: || defaults(json!({})),
	behavior: &StopSounds,
	..Kind::BASE
};

// ----- Volume of the soundboard: up, down or to a value. Hold keeps going by default. -----

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SoundVolumeSettings {
	/// Percent per press.
	step: f64,
	/// What "set" sets, in percent.
	value: f64,
	show_value: bool,
	value_at: String,
}

struct SoundVolume;

impl SoundVolume {
	fn glyph_name(&self, k: &KeyRc) -> &'static str {
		volume_glyph(&k.gesture(Gesture::Press)).unwrap_or("volume")
	}
}

#[async_trait(?Send)]
impl Behavior for SoundVolume {
	fn on_change(&self, k: &KeyRc, change: Change) {
		if change == Change::State {
			self.render(k);
		}
	}

	fn render_offline(&self, k: &KeyRc) {
		k.set_image(&glyph_key(self.glyph_name(k), &GlyphKey::default()));
	}

	fn render(&self, k: &KeyRc) {
		let s: SoundVolumeSettings = k.settings();
		let title = match k.vn().sound_volume {
			Some(volume) if s.show_value => format!("{}%", fmt(volume)),
			_ => String::new(),
		};
		k.own_title(&title);
		k.set_image(&glyph_key(self.glyph_name(k), &GlyphKey { title: &title, title_at: &s.value_at, ..Default::default() }));
	}

	async fn on_press(&self, k: &KeyRc, press: Press) -> Res<()> {
		let s: SoundVolumeSettings = k.settings();
		let mode = press.op;
		let now = k.vn().sound_volume.unwrap_or(0.0);
		if mode == "set" {
			if !press.repeat {
				k.call("sound-volume", Some(&fmt(clamp(s.value)))).await?;
			}
		} else {
			let step = or(s.step, 1.0).abs();
			let next = clamp(now + if mode == "down" { -step } else { step });
			if next != now {
				k.call("sound-volume", Some(&fmt(next))).await?;
			}
		}
		self.render(k);
		Ok(())
	}
}

pub static SOUND_VOLUME: Kind = Kind {
	uuid: "de.felitendo.voicenux.soundvolume",
	name: "SoundVolume",
	defaults: || {
		defaults(
			json!({ "mode": "up", "hold": "repeat", "double": "none", "step": 10, "value": 50, "showValue": true, "valueAt": "below" }),
		)
	},
	press_key: "mode",
	ops: &["up", "down", "set"],
	can_repeat: true,
	behavior: &SoundVolume,
	..Kind::BASE
};

// ----- Stream Deck + dial: turn for the soundboard volume, press or touch to stop all sounds. -----
// pressAction: "stop" all sounds, "hear" (hear the sounds yourself, on or off), or "nothing".

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SoundVolumeDialSettings {
	/// Percent per tick.
	step: f64,
	press_action: String,
}

struct SoundVolumeDial;

#[async_trait(?Send)]
impl Behavior for SoundVolumeDial {
	fn layout(&self) -> Option<&'static str> {
		Some("layouts/dial.json")
	}

	fn render_offline(&self, k: &KeyRc) {
		k.set_feedback(json!({
			"title": t("Soundboard"),
			"icon": dial_icon("volume", false, None),
			"value": k.dial_status(),
			"indicator": { "value": 0 },
		}));
	}

	fn render(&self, k: &KeyRc) {
		let volume = k.vn().sound_volume.unwrap_or(0.0);
		let playing = !k.client.playing().is_empty();
		k.set_feedback(json!({
			"title": t("Soundboard"),
			"icon": dial_icon(if playing { "sound" } else { "volume" }, false, Some(if playing { colors::CYAN } else { colors::WHITE })),
			// With its color: OpenDeck keeps the one of the "Voicenux off" text otherwise.
			"value": { "value": format!("{}%", fmt(volume)), "color": colors::WHITE },
			"indicator": { "value": js::num(volume), "bar_fill_c": colors::SKY },
		}));
	}

	async fn on_dial_rotate(&self, k: &KeyRc, ticks: f64) -> Res<()> {
		let now = k.vn().sound_volume.unwrap_or(0.0);
		let step = or(k.settings::<SoundVolumeDialSettings>().step.abs(), 1.0);
		let next = clamp(now + ticks * step);
		if next != now {
			k.call("sound-volume", Some(&fmt(next))).await?;
		}
		self.render(k);
		Ok(())
	}

	async fn on_dial_down(&self, k: &KeyRc) -> Res<()> {
		match k.settings::<SoundVolumeDialSettings>().press_action.as_str() {
			"hear" => k.call("hear-sounds", Some("toggle")).await.map(|_| ()),
			"nothing" => Ok(()),
			_ => k.call("stop-sounds", None).await.map(|_| ()),
		}
	}
}

pub static SOUND_VOLUME_DIAL: Kind = Kind {
	uuid: "de.felitendo.voicenux.soundvolumedial",
	name: "SoundVolumeDial",
	defaults: || defaults(json!({ "step": 5, "pressAction": "stop", "disableTouch": false })),
	behavior: &SoundVolumeDial,
	..Kind::BASE
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::testing::*;
	use crate::settings::testing::assert_defaults;

	#[test]
	fn every_setting_has_a_default() {
		assert_defaults::<SoundSettings>(&[&SOUND]);
		assert_defaults::<RandomSoundSettings>(&[&RANDOM_SOUND]);
		assert_defaults::<SaySoundSettings>(&[&SAY_SOUND]);
		assert_defaults::<SoundVolumeSettings>(&[&SOUND_VOLUME]);
		assert_defaults::<SoundVolumeDialSettings>(&[&SOUND_VOLUME_DIAL]);
	}

	fn args(op: &str) -> Press {
		Press { op: op.into(), repeat: false }
	}

	#[tokio::test]
	async fn a_sound_key_plays_in_its_own_mode_also_with_an_older_voicenux() {
		local(async {
			let (_p, k) = key(&SOUND, json!({ "sound": "airhorn", "press": "overlap" }));
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&k), ["sound-overlap", "airhorn"]);
			Sound.on_press(&k, args("stop")).await.unwrap();
			assert_eq!(last(&k), ["stop-sound", "airhorn"]);
			// Voicenux 0.1.0 knows only "sound": toggle and restart still work, on top can not.
			let (_p, old) = key(&SOUND, json!({ "sound": "airhorn", "press": "toggle" }));
			old_voicenux(&old);
			old.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&old), ["sound", "airhorn"]);
			old.client.set_playing(vec!["airhorn".into()]);
			old.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&old), ["stop-sound", "airhorn"]);
			assert_eq!(Sound.on_press(&old, args("overlap")).await, Err(Error::from("Play on top needs a newer Voicenux")));
		})
		.await;
	}

	#[tokio::test]
	async fn a_key_without_its_sound_says_why() {
		local(async {
			let (_p, k) = key(&SOUND, json!({ "sound": "" }));
			assert_eq!(k.press(Gesture::Press, false).await, Err(Error::from("no sound selected")));
		})
		.await;
	}

	#[tokio::test]
	async fn a_random_sound_key_plays_the_sound_it_shows_then_picks_another_one() {
		local(async {
			let (_p, k) = key(&RANDOM_SOUND, json!({ "look": "next" }));
			k.client.set_sounds(vec![
				sound("airhorn", "Air Horn", "postal-horn"),
				sound("bruh", "Bruh", "face"),
				sound("coin", "Coin", "coin"),
			]);
			for _ in 0..20 {
				let shown = RandomSound.next(&k).map(|s| s.id).unwrap_or_default();
				k.press(Gesture::Press, false).await.unwrap();
				assert_eq!(last(&k), ["sound-restart".to_string(), shown.clone()]);
				assert_ne!(RandomSound.next(&k).map(|s| s.id), Some(shown), "the same sound twice in a row");
			}
			k.client.set_sounds(Vec::new());
			assert_eq!(k.press(Gesture::Press, false).await, Err(Error::from("The soundboard has no sounds")));
		})
		.await;
	}

	#[tokio::test]
	async fn soundboard_volume_stays_between_0_and_100() {
		local(async {
			let (_p, k) = key(&SOUND_VOLUME, json!({ "step": 15 }));
			assert_eq!(k.gesture(Gesture::Hold), "repeat");
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&k), ["sound-volume", "95"]);
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&k), ["sound-volume", "100"]);
			let count = calls(&k).len();
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(calls(&k).len(), count, "no call when it is at 100 already");
			SoundVolume.on_press(&k, args("set")).await.unwrap();
			assert_eq!(last(&k), ["sound-volume", "50"]);
		})
		.await;
	}

	fn said(k: &KeyRc, words: &str) {
		if let Some(fake) = k.client.fake.borrow_mut().as_mut() {
			fake.said = words.into();
		}
	}

	async fn hold(k: &KeyRc, ms: f64) {
		k.key_down(false).await;
		crate::timers::sleep(ms).await;
		k.key_up().await;
	}

	#[tokio::test(start_paused = true)]
	async fn holding_a_say_key_finds_the_sound_and_a_press_plays_it() {
		local(async {
			let (_p, k) = key(&SAY_SOUND, json!({}));
			said(&k, "Vine boom");
			k.key_down(false).await;
			assert!(k.vn().listening, "Voicenux listens from the start");
			crate::timers::sleep(SAY_AFTER_MS + 100.0).await;
			assert_eq!(k.own::<Saying>().phase, Phase::Listening);
			k.key_up().await;
			assert_eq!(last(&k), ["find-sound", "Vine boom"]);
			let s: SaySoundSettings = k.settings();
			assert_eq!((s.sound.as_str(), s.sound_name.as_str(), s.sound_icon.as_str()), ("found-1", "Vine boom", "collision"));
			assert!(k.client.sound("found-1").is_none(), "not on the soundboard");
			assert_eq!(k.own::<Saying>().phase, Phase::Idle);
			let saved =
				k.sd.sent.as_ref().unwrap().borrow().iter().any(|m| m["event"] == "setSettings" && m["payload"]["sound"] == "found-1");
			assert!(saved, "OpenDeck keeps the sound");
			// A press stops listening and plays it.
			hold(&k, 50.0).await;
			assert_eq!(calls(&k)[calls(&k).len() - 2].0, "listen");
			assert_eq!(last(&k), ["sound-restart", "found-1"]);
			assert!(!k.vn().listening);
		})
		.await;
	}

	#[tokio::test(start_paused = true)]
	async fn a_say_key_says_why_it_found_nothing() {
		local(async {
			let (_p, k) = key(&SAY_SOUND, json!({ "language": "de" }));
			hold(&k, 50.0).await;
			assert_eq!(k.last_error().as_deref(), Some("Hold the key and say a sound"));
			hold(&k, 1000.0).await;
			assert_eq!(last(&k), ["transcribe", "de"]);
			assert_eq!(k.last_error().as_deref(), Some("Nothing heard. Hold the key and say a sound."));
			assert_eq!(k.settings::<SaySoundSettings>().sound, "");
			// An older Voicenux cannot listen, but it still plays.
			let (_p, old) = key(&SAY_SOUND, json!({ "sound": "airhorn" }));
			old_voicenux(&old);
			hold(&old, 50.0).await;
			assert_eq!(last(&old), ["sound", "airhorn"]);
			hold(&old, 1000.0).await;
			assert_eq!(old.last_error().as_deref(), Some("Saying a sound needs a newer Voicenux"));
		})
		.await;
	}

	#[test]
	fn random_numbers_stay_below_one() {
		assert!((0..100).map(|_| random()).all(|r| (0.0..1.0).contains(&r)));
	}
}
