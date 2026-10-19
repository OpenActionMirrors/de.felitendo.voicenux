// The base of every action: connection, press, hold and double press, the setup and status images,
// and the hooks the actions fill in (see Behavior).
pub mod sounds;
pub mod switches;
pub mod voices;

use std::any::Any;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use async_trait::async_trait;
use serde_json::{Map, Value};
use tokio::task::spawn_local;

use crate::icons::colors;
use crate::js::{self, truthy};
use crate::key_images::key_image;
use crate::plugin::Plugin;
use crate::render::{GlyphKey, glyph_key, status_key, status_label};
use crate::sd::Sd;
use crate::timers::{Timer, set_timeout};
use crate::voicenux::{State, VoicenuxClient};
use crate::{Error, Res};

pub const LONG_PRESS_MS: f64 = 700.0;
pub const DOUBLE_PRESS_MS: f64 = 300.0;
const REPEAT_DELAY_MS: f64 = 450.0;
const REPEAT_EVERY_MS: f64 = 140.0;

/// An action, like a class of the JavaScript version.
pub struct Kind {
	pub uuid: &'static str,
	/// For the log, like the class name: "Voice: error: no voice selected".
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
	/// The action's own state, kept per key.
	pub new_state: fn() -> Box<dyn Any>,
}

/// What a press does.
#[derive(Debug, Clone, PartialEq)]
pub struct PressArgs {
	pub op: String,
	pub gesture: String,
	pub repeat: bool,
}

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
	/// what: "state", "voices", "sounds" or "playing".
	fn on_change(&self, k: &KeyRc, _what: &str) {
		k.draw();
	}
	async fn on_press(&self, _k: &KeyRc, _args: Option<PressArgs>) -> Res<()> {
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
		if !k.s_truthy("disableTouch") {
			return self.on_dial_down(k).await;
		}
		Ok(())
	}
}

pub type KeyRc = Rc<Key>;

#[derive(Default)]
struct Base {
	controller: String,
	is_in_multi_action: bool,
	state: u16,
	settings: Map<String, Value>,
	visible: bool,
	online: bool,
	attached: bool,
	unsubscribe: Vec<Box<dyn FnOnce()>>,
	timers: HashMap<u64, Timer>,
	next_timer: u64,
	sent: HashMap<String, Value>,
	image_overridden: bool,
	status_shown: Option<String>,
	setup_shown: bool,
	last_error: Option<String>,
	gesturing: bool,
	double_gesture: String,
	second_press: bool,
	single_timer: Option<Timer>,
	long_fired: bool,
	long_timer: Option<Timer>,
	repeat_timer: Option<Timer>,
	// The picture of a voice or sound, by its name, once Voicenux sent it.
	picture_name: Option<String>,
	png: Option<String>,
}

/// One key or dial of an action (an instance).
pub struct Key {
	pub kind: &'static Kind,
	plugin: Weak<Plugin>,
	pub sd: Sd,
	pub client: Rc<VoicenuxClient>,
	pub context: String,
	pub action: String,
	b: RefCell<Base>,
	x: RefCell<Box<dyn Any>>,
	// Counts key ups, so a repeat that is still pressing knows the key was let go meanwhile.
	release_count: Cell<u64>,
	me: Weak<Key>,
}

/// What OpenDeck says about a key when it first shows up.
#[derive(Debug, Clone, Default)]
pub struct Appearance {
	pub controller: Option<String>,
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
	pub fn new(plugin: &Rc<Plugin>, kind: &'static Kind, context: &str, action: &str, a: &Appearance) -> KeyRc {
		Rc::new_cyclic(|me| Key {
			kind,
			plugin: Rc::downgrade(plugin),
			sd: plugin.sd.clone(),
			client: plugin.client.clone(),
			context: context.to_string(),
			action: action.to_string(),
			b: RefCell::new(Base {
				controller: a.controller.clone().unwrap_or_else(|| "Keypad".into()),
				is_in_multi_action: a.is_in_multi_action,
				state: a.state.unwrap_or(0),
				settings: with_defaults(kind, a.settings.as_ref()),
				..Base::default()
			}),
			x: RefCell::new((kind.new_state)()),
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

	/// The action's own state.
	pub fn x<T: 'static>(&self) -> RefMut<'_, T> {
		RefMut::map(self.x.borrow_mut(), |b| b.downcast_mut::<T>().expect("state of this action"))
	}

	// ----- settings -----

	pub fn with_defaults(&self, settings: Option<&Value>) -> Map<String, Value> {
		with_defaults(self.kind, settings)
	}

	pub fn settings(&self) -> Map<String, Value> {
		self.b.borrow().settings.clone()
	}

	/// A setting as it is (None: not there).
	pub fn s(&self, key: &str) -> Option<Value> {
		self.b.borrow().settings.get(key).cloned()
	}

	/// A setting as text, like `String(value ?? "")`.
	pub fn s_str(&self, key: &str) -> String {
		js::str_or_empty(self.b.borrow().settings.get(key))
	}

	pub fn s_truthy(&self, key: &str) -> bool {
		truthy(self.b.borrow().settings.get(key))
	}

	/// Number(value)
	pub fn s_num(&self, key: &str) -> f64 {
		js::number(self.b.borrow().settings.get(key))
	}

	pub fn controller(&self) -> String {
		self.b.borrow().controller.clone()
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
		let on_change = self.client.on_change.on(move |what| {
			let Some(me) = me.upgrade() else { return };
			me.refresh_online();
			if me.online() && what != "all" {
				me.behavior().on_change(&me, what);
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

	fn show_status_image(&self) {
		let status = self.client.status();
		if !self.visible() || status == "connecting" || status == "idle" || status.is_empty() {
			return;
		}
		self.b.borrow_mut().status_shown = Some(status.clone());
		// Dials show it as text in their offline look (see dial_status).
		if self.controller() == "Encoder" {
			if let Some(me) = self.rc() {
				self.behavior().render_offline(&me);
			}
		} else {
			self.set_image_with(&status_key(&status), None, true);
		}
	}

	fn clear_status_image(&self) {
		if self.b.borrow_mut().status_shown.take().is_none() {
			return;
		}
		// Dials draw again once they are online.
		if self.controller() == "Encoder" {
			return;
		}
		if self.b.borrow().setup_shown {
			self.set_image_with(&self.setup_image(), None, true);
		} else {
			self.reset_image(false);
		}
	}

	/// What a dial shows instead of its value while Voicenux cannot be reached: the words and colors
	/// of the status image of a key. Empty while connecting.
	pub fn dial_status(&self) -> Value {
		let Some(status) = self.b.borrow().status_shown.clone() else { return Value::from("") };
		crate::obj! { "value" => status_label(&status), "color" => if status == "offline" { colors::MUTED } else { colors::AMBER } }
	}

	fn set_layout(&self) {
		let layout = if self.controller() == "Encoder" { self.behavior().layout() } else { None };
		let Some(layout) = layout else { return };
		self.b.borrow_mut().sent.insert("feedback".into(), Value::Null);
		self.sd.set_feedback_layout(&self.context, layout);
		self.resend_feedback_soon();
	}

	/// A key is set up when its required settings are filled in.
	pub fn is_set_up(&self) -> bool {
		self.kind.required.iter().all(|key| !js::trim(&self.s_str(key)).is_empty())
	}

	fn icon(&self) -> String {
		crate::manifest::action_def(&self.action).map_or_else(|| "plugin".to_string(), |a| a.icon.to_string())
	}

	/// The glyph of the action in gray with a yellow dot: "pick something first".
	pub fn setup_image(&self) -> String {
		glyph_key(&self.icon(), &GlyphKey { color: colors::GRAY, setup: true, ..Default::default() })
	}

	/// Draw the key, or the setup image while it is not set up. Dials show this in their feedback.
	pub fn draw(&self) {
		if self.show_setup() {
			return;
		}
		if let Some(me) = self.rc() {
			self.behavior().render(&me);
		}
	}

	pub fn draw_offline(&self) {
		if self.show_setup() {
			return;
		}
		if let Some(me) = self.rc() {
			self.behavior().render_offline(&me);
		}
	}

	fn show_setup(&self) -> bool {
		if self.controller() != "Keypad" {
			return false;
		}
		if !self.is_set_up() {
			self.b.borrow_mut().setup_shown = true;
			if self.b.borrow().status_shown.is_none() {
				self.set_image_with(&self.setup_image(), None, true);
			}
			return true;
		}
		if self.b.borrow().setup_shown {
			self.b.borrow_mut().setup_shown = false;
			self.reset_image(true);
		}
		false
	}

	/// A key image from the manifest, drawn again with a title by the glyph.
	pub fn state_image(&self, state: u16, title: &str, title_at: &str) -> String {
		let def = crate::manifest::action_def(&self.action);
		let name = def.and_then(|a| a.state_images().get(state as usize).cloned());
		let (glyph, color) = match name.as_deref().and_then(key_image) {
			Some(image) => (image.glyph.to_string(), image.color.unwrap_or(colors::WHITE)),
			None => (self.icon(), colors::WHITE),
		};
		glyph_key(&glyph, &GlyphKey { color, title, title_at, ..Default::default() })
	}

	// ----- lifecycle from the plugin -----

	pub fn appear(&self, controller: Option<&str>, state: Option<u16>) {
		{
			let mut b = self.b.borrow_mut();
			b.visible = !b.is_in_multi_action;
			if let Some(state) = state {
				b.state = state;
			}
			if let Some(controller) = controller {
				b.controller = controller.to_string();
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

	/// What press, hold or double press does: one of `ops`, "none", or "repeat" for hold.
	pub fn gesture(&self, name: &str) -> String {
		let k = self.kind;
		if k.ops.is_empty() {
			return if name == "press" { "press".into() } else { "none".into() };
		}
		let pick = |v: Option<Value>| match v {
			Some(Value::String(s)) if k.ops.contains(&s.as_str()) => Some(s),
			_ => None,
		};
		let press = pick(self.s(k.press_key)).unwrap_or_else(|| k.ops[0].to_string());
		if name == "press" {
			return press;
		}
		if k.press_only.contains(&press.as_str()) {
			return "none".into();
		}
		let value = self.s(name);
		if name == "hold" && value == Some(Value::from("repeat")) && k.can_repeat {
			return "repeat".into();
		}
		match pick(value) {
			Some(v) if !k.press_only.contains(&v.as_str()) => v,
			_ => "none".into(),
		}
	}

	/// The choice for this call of on_press.
	pub fn op(&self, args: &Option<PressArgs>) -> String {
		match args {
			Some(a) => a.op.clone(),
			None => self.gesture("press"),
		}
	}

	pub async fn press(self: &Rc<Self>, name: &str, repeat: bool) -> Res<()> {
		let args = PressArgs { op: self.gesture(name), gesture: name.to_string(), repeat };
		self.behavior().on_press(self, Some(args)).await
	}

	// ----- key handling. In a multi action there are no gestures, every press is a short press. -----

	pub async fn key_down(self: &Rc<Self>, is_in_multi_action: bool) {
		self.b.borrow_mut().is_in_multi_action = is_in_multi_action;
		if !self.ensure_online(3000.0).await {
			return;
		}
		let multi = self.is_in_multi_action();
		let hold = if multi { "none".to_string() } else { self.gesture("hold") };
		let double = if multi { "none".to_string() } else { self.gesture("double") };
		if let Some(timer) = self.b.borrow_mut().repeat_timer.take() {
			timer.clear();
		}
		if double == "none" && (hold == "none" || hold == "repeat") {
			// Right away on key down. "repeat" goes on while the key is held.
			self.b.borrow_mut().gesturing = false;
			if hold == "repeat" {
				self.start_repeat(REPEAT_DELAY_MS);
			}
			self.act(self.press("press", false)).await;
			return;
		}
		// Wait for key up (or the long press) to know which gesture it is.
		{
			let mut b = self.b.borrow_mut();
			b.gesturing = true;
			b.double_gesture = double;
			// A second press while the first one waits for its double press.
			let single = b.single_timer.take();
			b.second_press = single.is_some();
			if let Some(timer) = single {
				timer.clear();
			}
			b.long_fired = false;
			if let Some(timer) = b.long_timer.take() {
				timer.clear();
			}
		}
		if hold == "none" {
			return;
		}
		let me = self.me.clone();
		let releases = self.release_count.get();
		let delay = if hold == "repeat" { REPEAT_DELAY_MS } else { LONG_PRESS_MS };
		let timer = set_timeout(delay, move || {
			let Some(me) = me.upgrade() else { return };
			spawn_local(async move {
				me.b.borrow_mut().long_fired = true;
				// The first press of the pair still counts.
				if me.b.borrow().second_press {
					me.act(me.press("press", false)).await;
				}
				if hold == "repeat" {
					me.act(me.press("press", false)).await;
					// Let go meanwhile: no repeat. (The JavaScript version kept repeating then.)
					if me.release_count.get() == releases {
						me.start_repeat(REPEAT_EVERY_MS);
					}
				} else if me.act(me.press("hold", false)).await {
					me.show_ok();
				}
			});
		});
		self.b.borrow_mut().long_timer = Some(timer);
	}

	fn start_repeat(&self, delay: f64) {
		let me = self.me.clone();
		let releases = self.release_count.get();
		let timer = set_timeout(delay, move || {
			let Some(me) = me.upgrade() else { return };
			spawn_local(async move {
				me.act(me.press("press", true)).await;
				if me.release_count.get() == releases {
					me.start_repeat(REPEAT_EVERY_MS);
				}
			});
		});
		self.b.borrow_mut().repeat_timer = Some(timer);
	}

	pub async fn key_up(self: &Rc<Self>) {
		self.release_count.set(self.release_count.get() + 1);
		if let Some(timer) = self.b.borrow_mut().repeat_timer.take() {
			timer.clear();
		}
		if self.b.borrow().gesturing {
			let (long_fired, second_press, double) = {
				let mut b = self.b.borrow_mut();
				b.gesturing = false;
				if let Some(timer) = b.long_timer.take() {
					timer.clear();
				}
				(b.long_fired, b.second_press, b.double_gesture.clone())
			};
			if long_fired || !self.online() {
				return;
			}
			if second_press {
				self.b.borrow_mut().second_press = false;
				self.act(self.press("double", false)).await;
			} else if double != "none" {
				// Wait a moment: a second press makes it a double press.
				let me = self.me.clone();
				let timer = set_timeout(DOUBLE_PRESS_MS, move || {
					let Some(me) = me.upgrade() else { return };
					me.b.borrow_mut().single_timer = None;
					spawn_local(async move {
						me.act(me.press("press", false)).await;
					});
				});
				self.b.borrow_mut().single_timer = Some(timer);
			} else {
				self.act(self.press("press", false)).await;
			}
			return;
		}
		// Not act(): letting go of the key must not clear the error of the press.
		let behavior = self.behavior();
		self.safe(behavior.on_release(self)).await;
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

	pub async fn safe(&self, f: impl std::future::Future<Output = Res<()>>) {
		if let Err(error) = f.await {
			self.report_error(&error);
		}
	}

	/// Something the user did (press, turn, touch). Success clears the last error.
	pub async fn act(&self, f: impl std::future::Future<Output = Res<()>>) -> bool {
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

	/// A timer that stops with the key.
	pub fn after(&self, ms: f64, f: impl FnOnce(KeyRc) + 'static) -> Timer {
		let me = self.me.clone();
		set_timeout(ms, move || {
			if let Some(me) = me.upgrade() {
				f(me);
			}
		})
	}

	fn clear_timers(&self) {
		let mut b = self.b.borrow_mut();
		for (_, timer) in b.timers.drain() {
			timer.clear();
		}
		for timer in [b.long_timer.take(), b.repeat_timer.take(), b.single_timer.take()].into_iter().flatten() {
			timer.clear();
		}
	}

	// ----- output helpers. They skip sends that would not change anything. -----

	fn set_title(&self, title: &str) {
		if !self.visible() {
			return;
		}
		{
			let mut b = self.b.borrow_mut();
			let key = "title:undefined";
			if b.sent.get(key) == Some(&Value::from(title)) {
				return;
			}
			b.sent.insert(key.into(), title.into());
			for (k, v) in b.sent.iter_mut() {
				if k.starts_with("title:") {
					*v = title.into();
				}
			}
		}
		self.sd.set_title(&self.context, title, None);
	}

	/// Keys draw their titles themselves, close to the glyph. A title in OpenDeck would be a second
	/// one, so it is cleared while the key shows its own. Without one, a title the user typed stays.
	pub fn own_title(&self, title: &str) {
		if !title.is_empty() {
			self.set_title("");
		}
	}

	pub fn set_image(&self, image: &str) {
		self.set_image_with(image, None, false);
	}

	/// force: also while the setup or status image is shown.
	pub fn set_image_with(&self, image: &str, state: Option<u16>, force: bool) {
		if !self.visible() {
			return;
		}
		{
			let mut b = self.b.borrow_mut();
			if !force && (b.setup_shown || b.status_shown.is_some()) {
				return;
			}
			let key = match state {
				Some(s) => format!("image:{s}"),
				None => "image:undefined".into(),
			};
			if b.sent.get(&key) == Some(&Value::from(image)) {
				return;
			}
			b.sent.insert(key, image.into());
			if state.is_none() {
				for (k, v) in b.sent.iter_mut() {
					if k.starts_with("image:") {
						*v = image.into();
					}
				}
			}
			b.image_overridden = true;
		}
		self.sd.set_image(&self.context, image, state);
	}

	/// The manifest pictures have the glyph in the middle. With a title, draw it with the title
	/// below or above (title_at).
	pub fn glyph_for_title(&self, title: &str, title_at: &str) {
		self.own_title(title);
		if !title.is_empty() {
			self.set_image(&self.state_image(self.state(), title, title_at));
		} else {
			self.reset_image(false);
		}
	}

	/// Back to the image from the manifest (or the one the user picked).
	pub fn reset_image(&self, force: bool) {
		{
			let mut b = self.b.borrow_mut();
			if !force && (b.setup_shown || b.status_shown.is_some()) {
				return;
			}
			if !b.image_overridden {
				return;
			}
			b.image_overridden = false;
			b.sent.retain(|k, _| !k.starts_with("image:"));
		}
		if self.visible() {
			self.sd.set_image(&self.context, "", None);
		}
	}

	pub fn set_state(&self, state: u16) {
		{
			let mut b = self.b.borrow_mut();
			if !b.visible && !b.is_in_multi_action {
				return;
			}
			if b.state == state && b.sent.get("state") == Some(&Value::from(state)) {
				return;
			}
			b.state = state;
			b.sent.insert("state".into(), state.into());
		}
		self.sd.set_state(&self.context, state);
	}

	/// OpenDeck loads a custom touch strip layout again after the key appeared, which drops what
	/// was sent before. So send it again a little later.
	fn resend_feedback_soon(&self) {
		for ms in [700.0, 2500.0] {
			let me = self.me.clone();
			let id = {
				let mut b = self.b.borrow_mut();
				b.next_timer += 1;
				b.next_timer
			};
			let timer = set_timeout(ms, move || {
				let Some(me) = me.upgrade() else { return };
				me.b.borrow_mut().timers.remove(&id);
				if !me.visible() {
					return;
				}
				me.b.borrow_mut().sent.insert("feedback".into(), Value::Null);
				let behavior = me.behavior();
				if me.online() {
					behavior.render(&me);
				} else {
					behavior.render_offline(&me);
				}
			});
			self.b.borrow_mut().timers.insert(id, timer);
		}
	}

	pub fn set_feedback(&self, payload: Value) {
		if !self.visible() {
			return;
		}
		let text = js::stringify(&payload);
		{
			let mut b = self.b.borrow_mut();
			if b.sent.get("feedback") == Some(&Value::from(text.as_str())) {
				return;
			}
			b.sent.insert("feedback".into(), text.into());
		}
		self.sd.set_feedback(&self.context, payload);
	}

	pub fn show_ok(&self) {
		if self.visible() {
			self.sd.show_ok(&self.context);
		}
	}

	pub fn show_alert(&self) {
		if self.visible() {
			self.sd.show_alert(&self.context);
		}
	}

	// ----- dials, for the plugin -----

	pub async fn dial_rotate(self: &Rc<Self>, ticks: f64) {
		if self.ensure_online(3000.0).await {
			let behavior = self.behavior();
			self.act(behavior.on_dial_rotate(self, ticks)).await;
		}
	}

	pub async fn dial_event(self: &Rc<Self>, event: &str) {
		if !self.ensure_online(3000.0).await {
			return;
		}
		let behavior = self.behavior();
		if event == "touchTap" {
			self.act(behavior.on_touch_tap(self)).await;
		} else {
			self.act(behavior.on_dial_down(self)).await;
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

/// The defaults of an action from a JSON object.
pub fn defaults(value: Value) -> Map<String, Value> {
	match value {
		Value::Object(map) => map,
		_ => Map::new(),
	}
}

/// All actions, in the order of the JavaScript version.
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
		&sounds::SOUND_VOLUME,
		&switches::HEAR_SOUNDS,
		&voices::VOICE_DIAL,
		&sounds::SOUND_VOLUME_DIAL,
	]
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
		let plugin = Plugin::with_client(Sd::recording(), all(), client);
		let key = Key::new(&plugin, kind, "c", kind.uuid, &Appearance { settings: Some(settings), ..Default::default() });
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

	pub async fn local<F: std::future::Future>(f: F) -> F::Output {
		tokio::task::LocalSet::new().run_until(f).await
	}
}

#[cfg(test)]
mod tests {
	use super::testing::*;
	use super::*;
	use serde_json::json;

	struct Probe;

	#[derive(Default)]
	struct ProbeState {
		seen: Vec<String>,
	}

	#[async_trait(?Send)]
	impl Behavior for Probe {
		async fn on_press(&self, k: &KeyRc, args: Option<PressArgs>) -> Res<()> {
			let op = k.op(&args);
			k.x::<ProbeState>().seen.push(op);
			Ok(())
		}
	}

	static PROBE: Kind = Kind {
		uuid: "de.felitendo.voicenux.probe",
		name: "Probe",
		defaults: Map::new,
		press_key: "mode",
		ops: &["a", "b"],
		press_only: &[],
		can_repeat: false,
		required: &[],
		behavior: &Probe,
		new_state: || Box::new(ProbeState::default()),
	};

	fn seen(key: &Key) -> Vec<String> {
		key.x::<ProbeState>().seen.clone()
	}

	#[tokio::test(start_paused = true)]
	async fn a_second_press_within_a_moment_is_a_double_press() {
		local(async {
			let (_plugin, key) = key(&PROBE, json!({ "mode": "a", "double": "b" }));
			let tap = || async {
				key.key_down(false).await;
				key.key_up().await;
			};
			tap().await;
			crate::timers::sleep(DOUBLE_PRESS_MS + 100.0).await;
			assert_eq!(seen(&key), ["a"]);
			tap().await;
			tap().await;
			crate::timers::sleep(DOUBLE_PRESS_MS + 100.0).await;
			assert_eq!(seen(&key), ["a", "b"]);
		})
		.await;
	}

	#[tokio::test(start_paused = true)]
	async fn holding_runs_the_hold_choice_a_short_press_the_press_choice() {
		local(async {
			let (_plugin, k) = key(&PROBE, json!({ "mode": "a", "hold": "b" }));
			k.key_down(false).await;
			crate::timers::sleep(LONG_PRESS_MS + 100.0).await;
			k.key_up().await;
			k.key_down(false).await;
			k.key_up().await;
			assert_eq!(seen(&k), ["b", "a"]);
			// Without hold and double press the press runs right on key down.
			let (_plugin, plain) = key(&PROBE, json!({ "mode": "b" }));
			plain.key_down(false).await;
			assert_eq!(seen(&plain), ["b"]);
			// In a multi action there are no gestures.
			let (_plugin, multi) = key(&PROBE, json!({ "mode": "a", "hold": "b" }));
			multi.key_down(true).await;
			assert_eq!(seen(&multi), ["a"]);
		})
		.await;
	}
}
