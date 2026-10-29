// Voices: one voice, the next, previous or a random one, and the voice dial.
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::task::spawn_local;

use super::{Behavior, Gesture, KeyRc, Kind, Press, defaults};
use crate::i18n::t;
use crate::icons::colors;
use crate::render::{Frame, GlyphKey, PictureKey, dial_picture, glyph_key, picture_key};
use crate::timers::Timer;
use crate::voicenux::Change;
use crate::voicenux::Voice as VoiceInfo;
use crate::{Error, Res};

// How long a dial shows a picked voice before it goes back to the one in use.
const PICK_TIMEOUT: f64 = 6000.0;

// ----- One voice. Press uses it, and the key has a frame while it is in use. -----
// toggle: this voice, or your own voice when it is in use. use: always this voice.
// whileHeld: this voice while the key is held, then back to the voice from before. favorite: star it.

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct VoiceSettings {
	/// The id of the voice.
	voice: String,
	show_name: bool,
	/// "below" or "above".
	name_at: String,
	/// A frame while the voice is in use.
	show_frame: bool,
}

#[derive(Default)]
struct VoiceState {
	// The voice in use and the voice changer before a "while held" press.
	before: Option<(Option<String>, bool)>,
}

struct Voice;

#[async_trait(?Send)]
impl Behavior for Voice {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let s: VoiceSettings = k.settings();
		let Some(voice) = k.client.voice(&s.voice) else {
			// Deleted in Voicenux, or not loaded yet.
			let title = if s.show_name { s.voice.as_str() } else { "" };
			k.own_title(title);
			k.set_image(&glyph_key("voice", &GlyphKey { color: colors::GRAY, title, title_at: &s.name_at, ..Default::default() }));
			return;
		};
		let vn = k.vn();
		let in_use = vn.voice_id() == Some(voice.id.as_str());
		let frame = (s.show_frame && in_use).then_some(if vn.voice_changer { Frame::On } else { Frame::Off });
		let title = if s.show_name { voice.name.as_str() } else { "" };
		k.own_title(title);
		let png = k.picture(&voice.icon);
		k.set_image(&picture_key(&PictureKey { png: png.as_deref(), title, title_at: &s.name_at, frame, ..Default::default() }));
	}

	async fn on_press(&self, k: &KeyRc, press: Press) -> Res<()> {
		let id = k.settings::<VoiceSettings>().voice;
		let Some(voice) = k.client.voice(&id) else {
			return Err(Error::from(if id.is_empty() { "no voice selected" } else { "This voice is gone from Voicenux" }));
		};
		let op = press.op;
		let vn = k.vn();
		let multi = k.is_in_multi_action();
		if op == "whileHeld" && !multi {
			k.own::<VoiceState>().before = Some((vn.voice_id().map(String::from), vn.voice_changer));
		}
		if op == "favorite" {
			k.call("favorite", Some(&voice.id)).await?;
			k.show_ok();
		} else if op == "toggle" && vn.voice_id() == Some(voice.id.as_str()) && vn.voice_changer && !multi {
			k.call("voice-changer", Some("off")).await?;
		} else {
			k.call("voice", Some(&voice.id)).await?;
		}
		self.render(k);
		Ok(())
	}

	async fn on_release(&self, k: &KeyRc) -> Res<()> {
		let before = k.own::<VoiceState>().before.take();
		let Some((id, on)) = before else { return Ok(()) };
		if k.gesture(Gesture::Press) != "whileHeld" {
			return Ok(());
		}
		if let Some(id) = id.filter(|id| !id.is_empty() && k.vn().voice_id() != Some(id.as_str())) {
			k.call("voice", Some(&id)).await?;
		}
		if !on {
			k.call("voice-changer", Some("off")).await?;
		}
		self.render(k);
		Ok(())
	}
}

pub static VOICE: Kind = Kind {
	uuid: "de.felitendo.voicenux.voice",
	name: "Voice",
	defaults: || {
		defaults(
			json!({ "voice": "", "press": "toggle", "hold": "none", "double": "none", "showName": true, "nameAt": "below", "showFrame": true }),
		)
	},
	press_key: "press",
	ops: &["toggle", "use", "whileHeld", "favorite"],
	press_only: &["whileHeld"],
	required: &["voice"],
	behavior: &Voice,
	new_state: || Box::new(VoiceState::default()),
	..Kind::BASE
};

// ----- Next, previous or a random voice. Can show the voice in use. -----

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct StepSettings {
	/// "icon" (an arrow or a dice) or "voice" (the picture of the voice in use).
	look: String,
	show_name: bool,
	name_at: String,
}

struct Step;

impl Step {
	fn glyph_name(&self, k: &KeyRc) -> &'static str {
		match k.gesture(Gesture::Press).as_str() {
			"previous" => "previous",
			"random" => "random",
			_ => "next",
		}
	}
}

#[async_trait(?Send)]
impl Behavior for Step {
	fn render_offline(&self, k: &KeyRc) {
		k.set_image(&glyph_key(self.glyph_name(k), &GlyphKey::default()));
	}

	fn render(&self, k: &KeyRc) {
		let s: StepSettings = k.settings();
		let vn = k.vn();
		let name_at = s.name_at;
		let title = match &vn.voice {
			Some(voice) if s.show_name => voice.name.clone(),
			_ => String::new(),
		};
		k.own_title(&title);
		match &vn.voice {
			Some(voice) if s.look == "voice" => {
				let png = k.picture(&voice.icon);
				k.set_image(&picture_key(&PictureKey {
					png: png.as_deref(),
					title: &title,
					title_at: &name_at,
					dim: !vn.voice_changer,
					..Default::default()
				}));
			}
			_ => k.set_image(&glyph_key(self.glyph_name(k), &GlyphKey { title: &title, title_at: &name_at, ..Default::default() })),
		}
	}

	async fn on_press(&self, k: &KeyRc, press: Press) -> Res<()> {
		let action = match press.op.as_str() {
			"random" => "random-voice",
			"previous" => "previous-voice",
			_ => "next-voice",
		};
		k.call(action, None).await?;
		self.render(k);
		Ok(())
	}
}

pub static NEXT_VOICE: Kind = Kind {
	uuid: "de.felitendo.voicenux.nextvoice",
	name: "NextVoice",
	defaults: || {
		defaults(json!({ "direction": "next", "hold": "none", "double": "none", "look": "icon", "showName": true, "nameAt": "below" }))
	},
	press_key: "direction",
	ops: &["next", "previous", "random"],
	behavior: &Step,
	..Kind::BASE
};

pub static RANDOM_VOICE: Kind = Kind {
	uuid: "de.felitendo.voicenux.randomvoice",
	name: "RandomVoice",
	defaults: || {
		defaults(json!({ "direction": "random", "hold": "none", "double": "none", "look": "icon", "showName": true, "nameAt": "below" }))
	},
	press_key: "direction",
	ops: &["random", "next", "previous"],
	behavior: &Step,
	..Kind::BASE
};

// ----- Stream Deck + dial: turn to pick a voice (with its picture), press to use it. -----
// Pressed while it shows the voice in use, it turns the voice changer on or off.
// list: "all" voices or only "favorites". mode: "pick" (press to use) or "direct" (turning uses it).

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct VoiceDialSettings {
	list: String,
	mode: String,
}

#[derive(Default)]
struct DialState {
	selected: Option<String>,
	pick_timer: Option<Timer>,
	switch_timer: Option<Timer>,
}

fn dial(k: &KeyRc) -> std::cell::RefMut<'_, DialState> {
	k.own::<DialState>()
}

fn clear(timer: Option<Timer>) {
	if let Some(timer) = timer {
		timer.clear();
	}
}

struct VoiceDial;

impl VoiceDial {
	// The voices to turn through. Favorites only when there are some.
	fn list(&self, k: &KeyRc) -> Vec<VoiceInfo> {
		let all = k.client.voices();
		if k.settings::<VoiceDialSettings>().list == "favorites" {
			let favorites: Vec<VoiceInfo> = all.iter().filter(|v| v.favorite).cloned().collect();
			if !favorites.is_empty() {
				return favorites;
			}
		}
		all
	}

	fn target(&self, k: &KeyRc) -> Option<VoiceInfo> {
		let selected = dial(k).selected.clone();
		selected.and_then(|id| k.client.voice(&id)).or_else(|| k.vn().voice)
	}

	async fn use_voice(&self, k: &KeyRc, id: &str) -> Res<()> {
		k.call("voice", Some(id)).await?;
		{
			let mut d = dial(k);
			clear(d.pick_timer.take());
			d.selected = None;
		}
		self.render(k);
		Ok(())
	}
}

#[async_trait(?Send)]
impl Behavior for VoiceDial {
	fn layout(&self) -> Option<&'static str> {
		Some("layouts/voice-dial.json")
	}

	fn on_change(&self, k: &KeyRc, change: Change) {
		if change == Change::Voices {
			let selected = dial(k).selected.clone();
			if selected.is_some_and(|id| k.client.voice(&id).is_none()) {
				dial(k).selected = None;
			}
		}
		self.render(k);
	}

	fn render_offline(&self, k: &KeyRc) {
		k.set_feedback(json!({ "title": t("Voice"), "name": "", "info": k.dial_status(), "picture": dial_picture(None, "voice") }));
	}

	fn render(&self, k: &KeyRc) {
		let Some(voice) = self.target(k) else { return self.render_offline(k) };
		let list = self.list(k);
		let vn = k.vn();
		let in_use = vn.voice_id() == Some(voice.id.as_str());
		let on = vn.voice_changer;
		let mut info = match list.iter().position(|v| v.id == voice.id) {
			Some(index) => format!("{} / {}", index + 1, list.len()),
			None => String::new(),
		};
		if in_use {
			info = t(if on { "In use" } else { "Voice changer off" });
		}
		let picked = dial(k).selected.is_some();
		let png = k.picture(&voice.icon);
		k.set_feedback(json!({
			"title": t(if picked && !in_use { "Press to use" } else { "Voice" }),
			"name": { "value": voice.name, "color": if in_use && !on { colors::MUTED } else { colors::WHITE } },
			"info": { "value": info, "color": if in_use && on { colors::CYAN } else { colors::MUTED } },
			"picture": dial_picture(png.as_deref(), "voice"),
		}));
	}

	async fn on_dial_rotate(&self, k: &KeyRc, ticks: f64) -> Res<()> {
		let list = self.list(k);
		if list.is_empty() {
			return Ok(());
		}
		let target = self.target(k).map(|v| v.id);
		let from = list.iter().position(|v| Some(&v.id) == target.as_ref()).unwrap_or(0) as i64;
		let len = list.len() as i64;
		let next = list[((from + ticks as i64) % len + len) as usize % list.len()].id.clone();
		{
			let mut d = dial(k);
			clear(d.pick_timer.take());
			d.selected = Some(next.clone());
		}
		self.render(k);
		if k.settings::<VoiceDialSettings>().mode == "direct" {
			// Wait a moment, so a fast turn does not switch through every voice.
			let timer = k.after(250.0, move |k| {
				spawn_local(async move {
					k.act(VoiceDial.use_voice(&k, &next)).await;
				});
			});
			clear(dial(k).switch_timer.replace(timer));
			return Ok(());
		}
		let timer = k.after(PICK_TIMEOUT, |k| {
			dial(&k).selected = None;
			VoiceDial.render(&k);
		});
		dial(k).pick_timer = Some(timer);
		Ok(())
	}

	async fn on_dial_down(&self, k: &KeyRc) -> Res<()> {
		let Some(voice) = self.target(k) else { return Ok(()) };
		if k.vn().voice_id() == Some(voice.id.as_str()) {
			{
				let mut d = dial(k);
				clear(d.pick_timer.take());
				d.selected = None;
			}
			k.call("voice-changer", Some("toggle")).await?;
			self.render(k);
			return Ok(());
		}
		self.use_voice(k, &voice.id).await
	}

	fn on_disappear(&self, k: &KeyRc) {
		let mut d = dial(k);
		clear(d.pick_timer.take());
		clear(d.switch_timer.take());
	}
}

pub static VOICE_DIAL: Kind = Kind {
	uuid: "de.felitendo.voicenux.voicedial",
	name: "VoiceDial",
	defaults: || defaults(json!({ "list": "all", "mode": "pick", "disableTouch": false })),
	behavior: &VoiceDial,
	new_state: || Box::new(DialState::default()),
	..Kind::BASE
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::testing::*;
	use crate::settings::testing::assert_defaults;

	#[test]
	fn every_setting_has_a_default() {
		assert_defaults::<VoiceSettings>(&[&VOICE]);
		assert_defaults::<StepSettings>(&[&NEXT_VOICE]);
		assert_defaults::<StepSettings>(&[&RANDOM_VOICE]);
		assert_defaults::<VoiceDialSettings>(&[&VOICE_DIAL]);
	}

	#[tokio::test]
	async fn a_voice_key_uses_its_voice_or_goes_back_to_your_own_voice() {
		local(async {
			let (_p, k) = key(&VOICE, json!({ "voice": "deep", "press": "toggle" }));
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&k), ["voice", "deep"]);
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&k), ["voice-changer", "off"]);
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&k), ["voice", "deep"]);
			Voice.on_press(&k, Press { op: "favorite".into(), repeat: false }).await.unwrap();
			assert_eq!(last(&k), ["favorite", "deep"]);
		})
		.await;
	}

	#[tokio::test]
	async fn a_key_without_its_voice_says_why() {
		local(async {
			let (_p, k) = key(&VOICE, json!({ "voice": "gone" }));
			assert_eq!(k.press(Gesture::Press, false).await, Err(Error::from("This voice is gone from Voicenux")));
		})
		.await;
	}

	#[tokio::test]
	async fn gestures_fall_back_to_the_first_choice() {
		local(async {
			assert_eq!(key(&VOICE, json!({})).1.gesture(Gesture::Press), "toggle");
			assert_eq!(key(&VOICE, json!({ "press": "nope" })).1.gesture(Gesture::Press), "toggle");
			assert_eq!(key(&VOICE, json!({ "hold": "favorite" })).1.gesture(Gesture::Hold), "favorite");
			assert_eq!(key(&RANDOM_VOICE, json!({})).1.gesture(Gesture::Press), "random");
		})
		.await;
	}

	#[tokio::test]
	async fn a_voice_used_while_held_gives_the_voice_from_before_back() {
		local(async {
			let (_p, k) = key(&VOICE, json!({ "voice": "deep", "press": "whileHeld" }));
			k.client.set_state(|s| s.voice_changer = false);
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(last(&k), ["voice", "deep"]);
			Voice.on_release(&k).await.unwrap();
			let tail: Vec<_> = calls(&k).into_iter().skip(1).collect();
			assert_eq!(tail, [("voice".into(), Some("robot".into())), ("voice-changer".into(), Some("off".into()))]);
			// A short press of another kind of voice key leaves the voice as it is.
			let (_p, other) = key(&VOICE, json!({ "voice": "deep", "press": "use" }));
			other.press(Gesture::Press, false).await.unwrap();
			Voice.on_release(&other).await.unwrap();
			assert_eq!(calls(&other), [("voice".into(), Some("deep".into()))]);
		})
		.await;
	}

	#[tokio::test]
	async fn next_voice_asks_voicenux_for_the_previous_one() {
		local(async {
			let (_p, k) = key(&NEXT_VOICE, json!({ "direction": "previous" }));
			k.press(Gesture::Press, false).await.unwrap();
			assert_eq!(calls(&k), [("previous-voice".into(), None)]);
		})
		.await;
	}

	#[tokio::test]
	async fn the_voice_dial_picks_first_and_uses_on_press() {
		local(async {
			let (_p, k) = key(&VOICE_DIAL, json!({ "list": "all" }));
			VoiceDial.on_dial_rotate(&k, 2.0).await.unwrap();
			assert_eq!(dial(&k).selected.as_deref(), Some("chipmunk"));
			assert!(calls(&k).is_empty());
			VoiceDial.on_dial_down(&k).await.unwrap();
			assert_eq!(last(&k), ["voice", "chipmunk"]);
			// Pressed on the voice in use: the voice changer goes off.
			VoiceDial.on_dial_down(&k).await.unwrap();
			assert_eq!(last(&k), ["voice-changer", "toggle"]);
			// Only favorites: there is just one.
			let (_p, favorites) = key(&VOICE_DIAL, json!({ "list": "favorites" }));
			VoiceDial.on_dial_rotate(&favorites, 1.0).await.unwrap();
			assert_eq!(dial(&favorites).selected.as_deref(), Some("deep"));
			VoiceDial.on_disappear(&k);
			VoiceDial.on_disappear(&favorites);
		})
		.await;
	}
}
