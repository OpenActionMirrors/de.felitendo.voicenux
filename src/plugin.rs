// Routes OpenDeck events to the keys and answers the settings pages.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use serde_json::{Value, json};
use tokio::task::spawn_local;

use crate::actions::{Appearance, Controller, Key, KeyRc, Kind};
use crate::desktop::{installed, open_voicenux};
use crate::js::{self, truthy};
use crate::sd::Sd;
use crate::voicenux::{Change, PORT, VoicenuxClient};

/// What happened to a key.
#[derive(Debug, Clone)]
pub enum EventKind {
	WillAppear,
	WillDisappear,
	KeyDown,
	KeyUp,
	DialRotate(f64),
	DialDown,
	TouchTap,
	DidReceiveSettings,
	InspectorDidAppear,
	InspectorDidDisappear,
	/// A message of the settings page.
	SendToPlugin(Value),
}

/// An event from OpenDeck for one key.
#[derive(Debug, Clone)]
pub struct Event {
	pub kind: EventKind,
	pub action: String,
	pub context: String,
	pub controller: Option<Controller>,
	pub is_in_multi_action: bool,
	pub state: Option<u16>,
	pub settings: Option<Value>,
}

pub struct Plugin {
	pub sd: Sd,
	/// One connection to Voicenux for all keys.
	pub client: Rc<VoicenuxClient>,
	kinds: HashMap<&'static str, &'static Kind>,
	instances: RefCell<HashMap<String, KeyRc>>,
	open_inspectors: RefCell<Vec<String>>,
	me: Weak<Plugin>,
}

impl Plugin {
	pub fn new(sd: Sd, kinds: Vec<&'static Kind>) -> Rc<Self> {
		let plugin = Self::with_client(sd, kinds, VoicenuxClient::new());
		plugin.client.start();
		plugin
	}

	pub fn with_client(sd: Sd, kinds: Vec<&'static Kind>, client: Rc<VoicenuxClient>) -> Rc<Self> {
		let plugin = Rc::new_cyclic(|me| Plugin {
			sd,
			client,
			kinds: kinds.into_iter().map(|k| (k.uuid, k)).collect(),
			instances: RefCell::new(HashMap::new()),
			open_inspectors: RefCell::new(Vec::new()),
			me: me.clone(),
		});
		let me = Rc::downgrade(&plugin);
		plugin.client.on_status.on({
			let me = me.clone();
			move |(status, error)| {
				let Some(plugin) = me.upgrade() else { return };
				let status = status.name();
				plugin.sd.log(&if error.is_empty() { format!("Voicenux {status}") } else { format!("Voicenux {status}: {error}") });
				plugin.broadcast_state();
			}
		});
		// Lists on the settings pages follow Voicenux.
		plugin.client.on_change.on(move |change| {
			if *change != Change::Playing
				&& let Some(plugin) = me.upgrade()
			{
				plugin.broadcast_state();
			}
		});
		plugin
	}

	pub fn key(&self, context: &str) -> Option<KeyRc> {
		self.instances.borrow().get(context).cloned()
	}

	fn instance_for(&self, e: &Event) -> Option<KeyRc> {
		if let Some(instance) = self.key(&e.context) {
			return Some(instance);
		}
		let kind = self.kinds.get(e.action.as_str())?;
		let plugin = self.me.upgrade()?;
		let appearance = if matches!(e.kind, EventKind::SendToPlugin(_)) {
			Appearance::default()
		} else {
			Appearance { controller: e.controller, is_in_multi_action: e.is_in_multi_action, state: e.state, settings: e.settings.clone() }
		};
		let instance = Key::new(&plugin, kind, &e.context, &appearance);
		self.instances.borrow_mut().insert(e.context.clone(), instance.clone());
		// Multi action children never get willAppear, they only press.
		if !matches!(e.kind, EventKind::WillAppear) {
			instance.attach();
		}
		Some(instance)
	}

	// Press events carry the settings, which matters for multi action children.
	fn with_settings(&self, e: &Event) -> Option<KeyRc> {
		let instance = self.instance_for(e)?;
		if let Some(settings) = e.settings.as_ref().filter(|s| truthy(Some(s))) {
			let merged = Value::Object(instance.with_defaults(Some(settings)));
			if !js::same(&merged, &Value::Object(instance.settings())) {
				instance.update_settings(Some(settings));
			}
		}
		Some(instance)
	}

	pub fn on_wake_up(&self) {
		self.client.reconnect();
	}

	pub async fn on_message(self: Rc<Self>, e: Event) {
		match &e.kind {
			EventKind::WillAppear => {
				if let Some(instance) = self.instance_for(&e) {
					instance.appear(e.controller, e.state);
				}
			}
			EventKind::WillDisappear => {
				let instance = self.instances.borrow_mut().remove(&e.context);
				if let Some(instance) = instance {
					instance.disappear();
				}
			}
			EventKind::KeyDown => {
				if let Some(instance) = self.with_settings(&e) {
					instance.key_down(e.is_in_multi_action).await;
				}
			}
			EventKind::KeyUp => {
				if let Some(instance) = self.with_settings(&e) {
					instance.key_up().await;
				}
			}
			EventKind::DialRotate(ticks) => {
				if let Some(instance) = self.with_settings(&e) {
					instance.dial_rotate(*ticks).await;
				}
			}
			EventKind::DialDown => {
				if let Some(instance) = self.with_settings(&e) {
					instance.dial_down().await;
				}
			}
			EventKind::TouchTap => {
				if let Some(instance) = self.with_settings(&e) {
					instance.touch_tap().await;
				}
			}
			EventKind::DidReceiveSettings => {
				if let Some(instance) = self.instance_for(&e) {
					instance.update_settings(e.settings.as_ref());
				}
			}
			EventKind::InspectorDidAppear => {
				self.add_inspector(&e.context);
				self.send_state(&e.context);
			}
			EventKind::InspectorDidDisappear => {
				self.open_inspectors.borrow_mut().retain(|c| *c != e.context);
			}
			EventKind::SendToPlugin(payload) => self.on_inspector_message(&e, payload),
		}
	}

	fn add_inspector(&self, context: &str) {
		let mut open = self.open_inspectors.borrow_mut();
		if !open.iter().any(|c| c == context) {
			open.push(context.to_string());
		}
	}

	/// State shown at the top of every settings page, with the lists for the drop downs.
	pub fn state_for(&self, context: &str) -> Value {
		let instance = self.key(context);
		let client = &self.client;
		let voicenux = client.state().map_or(Value::Null, |state| {
			json!({
				"voice": state.voice.map_or(Value::Null, |v| json!({ "id": v.id, "name": v.name, "icon": v.icon })),
				"sound_volume": state.sound_volume.map_or(Value::Null, js::num),
			})
		});
		let voices: Vec<Value> = client
			.voices()
			.into_iter()
			.map(|v| json!({ "value": v.id, "label": v.name, "icon": v.icon, "favorite": v.favorite }))
			.collect();
		let sounds: Vec<Value> = client.sounds().into_iter().map(|s| json!({ "value": s.id, "label": s.name, "icon": s.icon })).collect();
		json!({
			"type": "state",
			"connection": {
				"status": client.status().name(),
				"error": client.last_error(),
				"version": client.version(),
				"port": PORT,
				"installed": installed(),
			},
			// The voice in use and the soundboard volume, for the pictures of the key.
			"voicenux": voicenux,
			"lists": { "voices": voices, "sounds": sounds },
			"settings": instance.as_ref().map_or(Value::Null, |i| Value::Object(i.settings())),
			"keyError": instance.as_ref().and_then(|i| i.last_error()).map_or(Value::Null, Value::from),
		})
	}

	/// Something on the settings page of this key changed, like its last error.
	pub fn state_changed(&self, context: &str) {
		if self.open_inspectors.borrow().iter().any(|c| c == context) {
			self.send_state(context);
		}
	}

	fn send_state(&self, context: &str) {
		self.sd.send_to_property_inspector(context, self.state_for(context));
	}

	fn broadcast_state(&self) {
		let open = self.open_inspectors.borrow().clone();
		for context in open {
			self.send_state(&context);
		}
	}

	fn on_inspector_message(&self, e: &Event, payload: &Value) {
		let context = e.context.clone();
		let payload = if payload.is_object() { payload.clone() } else { json!({}) };
		self.instance_for(e);
		self.add_inspector(&context);
		let reply = |data: Value| {
			let mut data = js::object(Some(&data));
			if let Some(id) = payload.get("requestId") {
				data.insert("requestId".into(), id.clone());
			}
			self.sd.send_to_property_inspector(&context, Value::Object(data));
		};
		match payload.get("type").and_then(Value::as_str).unwrap_or("") {
			"hello" => self.send_state(&context),
			// Starts Voicenux on its settings, where "Let other apps control Voicenux" is.
			"openVoicenux" => {
				let page = match payload.get("page") {
					None | Some(Value::Null) => "settings".to_string(),
					page => js::to_string(page),
				};
				match open_voicenux(&page) {
					Ok(()) => reply(json!({ "type": "opened" })),
					Err(error) => reply(json!({ "type": "opened", "error": error.message() })),
				}
			}
			"retry" => self.client.reconnect(),
			"openUrl" => {
				let url = js::str_or_empty(payload.get("url"));
				if url.starts_with("http://") || url.starts_with("https://") {
					self.sd.open_url(&url);
				}
			}
			_ => {}
		}
	}
}

/// Starts the handling of an event as a task of its own, without waiting for it.
pub fn dispatch(plugin: &Rc<Plugin>, event: Event) {
	spawn_local(plugin.clone().on_message(event));
}
