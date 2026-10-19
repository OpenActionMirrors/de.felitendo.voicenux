use async_trait::async_trait;
use serde_json::json;
use tokio::task::spawn_local;

use super::{Behavior, KeyRc, Kind, PressArgs, defaults};
use crate::i18n::t;
use crate::icons::colors;
use crate::render::{Frame, GlyphKey, PictureKey, dial_picture, glyph_key, picture_key};
use crate::timers::Timer;
use crate::voicenux::Voice as VoiceInfo;
use crate::{Error, Res};

// How long a dial shows a picked voice before it goes back to the one in use.
const PICK_TIMEOUT: f64 = 6000.0;

// ----- One voice. Press uses it, and the key has a frame while it is in use. -----
// toggle: this voice, or your own voice when it is in use. use: always this voice.
// whileHeld: this voice while the key is held, then back to the voice from before. favorite: star it.

#[derive(Default)]
struct VoiceState {
	// The voice in use and the voice changer before a "while held" press.
	before: Option<(Option<String>, bool)>,
}

struct Voice;

impl Voice {
	fn voice(&self, k: &KeyRc) -> Option<VoiceInfo> {
		k.client.voice(&k.s_str("voice"))
	}
}

#[async_trait(?Send)]
impl Behavior for Voice {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let show_name = k.s_truthy("showName");
		let name_at = k.s_str("nameAt");
		let Some(voice) = self.voice(k) else {
			// Deleted in Voicenux, or not loaded yet.
			let title = if show_name { k.s_str("voice") } else { String::new() };
			k.own_title(&title);
			k.set_image(&glyph_key("voice", &GlyphKey { color: colors::GRAY, title: &title, title_at: &name_at, ..Default::default() }));
			return;
		};
		let vn = k.vn();
		let in_use = vn.voice_id() == Some(voice.id.as_str());
		let frame = (k.s_truthy("showFrame") && in_use).then_some(if vn.voice_changer { Frame::On } else { Frame::Off });
		let title = if show_name { voice.name.clone() } else { String::new() };
		k.own_title(&title);
		let png = k.picture(&voice.icon);
		k.set_image(&picture_key(&PictureKey { png: png.as_deref(), title: &title, title_at: &name_at, frame, ..Default::default() }));
	}

	async fn on_press(&self, k: &KeyRc, args: Option<PressArgs>) -> Res<()> {
		let Some(voice) = self.voice(k) else {
			return Err(Error::from(if k.s_truthy("voice") { "This voice is gone from Voicenux" } else { "no voice selected" }));
		};
		let op = k.op(&args);
		let vn = k.vn();
		let multi = k.is_in_multi_action();
		if op == "whileHeld" && !multi {
			k.x::<VoiceState>().before = Some((vn.voice_id().map(String::from), vn.voice_changer));
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
		let before = k.x::<VoiceState>().before.take();
		let Some((id, on)) = before else { return Ok(()) };
		if k.gesture("press") != "whileHeld" {
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
	can_repeat: false,
	required: &["voice"],
	behavior: &Voice,
	new_state: || Box::new(VoiceState::default()),
};

// ----- Next, previous or a random voice. Can show the voice in use. -----

struct Step;

impl Step {
	fn glyph_name(&self, k: &KeyRc) -> &'static str {
		match k.gesture("press").as_str() {
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
		let vn = k.vn();
		let name_at = k.s_str("nameAt");
		let title = match &vn.voice {
			Some(voice) if k.s_truthy("showName") => voice.name.clone(),
			_ => String::new(),
		};
		k.own_title(&title);
		match &vn.voice {
			Some(voice) if k.s_str("look") == "voice" => {
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

	async fn on_press(&self, k: &KeyRc, args: Option<PressArgs>) -> Res<()> {
		let action = match k.op(&args).as_str() {
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
	press_only: &[],
	can_repeat: false,
	required: &[],
	behavior: &Step,
	new_state: || Box::new(()),
};

pub static RANDOM_VOICE: Kind = Kind {
	uuid: "de.felitendo.voicenux.randomvoice",
	name: "RandomVoice",
	defaults: || {
		defaults(json!({ "direction": "random", "hold": "none", "double": "none", "look": "icon", "showName": true, "nameAt": "below" }))
	},
	press_key: "direction",
	ops: &["random", "next", "previous"],
	press_only: &[],
	can_repeat: false,
	required: &[],
	behavior: &Step,
	new_state: || Box::new(()),
};

// ----- Stream Deck + dial: turn to pick a voice (with its picture), press to use it. -----
// Pressed while it shows the voice in use, it turns the voice changer on or off.
// list: "all" voices or only "favorites". mode: "pick" (press to use) or "direct" (turning uses it).

#[derive(Default)]
struct DialState {
	selected: Option<String>,
	pick_timer: Option<Timer>,
	switch_timer: Option<Timer>,
}

fn dial(k: &KeyRc) -> std::cell::RefMut<'_, DialState> {
	k.x::<DialState>()
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
		if k.s_str("list") == "favorites" {
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

	fn on_change(&self, k: &KeyRc, what: &str) {
		if what == "voices" {
			let selected = dial(k).selected.clone();
			if selected.is_some_and(|id| k.client.voice(&id).is_none()) {
				dial(k).selected = None;
			}
		}
		self.render(k);
	}

	fn render_offline(&self, k: &KeyRc) {
		k.set_feedback(json!({ "title": t("Voice"), "name": "", "info": k.dial_status(), "picture": dial_picture(None) }));
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
			"picture": dial_picture(png.as_deref()),
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
		if k.s_str("mode") == "direct" {
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
	press_key: "",
	ops: &[],
	press_only: &[],
	can_repeat: false,
	required: &[],
	behavior: &VoiceDial,
	new_state: || Box::new(DialState::default()),
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::testing::*;

	#[tokio::test]
	async fn a_voice_key_uses_its_voice_or_goes_back_to_your_own_voice() {
		local(async {
			let (_p, k) = key(&VOICE, json!({ "voice": "deep", "press": "toggle" }));
			Voice.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["voice", "deep"]);
			Voice.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["voice-changer", "off"]);
			Voice.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["voice", "deep"]);
			let favorite = PressArgs { op: "favorite".into(), gesture: "press".into(), repeat: false };
			Voice.on_press(&k, Some(favorite)).await.unwrap();
			assert_eq!(last(&k), ["favorite", "deep"]);
		})
		.await;
	}

	#[tokio::test]
	async fn a_key_without_its_voice_says_why() {
		local(async {
			let (_p, k) = key(&VOICE, json!({ "voice": "gone" }));
			assert_eq!(Voice.on_press(&k, None).await, Err(Error::from("This voice is gone from Voicenux")));
		})
		.await;
	}

	#[tokio::test]
	async fn gestures_fall_back_to_the_first_choice() {
		local(async {
			assert_eq!(key(&VOICE, json!({})).1.gesture("press"), "toggle");
			assert_eq!(key(&VOICE, json!({ "press": "nope" })).1.gesture("press"), "toggle");
			assert_eq!(key(&VOICE, json!({ "hold": "favorite" })).1.gesture("hold"), "favorite");
			assert_eq!(key(&RANDOM_VOICE, json!({})).1.gesture("press"), "random");
		})
		.await;
	}

	#[tokio::test]
	async fn a_voice_used_while_held_gives_the_voice_from_before_back() {
		local(async {
			let (_p, k) = key(&VOICE, json!({ "voice": "deep", "press": "whileHeld" }));
			k.client.set_state(|s| s.voice_changer = false);
			Voice.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["voice", "deep"]);
			Voice.on_release(&k).await.unwrap();
			let tail: Vec<_> = calls(&k).into_iter().skip(1).collect();
			assert_eq!(tail, [("voice".into(), Some("robot".into())), ("voice-changer".into(), Some("off".into()))]);
			// A short press of another kind of voice key leaves the voice as it is.
			let (_p, other) = key(&VOICE, json!({ "voice": "deep", "press": "use" }));
			Voice.on_press(&other, None).await.unwrap();
			Voice.on_release(&other).await.unwrap();
			assert_eq!(calls(&other), [("voice".into(), Some("deep".into()))]);
		})
		.await;
	}

	#[tokio::test]
	async fn next_voice_asks_voicenux_for_the_previous_one() {
		local(async {
			let (_p, k) = key(&NEXT_VOICE, json!({ "direction": "previous" }));
			Step.on_press(&k, None).await.unwrap();
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
