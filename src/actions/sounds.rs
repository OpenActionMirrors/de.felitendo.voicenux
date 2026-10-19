use std::hash::{BuildHasher, Hasher};

use async_trait::async_trait;
use serde_json::json;

use super::{Behavior, KeyRc, Kind, PressArgs, defaults};
use crate::i18n::t;
use crate::icons::{colors, volume_glyph};
use crate::js::{self, fmt};
use crate::render::{Frame, GlyphKey, PictureKey, dial_icon, glyph_key, picture_key};
use crate::voicenux::Sound as SoundInfo;
use crate::{Error, Res};

// 0 to 100, like Math.max(0, Math.min(100, Math.round(Number(value) || 0))).
fn clamp(value: f64) -> f64 {
	let value = if js::truthy_num(value) { value } else { 0.0 };
	js::max(0.0, js::min(100.0, js::round(value)))
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

struct Sound;

impl Sound {
	fn sound(&self, k: &KeyRc) -> Option<SoundInfo> {
		k.client.sound(&k.s_str("sound"))
	}
}

#[async_trait(?Send)]
impl Behavior for Sound {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let show_name = k.s_truthy("showName");
		let name_at = k.s_str("nameAt");
		let id = k.s_str("sound");
		let Some(sound) = self.sound(k) else {
			let title = if show_name { id } else { String::new() };
			k.own_title(&title);
			k.set_image(&glyph_key("sound", &GlyphKey { color: colors::GRAY, title: &title, title_at: &name_at, ..Default::default() }));
			return;
		};
		let title = if show_name { sound.name.clone() } else { String::new() };
		k.own_title(&title);
		let playing = k.client.playing().contains(&id);
		let png = k.picture(&sound.icon);
		k.set_image(&picture_key(&PictureKey {
			png: png.as_deref(),
			fallback: "sound",
			title: &title,
			title_at: &name_at,
			frame: (k.s_truthy("showPlaying") && playing).then_some(Frame::On),
			..Default::default()
		}));
	}

	async fn on_press(&self, k: &KeyRc, args: Option<PressArgs>) -> Res<()> {
		let Some(sound) = self.sound(k) else {
			return Err(Error::from(if k.s_truthy("sound") { "This sound is gone from Voicenux" } else { "no sound selected" }));
		};
		match k.op(&args).as_str() {
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
	press_only: &[],
	can_repeat: false,
	required: &["sound"],
	behavior: &Sound,
	new_state: || Box::new(()),
};

// ----- A random sound. It picks the next one ahead, so the key can show it ("next"), or shows a -----
// dice. Never the same sound twice in a row. look: "dice", or "next" (the picture of the sound the
// next press plays).

#[derive(Default)]
struct RandomPick {
	next: Option<String>,
	last: Option<String>,
}

struct RandomSound;

impl RandomSound {
	// The sound the next press plays.
	fn next(&self, k: &KeyRc) -> Option<SoundInfo> {
		let next = k.x::<RandomPick>().next.clone();
		if let Some(sound) = next.and_then(|id| k.client.sound(&id)) {
			return Some(sound);
		}
		self.pick(k);
		let next = k.x::<RandomPick>().next.clone();
		next.and_then(|id| k.client.sound(&id))
	}

	fn pick(&self, k: &KeyRc) {
		let sounds = k.client.sounds();
		let mut x = k.x::<RandomPick>();
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
		let sound = self.next(k);
		let Some(sound) = sound.filter(|_| k.s_str("look") == "next") else { return k.glyph_for_title("", "below") };
		let title = if k.s_truthy("showName") { sound.name.clone() } else { String::new() };
		k.own_title(&title);
		let png = k.picture(&sound.icon);
		k.set_image(&picture_key(&PictureKey {
			png: png.as_deref(),
			fallback: "sound",
			title: &title,
			title_at: &k.s_str("nameAt"),
			..Default::default()
		}));
	}

	async fn on_press(&self, k: &KeyRc, _args: Option<PressArgs>) -> Res<()> {
		let sound = self.next(k).ok_or("The soundboard has no sounds")?;
		k.x::<RandomPick>().last = Some(sound.id.clone());
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
	press_key: "",
	ops: &[],
	press_only: &[],
	can_repeat: false,
	required: &[],
	behavior: &RandomSound,
	new_state: || Box::new(RandomPick::default()),
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
	}

	async fn on_press(&self, k: &KeyRc, _args: Option<PressArgs>) -> Res<()> {
		k.call("stop-sounds", None).await.map(|_| ())
	}
}

pub static STOP_SOUNDS: Kind = Kind {
	uuid: "de.felitendo.voicenux.stopsounds",
	name: "StopSounds",
	defaults: || defaults(json!({})),
	press_key: "",
	ops: &[],
	press_only: &[],
	can_repeat: false,
	required: &[],
	behavior: &StopSounds,
	new_state: || Box::new(()),
};

// ----- Volume of the soundboard: up, down or to a value. Hold keeps going by default. -----

struct SoundVolume;

impl SoundVolume {
	fn glyph_name(&self, k: &KeyRc) -> &'static str {
		volume_glyph(&k.gesture("press")).unwrap_or("volume")
	}
}

#[async_trait(?Send)]
impl Behavior for SoundVolume {
	fn on_change(&self, k: &KeyRc, what: &str) {
		if what == "state" {
			self.render(k);
		}
	}

	fn render_offline(&self, k: &KeyRc) {
		k.set_image(&glyph_key(self.glyph_name(k), &GlyphKey::default()));
	}

	fn render(&self, k: &KeyRc) {
		let title = match k.vn().sound_volume {
			Some(volume) if k.s_truthy("showValue") => format!("{}%", fmt(volume)),
			_ => String::new(),
		};
		k.own_title(&title);
		k.set_image(&glyph_key(self.glyph_name(k), &GlyphKey { title: &title, title_at: &k.s_str("valueAt"), ..Default::default() }));
	}

	async fn on_press(&self, k: &KeyRc, args: Option<PressArgs>) -> Res<()> {
		let mode = k.op(&args);
		let now = k.vn().sound_volume.unwrap_or(0.0);
		if mode == "set" {
			if !args.is_some_and(|a| a.repeat) {
				k.call("sound-volume", Some(&fmt(clamp(k.s_num("value"))))).await?;
			}
		} else {
			let step = js::number_or(k.s("step").as_ref(), 1.0).abs();
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
	press_only: &[],
	can_repeat: true,
	required: &[],
	behavior: &SoundVolume,
	new_state: || Box::new(()),
};

// ----- Stream Deck + dial: turn for the soundboard volume, press or touch to stop all sounds. -----
// pressAction: "stop" all sounds, "hear" (hear the sounds yourself, on or off), or "nothing".

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
		let step = js::number(k.s("step").as_ref()).abs();
		let next = clamp(now + ticks * if js::truthy_num(step) { step } else { 1.0 });
		if next != now {
			k.call("sound-volume", Some(&fmt(next))).await?;
		}
		self.render(k);
		Ok(())
	}

	async fn on_dial_down(&self, k: &KeyRc) -> Res<()> {
		match k.s_str("pressAction").as_str() {
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
	press_key: "",
	ops: &[],
	press_only: &[],
	can_repeat: false,
	required: &[],
	behavior: &SoundVolumeDial,
	new_state: || Box::new(()),
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::testing::*;

	fn args(op: &str) -> Option<PressArgs> {
		Some(PressArgs { op: op.into(), gesture: "press".into(), repeat: false })
	}

	#[tokio::test]
	async fn a_sound_key_plays_in_its_own_mode_also_with_an_older_voicenux() {
		local(async {
			let (_p, k) = key(&SOUND, json!({ "sound": "airhorn", "press": "overlap" }));
			Sound.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["sound-overlap", "airhorn"]);
			Sound.on_press(&k, args("stop")).await.unwrap();
			assert_eq!(last(&k), ["stop-sound", "airhorn"]);
			// Voicenux 0.1.0 knows only "sound": toggle and restart still work, on top can not.
			let (_p, old) = key(&SOUND, json!({ "sound": "airhorn", "press": "toggle" }));
			old_voicenux(&old);
			Sound.on_press(&old, None).await.unwrap();
			assert_eq!(last(&old), ["sound", "airhorn"]);
			old.client.set_playing(vec!["airhorn".into()]);
			Sound.on_press(&old, None).await.unwrap();
			assert_eq!(last(&old), ["stop-sound", "airhorn"]);
			assert_eq!(Sound.on_press(&old, args("overlap")).await, Err(Error::from("Play on top needs a newer Voicenux")));
		})
		.await;
	}

	#[tokio::test]
	async fn a_key_without_its_sound_says_why() {
		local(async {
			let (_p, k) = key(&SOUND, json!({ "sound": "" }));
			assert_eq!(Sound.on_press(&k, None).await, Err(Error::from("no sound selected")));
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
				RandomSound.on_press(&k, None).await.unwrap();
				assert_eq!(last(&k), ["sound-restart".to_string(), shown.clone()]);
				assert_ne!(RandomSound.next(&k).map(|s| s.id), Some(shown), "the same sound twice in a row");
			}
			k.client.set_sounds(Vec::new());
			assert_eq!(RandomSound.on_press(&k, None).await, Err(Error::from("The soundboard has no sounds")));
		})
		.await;
	}

	#[tokio::test]
	async fn soundboard_volume_stays_between_0_and_100() {
		local(async {
			let (_p, k) = key(&SOUND_VOLUME, json!({ "step": 15 }));
			assert_eq!(k.gesture("hold"), "repeat");
			SoundVolume.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["sound-volume", "95"]);
			SoundVolume.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["sound-volume", "100"]);
			let count = calls(&k).len();
			SoundVolume.on_press(&k, None).await.unwrap();
			assert_eq!(calls(&k).len(), count, "no call when it is at 100 already");
			SoundVolume.on_press(&k, args("set")).await.unwrap();
			assert_eq!(last(&k), ["sound-volume", "50"]);
		})
		.await;
	}

	#[test]
	fn random_numbers_stay_below_one() {
		assert!((0..100).map(|_| random()).all(|r| (0.0..1.0).contains(&r)));
	}
}
