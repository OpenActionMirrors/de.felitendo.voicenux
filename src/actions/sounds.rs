// Sounds: one sound of the soundboard, a random one, stop all, the soundboard volume, and dials to
// pick a sound and for the volume.
use std::hash::{BuildHasher, Hasher};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::{Behavior, Gesture, KeyRc, Kind, Press, defaults};
use crate::i18n::t;
use crate::icons::{colors, volume_glyph};
use crate::js::{self, fmt};
use crate::render::{Frame, GlyphKey, PictureKey, dial_icon, dial_picture, glyph_key, picture_key};
use crate::settings::or;
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
pub(super) async fn play_as(k: &KeyRc, mode: &str, id: &str) -> Res<()> {
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

// ----- Stream Deck + dial: turn to pick a sound (with its picture), press or touch to play it. -----
// list: "all" sounds or only "favorites". play: "restart" (from the start) or "toggle" (play and
// stop). It stays on the sound you picked.

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SoundDialSettings {
	list: String,
	play: String,
}

#[derive(Default)]
struct SoundPick {
	selected: Option<String>,
}

struct SoundDial;

impl SoundDial {
	/// The sounds to turn through. Favorites only when there are some.
	fn list(&self, k: &KeyRc) -> Vec<SoundInfo> {
		let all = k.client.sounds();
		if k.settings::<SoundDialSettings>().list == "favorites" {
			let favorites: Vec<SoundInfo> = all.iter().filter(|s| s.favorite).cloned().collect();
			if !favorites.is_empty() {
				return favorites;
			}
		}
		all
	}

	/// The sound picked, or the first one.
	fn target(&self, k: &KeyRc) -> Option<SoundInfo> {
		let list = self.list(k);
		let selected = k.own::<SoundPick>().selected.clone();
		selected.and_then(|id| list.iter().find(|s| s.id == id).cloned()).or_else(|| list.into_iter().next())
	}
}

#[async_trait(?Send)]
impl Behavior for SoundDial {
	fn layout(&self) -> Option<&'static str> {
		Some("layouts/voice-dial.json")
	}

	fn on_change(&self, k: &KeyRc, change: Change) {
		if change == Change::Sounds {
			let selected = k.own::<SoundPick>().selected.clone();
			if selected.is_some_and(|id| k.client.sound(&id).is_none()) {
				k.own::<SoundPick>().selected = None;
			}
		}
		self.render(k);
	}

	fn render_offline(&self, k: &KeyRc) {
		k.set_feedback(json!({ "title": t("Sound"), "name": "", "info": k.dial_status(), "picture": dial_picture(None, "sound") }));
	}

	fn render(&self, k: &KeyRc) {
		let Some(sound) = self.target(k) else {
			return k.set_feedback(json!({
				"title": t("Sound"),
				"name": "",
				"info": { "value": t("The soundboard has no sounds"), "color": colors::MUTED },
				"picture": dial_picture(None, "sound"),
			}));
		};
		let sounds = self.list(k);
		let playing = k.client.playing().contains(&sound.id);
		let info = match sounds.iter().position(|s| s.id == sound.id) {
			_ if playing => t("Playing"),
			Some(index) => format!("{} / {}", index + 1, sounds.len()),
			None => String::new(),
		};
		let png = k.picture(&sound.icon);
		k.set_feedback(json!({
			"title": t("Sound"),
			"name": { "value": sound.name, "color": colors::WHITE },
			"info": { "value": info, "color": if playing { colors::CYAN } else { colors::MUTED } },
			"picture": dial_picture(png.as_deref(), "sound"),
		}));
	}

	async fn on_dial_rotate(&self, k: &KeyRc, ticks: f64) -> Res<()> {
		let sounds = self.list(k);
		if sounds.is_empty() {
			return Ok(());
		}
		let target = self.target(k).map(|s| s.id);
		let from = sounds.iter().position(|s| Some(&s.id) == target.as_ref()).unwrap_or(0) as i64;
		let len = sounds.len() as i64;
		let next = sounds[((from + ticks as i64) % len + len) as usize % sounds.len()].id.clone();
		k.own::<SoundPick>().selected = Some(next);
		self.render(k);
		Ok(())
	}

	async fn on_dial_down(&self, k: &KeyRc) -> Res<()> {
		let sound = self.target(k).ok_or("The soundboard has no sounds")?;
		let mode = if k.settings::<SoundDialSettings>().play == "toggle" { "toggle" } else { "restart" };
		play_as(k, mode, &sound.id).await?;
		self.render(k);
		Ok(())
	}
}

pub static SOUND_DIAL: Kind = Kind {
	uuid: "de.felitendo.voicenux.sounddial",
	name: "SoundDial",
	defaults: || defaults(json!({ "list": "all", "play": "restart", "disableTouch": false })),
	behavior: &SoundDial,
	new_state: || Box::new(SoundPick::default()),
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
		assert_defaults::<SoundVolumeSettings>(&[&SOUND_VOLUME]);
		assert_defaults::<SoundVolumeDialSettings>(&[&SOUND_VOLUME_DIAL]);
		assert_defaults::<SoundDialSettings>(&[&SOUND_DIAL]);
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

	#[tokio::test]
	async fn a_sound_dial_picks_a_sound_and_a_press_plays_it() {
		local(async {
			let (_p, k) = key(&SOUND_DIAL, json!({ "play": "toggle" }));
			k.client.set_sounds(vec![sound("airhorn", "Air Horn", "postal-horn"), sound("bruh", "Bruh", "face")]);
			assert_eq!(SoundDial.target(&k).map(|s| s.id), Some("airhorn".into()));
			SoundDial.on_dial_rotate(&k, 1.0).await.unwrap();
			assert_eq!(SoundDial.target(&k).map(|s| s.id), Some("bruh".into()));
			// Past the last one it starts over, both ways.
			SoundDial.on_dial_rotate(&k, 1.0).await.unwrap();
			assert_eq!(SoundDial.target(&k).map(|s| s.id), Some("airhorn".into()));
			SoundDial.on_dial_rotate(&k, -1.0).await.unwrap();
			SoundDial.on_dial_down(&k).await.unwrap();
			assert_eq!(last(&k), ["sound-toggle", "bruh"]);
			// A sound gone from the soundboard: back to the first one.
			k.client.set_sounds(vec![sound("airhorn", "Air Horn", "postal-horn")]);
			SoundDial.on_change(&k, Change::Sounds);
			assert_eq!(SoundDial.target(&k).map(|s| s.id), Some("airhorn".into()));
		})
		.await;
	}

	#[tokio::test]
	async fn a_sound_dial_can_turn_through_favorites_only() {
		local(async {
			let (_p, k) = key(&SOUND_DIAL, json!({ "list": "favorites" }));
			let star = |s: SoundInfo| SoundInfo { favorite: true, ..s };
			// No stars yet: all sounds.
			k.client.set_sounds(vec![sound("airhorn", "Air Horn", "postal-horn"), sound("bruh", "Bruh", "face")]);
			assert_eq!(SoundDial.list(&k).len(), 2);
			k.client.set_sounds(vec![
				sound("airhorn", "Air Horn", "postal-horn"),
				star(sound("bruh", "Bruh", "face")),
				sound("coin", "Coin", "coin"),
				star(sound("drum", "Drum Roll", "drum")),
			]);
			assert_eq!(SoundDial.target(&k).map(|s| s.id), Some("bruh".into()));
			SoundDial.on_dial_rotate(&k, 1.0).await.unwrap();
			assert_eq!(SoundDial.target(&k).map(|s| s.id), Some("drum".into()));
			SoundDial.on_dial_rotate(&k, 1.0).await.unwrap();
			assert_eq!(SoundDial.target(&k).map(|s| s.id), Some("bruh".into()));
		})
		.await;
	}

	#[test]
	fn random_numbers_stay_below_one() {
		assert!((0..100).map(|_| random()).all(|r| (0.0..1.0).contains(&r)));
	}
}
