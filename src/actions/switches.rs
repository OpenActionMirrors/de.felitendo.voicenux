use async_trait::async_trait;
use serde_json::{Map, Value, json};

use super::{Behavior, KeyRc, Kind, PressArgs, defaults};
use crate::Res;
use crate::i18n::t;
use crate::voicenux::State;

fn switch_defaults() -> Map<String, Value> {
	defaults(json!({ "mode": "toggle", "hold": "none", "double": "none", "showState": false, "stateAt": "below" }))
}

// ----- A switch of Voicenux: on, off or toggle. White when on, gray when off. -----
// holdOn: on only while the key is held, like push to talk. holdOff: off only while held.

struct Switch {
	/// Its value in the state.
	field: fn(&State) -> bool,
	/// Its action in the API.
	api: &'static str,
}

impl Switch {
	fn is_on(&self, k: &KeyRc) -> bool {
		(self.field)(&k.vn())
	}
}

#[async_trait(?Send)]
impl Behavior for Switch {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let on = self.is_on(k);
		k.set_state(u16::from(on));
		let title = if k.s_truthy("showState") { t(if on { "On" } else { "Off" }) } else { String::new() };
		k.glyph_for_title(&title, &k.s_str("stateAt"));
	}

	async fn on_press(&self, k: &KeyRc, args: Option<PressArgs>) -> Res<()> {
		let op = k.op(&args);
		let arg = match op.as_str() {
			"holdOn" => "on",
			"holdOff" => "off",
			op => op,
		};
		k.call(self.api, Some(arg)).await?;
		self.render(k);
		Ok(())
	}

	async fn on_release(&self, k: &KeyRc) -> Res<()> {
		if k.is_in_multi_action() {
			return Ok(());
		}
		let mode = k.gesture("press");
		if mode != "holdOn" && mode != "holdOff" {
			return Ok(());
		}
		k.call(self.api, Some(if mode == "holdOn" { "off" } else { "on" })).await?;
		self.render(k);
		Ok(())
	}
}

const SWITCH_OPS: &[&str] = &["toggle", "on", "off", "holdOn", "holdOff"];
const HELD: &[&str] = &["holdOn", "holdOff"];

pub static VOICE_CHANGER: Kind = Kind {
	uuid: "de.felitendo.voicenux.voicechanger",
	name: "VoiceChanger",
	defaults: switch_defaults,
	press_key: "mode",
	ops: SWITCH_OPS,
	press_only: HELD,
	can_repeat: false,
	required: &[],
	behavior: &Switch { field: |s| s.voice_changer, api: "voice-changer" },
	new_state: || Box::new(()),
};

pub static HEAR_MYSELF: Kind = Kind {
	uuid: "de.felitendo.voicenux.hearmyself",
	name: "HearMyself",
	defaults: switch_defaults,
	press_key: "mode",
	ops: SWITCH_OPS,
	press_only: HELD,
	can_repeat: false,
	required: &[],
	behavior: &Switch { field: |s| s.hear_myself, api: "hear-myself" },
	new_state: || Box::new(()),
};

pub static BACKGROUND: Kind = Kind {
	uuid: "de.felitendo.voicenux.background",
	name: "Background",
	defaults: switch_defaults,
	press_key: "mode",
	ops: SWITCH_OPS,
	press_only: HELD,
	can_repeat: false,
	required: &[],
	behavior: &Switch { field: |s| s.background, api: "background" },
	new_state: || Box::new(()),
};

pub static HEAR_SOUNDS: Kind = Kind {
	uuid: "de.felitendo.voicenux.hearsounds",
	name: "HearSounds",
	defaults: switch_defaults,
	press_key: "mode",
	ops: SWITCH_OPS,
	press_only: HELD,
	can_repeat: false,
	required: &[],
	behavior: &Switch { field: |s| s.hear_sounds, api: "hear-sounds" },
	new_state: || Box::new(()),
};

// ----- Mutes the microphone. Red when muted. -----
// talk: live only while the key is held (push to talk). holdMute: muted only while held.

struct Mute;

impl Mute {
	async fn set_muted(&self, k: &KeyRc, muted: bool) -> Res<()> {
		k.call("mute", Some(if muted { "on" } else { "off" })).await?;
		self.render(k);
		Ok(())
	}
}

#[async_trait(?Send)]
impl Behavior for Mute {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let muted = k.vn().mute;
		k.set_state(u16::from(muted));
		let title = if k.s_truthy("showState") { t(if muted { "Muted" } else { "Live" }) } else { String::new() };
		k.glyph_for_title(&title, &k.s_str("stateAt"));
	}

	async fn on_press(&self, k: &KeyRc, args: Option<PressArgs>) -> Res<()> {
		match k.op(&args).as_str() {
			"toggle" => {
				k.call("mute", Some("toggle")).await?;
			}
			"talk" => self.set_muted(k, false).await?,
			"holdMute" => self.set_muted(k, true).await?,
			mode => self.set_muted(k, mode == "mute").await?,
		}
		self.render(k);
		Ok(())
	}

	async fn on_release(&self, k: &KeyRc) -> Res<()> {
		if k.is_in_multi_action() {
			return Ok(());
		}
		match k.gesture("press").as_str() {
			"talk" => self.set_muted(k, true).await,
			"holdMute" => self.set_muted(k, false).await,
			_ => Ok(()),
		}
	}
}

pub static MUTE: Kind = Kind {
	uuid: "de.felitendo.voicenux.mute",
	name: "Mute",
	defaults: switch_defaults,
	press_key: "mode",
	ops: &["toggle", "mute", "unmute", "talk", "holdMute"],
	press_only: &["talk", "holdMute"],
	can_repeat: false,
	required: &[],
	behavior: &Mute,
	new_state: || Box::new(()),
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::testing::*;

	#[tokio::test]
	async fn switches_and_mute_send_on_off_or_toggle() {
		local(async {
			let (_p, changer) = key(&VOICE_CHANGER, json!({ "mode": "off" }));
			VOICE_CHANGER.behavior.on_press(&changer, None).await.unwrap();
			assert_eq!(last(&changer), ["voice-changer", "off"]);
			let (_p, mute) = key(&MUTE, json!({ "mode": "talk" }));
			MUTE.behavior.on_press(&mute, None).await.unwrap();
			assert_eq!(last(&mute), ["mute", "off"]);
			MUTE.behavior.on_release(&mute).await.unwrap();
			assert_eq!(last(&mute), ["mute", "on"]);
			// Push to talk uses key up, so it has no hold or double press.
			let (_p, talk) = key(&MUTE, json!({ "mode": "talk", "hold": "mute" }));
			assert_eq!(talk.gesture("hold"), "none");
		})
		.await;
	}

	#[tokio::test]
	async fn push_to_change_voice_on_while_held_off_when_let_go() {
		local(async {
			let (_p, k) = key(&VOICE_CHANGER, json!({ "mode": "holdOn", "hold": "off" }));
			k.client.set_state(|s| s.voice_changer = false);
			// It uses key down and up, so there is no hold or double press.
			assert_eq!(k.gesture("hold"), "none");
			VOICE_CHANGER.behavior.on_press(&k, None).await.unwrap();
			assert_eq!(last(&k), ["voice-changer", "on"]);
			VOICE_CHANGER.behavior.on_release(&k).await.unwrap();
			assert_eq!(last(&k), ["voice-changer", "off"]);
			let (_p, own) = key(&VOICE_CHANGER, json!({ "mode": "holdOff" }));
			VOICE_CHANGER.behavior.on_press(&own, None).await.unwrap();
			VOICE_CHANGER.behavior.on_release(&own).await.unwrap();
			let calls: Vec<_> = calls(&own).into_iter().map(|(a, arg)| (a, arg.unwrap_or_default())).collect();
			assert_eq!(calls, [("voice-changer".to_string(), "off".to_string()), ("voice-changer".to_string(), "on".to_string())]);
		})
		.await;
	}
}
