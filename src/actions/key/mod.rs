// One key or dial of an action: its settings, the pictures of its voice or sound, and the hooks of
// its action. gestures.rs turns key presses into press, hold and double press, output.rs sends what
// the key shows to OpenDeck.
mod gestures;
mod output;

use std::any::Any;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::future::Future;
use std::rc::{Rc, Weak};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tokio::task::spawn_local;

pub use gestures::{DOUBLE_PRESS_MS, LONG_PRESS_MS};

use super::{Behavior, Controller, Kind};
use crate::js;
use crate::plugin::Plugin;
use crate::sd::Sd;
use crate::settings;
use crate::timers::{Timer, set_timeout};
use crate::voicenux::{Change, State, Status, VoicenuxClient};
use crate::{Error, Res};

pub type KeyRc = Rc<Key>;

/// One key or dial of an action (an instance).
pub struct Key {
	pub kind: &'static Kind,
	plugin: Weak<Plugin>,
	pub sd: Sd,
	pub client: Rc<VoicenuxClient>,
	pub context: String,
	b: RefCell<Base>,
	own: RefCell<Box<dyn Any>>,
	// Counts key ups, so a repeat that is still pressing knows the key was let go meanwhile.
	release_count: Cell<u64>,
	me: Weak<Key>,
}

#[derive(Default)]
struct Base {
	controller: Controller,
	is_in_multi_action: bool,
	state: u16,
	settings: Map<String, Value>,
	visible: bool,
	online: bool,
	attached: bool,
	unsubscribe: Vec<Box<dyn FnOnce()>>,
	timers: HashMap<u64, Timer>,
	next_timer: u64,
	last_error: Option<String>,
	screen: Screen,
	pending: Pending,
	// The picture of a voice or sound, by its name, once Voicenux sent it.
	picture_name: Option<String>,
	png: Option<String>,
}

/// What the key shows. Keeps what was sent to OpenDeck, so the same is not sent twice.
#[derive(Default)]
struct Screen {
	title: Option<String>,
	image: Option<String>,
	state: Option<u16>,
	/// The touch strip content as JSON text. None sends it again.
	feedback: Option<String>,
	/// The key shows an image of its own, not the one from the manifest. OpenDeck keeps such an
	/// image in the profile, also across restarts, so a new key starts out assuming one: like
	/// "Voicenux off" from before, which would stay on keys that show their manifest image.
	image_overridden: bool,
	/// The status image, while Voicenux cannot be reached.
	status_shown: Option<Status>,
	/// The setup image, while the key is not set up.
	setup_shown: bool,
}

/// A press that waits for key up, the long press or a second press, to know which gesture it is.
#[derive(Default)]
struct Pending {
	/// Waiting for key up.
	waiting: bool,
	/// What a double press does.
	double: String,
	/// This press is the second of a double press.
	second: bool,
	/// Runs the first press, unless a second one comes in time.
	single_timer: Option<Timer>,
	/// The long press already ran.
	long_fired: bool,
	long_timer: Option<Timer>,
	repeat_timer: Option<Timer>,
}

/// What OpenDeck says about a key when it first shows up.
#[derive(Debug, Clone, Default)]
pub struct Appearance {
	pub controller: Option<Controller>,
	pub is_in_multi_action: bool,
	pub state: Option<u16>,
	pub settings: Option<Value>,
}

fn with_defaults(kind: &Kind, settings: Option<&Value>) -> Map<String, Value> {
	let mut merged = (kind.defaults)();
	if let Some(Value::Object(settings)) = settings {
		js::assign(&mut merged, settings);
	}
	merged
}

impl Key {
	pub fn new(plugin: &Rc<Plugin>, kind: &'static Kind, context: &str, a: &Appearance) -> KeyRc {
		Rc::new_cyclic(|me| Key {
			kind,
			plugin: Rc::downgrade(plugin),
			sd: plugin.sd.clone(),
			client: plugin.client.clone(),
			context: context.to_string(),
			b: RefCell::new(Base {
				controller: a.controller.unwrap_or_default(),
				is_in_multi_action: a.is_in_multi_action,
				state: a.state.unwrap_or(0),
				settings: with_defaults(kind, a.settings.as_ref()),
				screen: Screen { image_overridden: true, ..Screen::default() },
				..Base::default()
			}),
			own: RefCell::new((kind.new_state)()),
			release_count: Cell::new(0),
			me: me.clone(),
		})
	}

	fn rc(&self) -> Option<KeyRc> {
		self.me.upgrade()
	}

	fn behavior(&self) -> &'static dyn Behavior {
		self.kind.behavior
	}

	/// The action's own state (Kind::new_state). Do not hold it across an await.
	pub fn own<T: 'static>(&self) -> RefMut<'_, T> {
		RefMut::map(self.own.borrow_mut(), |b| b.downcast_mut::<T>().expect("state of this action"))
	}

	// ----- settings -----

	pub fn with_defaults(&self, settings: Option<&Value>) -> Map<String, Value> {
		with_defaults(self.kind, settings)
	}

	/// The settings as the action keeps them (with its defaults), for OpenDeck and the settings page.
	pub fn settings_map(&self) -> Map<String, Value> {
		self.b.borrow().settings.clone()
	}

	/// The settings as the struct of the action (see settings.rs).
	pub fn settings<T: DeserializeOwned + Default>(&self) -> T {
		settings::read(&self.b.borrow().settings)
	}

	/// One setting as it is, for settings whose name is not fixed (None: not there).
	pub fn setting(&self, key: &str) -> Option<Value> {
		self.b.borrow().settings.get(key).cloned()
	}

	/// Changes settings from the key itself, like the sound it found. OpenDeck keeps them, and an
	/// open settings page shows them.
	pub fn save_settings(&self, changes: &[(&str, Value)]) {
		let settings = {
			let mut b = self.b.borrow_mut();
			for (key, value) in changes {
				b.settings.insert(key.to_string(), value.clone());
			}
			b.settings.clone()
		};
		self.sd.set_settings(&self.context, &settings);
		if let Some(plugin) = self.plugin.upgrade() {
			plugin.state_changed(&self.context);
		}
	}

	/// A key is set up when its required settings are filled in.
	pub fn is_set_up(&self) -> bool {
		let b = self.b.borrow();
		self.kind.required.iter().all(|key| !js::trim(&js::str_or_empty(b.settings.get(*key))).is_empty())
	}

	pub fn controller(&self) -> Controller {
		self.b.borrow().controller
	}

	pub fn is_in_multi_action(&self) -> bool {
		self.b.borrow().is_in_multi_action
	}

	pub fn visible(&self) -> bool {
		self.b.borrow().visible
	}

	pub fn online(&self) -> bool {
		self.b.borrow().online
	}

	pub fn state(&self) -> u16 {
		self.b.borrow().state
	}

	pub fn last_error(&self) -> Option<String> {
		self.b.borrow().last_error.clone()
	}

	// ----- Voicenux -----

	/// What Voicenux says right now: voice, switches, soundboard volume.
	pub fn vn(&self) -> State {
		self.client.state().unwrap_or_default()
	}

	pub fn log(&self, message: &str) {
		self.sd.log(&format!("{}: {message}", self.kind.name));
	}

	/// An action from the Voicenux API. Errors bubble up to safe(), which shows the alert.
	pub async fn call(&self, action: &str, arg: Option<&str>) -> Res<Value> {
		self.client.call(action, arg).await
	}

	/// The picture of a voice or sound. Loads it and draws again when it is there.
	pub fn picture(&self, icon: &str) -> Option<String> {
		if icon.is_empty() {
			return None;
		}
		{
			let mut b = self.b.borrow_mut();
			if b.picture_name.as_deref() == Some(icon) {
				return b.png.clone();
			}
			b.picture_name = Some(icon.to_string());
			// Pictures Voicenux sent before are there right away, without a second draw.
			b.png = self.client.cached_icon(icon);
			if b.png.is_some() {
				return b.png.clone();
			}
		}
		let load = self.client.icon(icon);
		let me = self.me.clone();
		let icon = icon.to_string();
		spawn_local(async move {
			let png = load.await;
			let Some(me) = me.upgrade() else { return };
			let Some(png) = png else { return };
			{
				let mut b = me.b.borrow_mut();
				if b.picture_name.as_deref() != Some(icon.as_str()) {
					return;
				}
				b.png = Some(png);
			}
			if me.online() {
				me.draw();
			}
		});
		None
	}

	pub fn attach(&self) {
		if self.b.borrow().attached {
			return;
		}
		self.b.borrow_mut().attached = true;
		let me = self.me.clone();
		let on_status = self.client.on_status.on(move |_| {
			if let Some(me) = me.upgrade() {
				me.refresh_online();
			}
		});
		let me = self.me.clone();
		let on_change = self.client.on_change.on(move |change| {
			let Some(me) = me.upgrade() else { return };
			me.refresh_online();
			if me.online() && *change != Change::All {
				me.behavior().on_change(&me, *change);
			}
		});
		let client = self.client.clone();
		self.b.borrow_mut().unsubscribe.push(Box::new(move || {
			client.on_status.off(on_status);
			client.on_change.off(on_change);
		}));
		self.b.borrow_mut().online = false;
		self.refresh_online();
	}

	fn detach(&self) {
		let unsubscribe = std::mem::take(&mut self.b.borrow_mut().unsubscribe);
		for f in unsubscribe {
			f();
		}
		let mut b = self.b.borrow_mut();
		b.attached = false;
		b.online = false;
	}

	fn refresh_online(&self) {
		let online = self.client.ready();
		let was = self.online();
		if online && !was {
			self.b.borrow_mut().online = true;
			self.clear_status_image();
			self.draw();
		} else if !online {
			if was {
				let mut b = self.b.borrow_mut();
				b.online = false;
				// Pictures come again from the next Voicenux.
				b.picture_name = None;
				b.png = None;
			}
			self.show_status_image();
		}
	}

	/// A fresh instance (for example in a multi action) may still be connecting.
	pub async fn ensure_online(&self, timeout: f64) -> bool {
		if self.online() {
			return true;
		}
		if !self.b.borrow().attached {
			self.attach();
		}
		let until = js::now() as f64 + timeout;
		while !self.online() && (js::now() as f64) < until {
			crate::timers::sleep(50.0).await;
		}
		if !self.online() {
			self.show_alert();
		}
		self.online()
	}

	// ----- lifecycle, from the plugin -----

	pub fn appear(&self, controller: Option<Controller>, state: Option<u16>) {
		{
			let mut b = self.b.borrow_mut();
			b.visible = !b.is_in_multi_action;
			if let Some(state) = state {
				b.state = state;
			}
			if let Some(controller) = controller {
				b.controller = controller;
			}
		}
		// The touch strip layout first, before any feedback is sent.
		self.set_layout();
		self.attach();
		if !self.online() {
			self.draw_offline();
		}
	}

	pub fn disappear(&self) {
		self.b.borrow_mut().visible = false;
		self.clear_timers();
		if let Some(me) = self.rc() {
			self.behavior().on_disappear(&me);
		}
		self.detach();
	}

	pub fn update_settings(&self, settings: Option<&Value>) {
		self.b.borrow_mut().settings = self.with_defaults(settings);
		if self.online() {
			self.draw();
		} else {
			self.draw_offline();
		}
	}

	// ----- errors -----

	pub async fn safe(&self, f: impl Future<Output = Res<()>>) {
		if let Err(error) = f.await {
			self.report_error(&error);
		}
	}

	/// Something the user did (press, turn, touch). Success clears the last error.
	pub async fn act(&self, f: impl Future<Output = Res<()>>) -> bool {
		match f.await {
			Ok(()) => {
				self.clear_error();
				true
			}
			Err(error) => {
				self.report_error(&error);
				false
			}
		}
	}

	/// The settings page shows the last error, so the user sees why the key showed an alert.
	pub fn report_error(&self, error: &Error) {
		let message = error.message().to_string();
		self.log(&format!("error: {message}"));
		self.show_alert();
		if self.b.borrow().last_error.as_deref() == Some(message.as_str()) {
			return;
		}
		self.b.borrow_mut().last_error = Some(message);
		if let Some(plugin) = self.plugin.upgrade() {
			plugin.state_changed(&self.context);
		}
	}

	pub fn clear_error(&self) {
		if self.b.borrow_mut().last_error.take().is_none() {
			return;
		}
		if let Some(plugin) = self.plugin.upgrade() {
			plugin.state_changed(&self.context);
		}
	}

	// ----- timers -----

	/// A timer that runs only while the key is still there.
	pub fn after(&self, ms: f64, f: impl FnOnce(KeyRc) + 'static) -> Timer {
		let me = self.me.clone();
		set_timeout(ms, move || {
			if let Some(me) = me.upgrade() {
				f(me);
			}
		})
	}

	/// A new id for `timers`, which stop when the key goes away.
	fn timer_id(&self) -> u64 {
		let mut b = self.b.borrow_mut();
		b.next_timer += 1;
		b.next_timer
	}

	fn clear_timers(&self) {
		let mut b = self.b.borrow_mut();
		for (_, timer) in b.timers.drain() {
			timer.clear();
		}
		let p = &mut b.pending;
		for timer in [p.long_timer.take(), p.repeat_timer.take(), p.single_timer.take()].into_iter().flatten() {
			timer.clear();
		}
	}

	// ----- dials, for the plugin -----

	pub async fn dial_rotate(self: &Rc<Self>, ticks: f64) {
		if self.ensure_online(3000.0).await {
			let behavior = self.behavior();
			self.act(behavior.on_dial_rotate(self, ticks)).await;
		}
	}

	pub async fn dial_down(self: &Rc<Self>) {
		if self.ensure_online(3000.0).await {
			let behavior = self.behavior();
			self.act(behavior.on_dial_down(self)).await;
		}
	}

	pub async fn touch_tap(self: &Rc<Self>) {
		if self.ensure_online(3000.0).await {
			let behavior = self.behavior();
			self.act(behavior.on_touch_tap(self)).await;
		}
	}

	/// Render now, for actions that draw from their own state.
	pub fn render(self: &Rc<Self>) {
		self.behavior().render(self);
	}

	pub fn render_offline(self: &Rc<Self>) {
		self.behavior().render_offline(self);
	}
}

#[cfg(test)]
pub(crate) mod testing {
	use super::*;
	use crate::voicenux::{Sound, Voice};

	pub fn voices() -> Vec<Voice> {
		let v = |id: &str, name: &str, icon: &str, favorite: bool| Voice { id: id.into(), name: name.into(), icon: icon.into(), favorite };
		vec![v("robot", "Robot", "robot", false), v("deep", "Deep", "bear", true), v("chipmunk", "Chipmunk", "chipmunk", false)]
	}

	pub fn sound(id: &str, name: &str, icon: &str) -> Sound {
		Sound { id: id.into(), name: name.into(), icon: icon.into() }
	}

	/// A plugin with one key of this action, visible and online, and a Voicenux that answers like
	/// the real API and writes down every call.
	pub fn key(kind: &'static Kind, settings: Value) -> (Rc<Plugin>, KeyRc) {
		let client = VoicenuxClient::with("127.0.0.1", 1, Box::new(|| false));
		let robot = voices().into_iter().next();
		let state =
			State { voice: robot, voice_changer: true, background: true, sound_volume: Some(80.0), hear_sounds: true, ..State::default() };
		client.fake_connected(state, voices(), vec![sound("airhorn", "Air Horn", "postal-horn")]);
		let plugin = Plugin::with_client(Sd::recording(), crate::actions::all(), client);
		let key = Key::new(&plugin, kind, "c", &Appearance { settings: Some(settings), ..Default::default() });
		{
			let mut b = key.b.borrow_mut();
			b.visible = true;
			b.online = true;
		}
		(plugin, key)
	}

	pub fn calls(key: &Key) -> Vec<(String, Option<String>)> {
		key.client.fake.borrow().as_ref().map(|f| f.calls.clone()).unwrap_or_default()
	}

	/// The last call, like ["voice", "deep"].
	pub fn last(key: &Key) -> Vec<String> {
		calls(key).last().map(|(a, arg)| std::iter::once(a.clone()).chain(arg.clone()).collect()).unwrap_or_default()
	}

	pub fn old_voicenux(key: &Key) {
		if let Some(fake) = key.client.fake.borrow_mut().as_mut() {
			fake.old = true;
		}
	}

	pub async fn local<F: Future>(f: F) -> F::Output {
		tokio::task::LocalSet::new().run_until(f).await
	}
}
