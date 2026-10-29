// The actions. Each one is a Kind: its settings, what press, hold and double press can do, and the
// hooks it fills in (Behavior). key/ does the rest for every key: connection, gestures, images.
mod key;
pub mod say;
pub mod sounds;
pub mod switches;
pub mod voices;

use std::any::Any;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Map, Value};

#[cfg(test)]
pub(crate) use key::testing;
pub use key::{Appearance, DOUBLE_PRESS_MS, Key, KeyRc, LONG_PRESS_MS};

use crate::Res;
use crate::voicenux::Change;

/// A key (Keypad) or a dial with its touch strip (Encoder).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Controller {
	#[default]
	Keypad,
	Encoder,
}

impl Controller {
	/// From the name OpenDeck uses.
	pub fn from_name(name: &str) -> Controller {
		if name == "Encoder" { Controller::Encoder } else { Controller::Keypad }
	}
}

/// The ways to press a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
	Press,
	Hold,
	Double,
}

impl Gesture {
	/// Also the setting that holds what hold and double press do.
	pub fn name(self) -> &'static str {
		match self {
			Gesture::Press => "press",
			Gesture::Hold => "hold",
			Gesture::Double => "double",
		}
	}
}

/// A press, once it is clear which gesture it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Press {
	/// What to do: one of the `ops` of the action, or "press" for an action without ops.
	pub op: String,
	/// Again and again while the key is held.
	pub repeat: bool,
}

/// An action: what all its keys have in common.
pub struct Kind {
	pub uuid: &'static str,
	/// For the log: "Voice: error: no voice selected".
	pub name: &'static str,
	pub defaults: fn() -> Map<String, Value>,
	/// Press, hold and double press. An action that can do more than one thing lists it in `ops`
	/// and keeps the press choice in the setting `press_key`. The settings `hold` and `double` pick
	/// from the same list. on_press gets the choice as `op`.
	pub press_key: &'static str,
	pub ops: &'static [&'static str],
	/// Press choices that use the key going down and up (push to talk): no hold or double press then.
	pub press_only: &'static [&'static str],
	/// Hold can also repeat the press while the key is held ("repeat").
	pub can_repeat: bool,
	/// Settings a key needs before it can work. Until then it shows a setup image.
	pub required: &'static [&'static str],
	pub behavior: &'static dyn Behavior,
	/// The action's own state, kept per key (Key::own).
	pub new_state: fn() -> Box<dyn Any>,
}

impl Kind {
	/// What an action does not have to set: one thing to do, nothing required, no state of its own.
	/// `Kind { uuid, name, defaults, behavior, ..Kind::BASE }`
	pub const BASE: Kind = Kind {
		uuid: "",
		name: "",
		defaults: Map::new,
		press_key: "",
		ops: &[],
		press_only: &[],
		can_repeat: false,
		required: &[],
		behavior: &NoHooks,
		new_state: || Box::new(()),
	};
}

struct NoHooks;

impl Behavior for NoHooks {}

/// The hooks of an action. All are optional.
#[async_trait(?Send)]
pub trait Behavior: Sync {
	/// The touch strip layout of a dial (layouts/*.json).
	fn layout(&self) -> Option<&'static str> {
		None
	}
	fn render(&self, _k: &KeyRc) {}
	fn render_offline(&self, _k: &KeyRc) {}
	fn on_disappear(&self, _k: &KeyRc) {}
	fn on_change(&self, k: &KeyRc, _change: Change) {
		k.draw();
	}
	async fn on_press(&self, _k: &KeyRc, _press: Press) -> Res<()> {
		Ok(())
	}
	async fn on_release(&self, _k: &KeyRc) -> Res<()> {
		Ok(())
	}
	async fn on_dial_rotate(&self, _k: &KeyRc, _ticks: f64) -> Res<()> {
		Ok(())
	}
	async fn on_dial_down(&self, _k: &KeyRc) -> Res<()> {
		Ok(())
	}
	/// A touch does what a press does, unless touches are off.
	async fn on_touch_tap(&self, k: &KeyRc) -> Res<()> {
		if !k.settings::<Touch>().disable_touch {
			return self.on_dial_down(k).await;
		}
		Ok(())
	}
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Touch {
	disable_touch: bool,
}

/// The defaults of an action from a JSON object.
pub fn defaults(value: Value) -> Map<String, Value> {
	match value {
		Value::Object(map) => map,
		_ => Map::new(),
	}
}

/// All actions.
pub fn all() -> Vec<&'static Kind> {
	vec![
		&voices::VOICE,
		&sounds::SOUND,
		&switches::VOICE_CHANGER,
		&switches::MUTE,
		&switches::HEAR_MYSELF,
		&switches::BACKGROUND,
		&voices::RANDOM_VOICE,
		&voices::NEXT_VOICE,
		&sounds::STOP_SOUNDS,
		&sounds::RANDOM_SOUND,
		&say::SAY_SOUND,
		&say::SAY_VOICE,
		&sounds::SOUND_VOLUME,
		&switches::HEAR_SOUNDS,
		&voices::VOICE_DIAL,
		&sounds::SOUND_DIAL,
		&sounds::SOUND_VOLUME_DIAL,
	]
}

#[cfg(test)]
mod tests {
	#[test]
	fn every_action_is_registered_with_openaction() {
		let main = include_str!("../main.rs");
		for kind in super::all() {
			assert!(main.contains(&format!("\"{}\"", kind.uuid)), "{} is missing in main.rs", kind.uuid);
		}
	}
}
