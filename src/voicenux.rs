// Client for the Voicenux API (see API.md in the Voicenux repo). One connection for all keys. It
// keeps the state, the voices, the sounds and what plays, and tells the keys about every change.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{AbortHandle, spawn_local};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{self, Message};

use crate::emitter::Emitter;
use crate::js;
use crate::timers::{Timer, set_timeout};
use crate::{Error, Res};

pub const PORT: u16 = 41967;
/// The API version this plugin speaks. Voicenux raises it when a change breaks clients.
pub const API: f64 = 1.0;

const RETRY_DELAYS: [f64; 4] = [1000.0, 2000.0, 3000.0, 5000.0];
const CALL_TIMEOUT: f64 = 5000.0;
// The hello comes right away. Without it, something else listens on the port.
const HELLO_TIMEOUT: f64 = 3000.0;
// Voicenux answers the handshake at once, unless it asks you first whether the plugin may in.
const ASK_AFTER: f64 = 1000.0;
// Voicenux waits two minutes for your answer, then turns the plugin away.
const HANDSHAKE_TIMEOUT: f64 = 130_000.0;

/// Is Voicenux running at all? Then a refused connection means the API is off in its settings.
/// Also finds the Flatpak, its processes show up here too.
pub fn voicenux_running(proc: &Path) -> bool {
	let Ok(dirs) = std::fs::read_dir(proc) else { return false };
	dirs.flatten().any(|dir| {
		let pid = dir.file_name();
		pid.to_string_lossy().bytes().all(|b| b.is_ascii_digit())
			&& std::fs::read_to_string(dir.path().join("comm")).is_ok_and(|comm| comm.trim() == "voicenux")
	})
}

/// The answer to an action, sent right away. Awaiting it waits for Voicenux.
pub type Reply<T = Value> = Pin<Box<dyn Future<Output = Res<T>>>>;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Voice {
	pub id: String,
	pub name: String,
	pub icon: String,
	pub favorite: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Sound {
	pub id: String,
	pub name: String,
	pub icon: String,
	/// It has a star. Voicenux 0.3.0 and older do not say.
	pub favorite: bool,
}

/// What Voicenux says right now: the voice in use, the switches and the soundboard volume.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct State {
	pub voice: Option<Voice>,
	pub voice_changer: bool,
	pub hear_myself: bool,
	pub background: bool,
	pub mute: bool,
	pub sound_volume: Option<f64>,
	pub hear_sounds: bool,
	/// Voicenux listens to the microphone for a key, and others do not hear you.
	pub listening: bool,
}

impl State {
	/// The id of the voice in use.
	pub fn voice_id(&self) -> Option<&str> {
		self.voice.as_ref().map(|v| v.id.as_str())
	}
}

/// The items of a list from Voicenux. An item that is not what it should be is left out.
fn list<T: DeserializeOwned>(data: &Value) -> Vec<T> {
	data.as_array().into_iter().flatten().filter_map(|item| T::deserialize(item).ok()).collect()
}

// A picture: still on its way (with everyone who waits for it), or there.
enum Icon {
	Loading(Vec<oneshot::Sender<Option<String>>>),
	Done(Option<String>),
}

#[derive(Default)]
struct Data {
	state: Option<State>,
	voices: Vec<Voice>,
	sounds: Vec<Sound>,
	playing: Vec<String>,
	// Pictures by name, as base64 PNG. They never change while Voicenux runs.
	icons: HashMap<String, Icon>,
	// Goes up with every new Voicenux, so a picture that comes late from the old one is dropped.
	generation: u64,
}

struct Socket {
	serial: u64,
	tx: mpsc::UnboundedSender<Message>,
	open: Rc<Cell<bool>>,
	greeted: Rc<Cell<bool>>,
	task: AbortHandle,
}

/// For tests: answers like the real API and writes down every call.
#[cfg(test)]
#[derive(Default)]
pub struct Fake {
	pub calls: Vec<(String, Option<String>)>,
	/// Like Voicenux 0.1.0, which does not know sound-restart, sound-toggle and sound-overlap,
	/// nor listening.
	pub old: bool,
	/// What transcribe says you said.
	pub said: String,
}

/// How the connection to Voicenux is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
	/// Not started.
	Idle,
	/// Waiting for the socket.
	Connecting,
	/// Voicenux said hello and sent its state, actions work.
	Connected,
	/// Voicenux does not run.
	Offline,
	/// Voicenux runs, but "Let other apps control Voicenux" is off.
	Disabled,
	/// Voicenux asks you whether the plugin may control it.
	Asking,
	/// Voicenux did not let the plugin in: you said no, or did not answer in time.
	Refused,
	/// Anything else, see last_error.
	Error,
}

impl Status {
	/// The name the settings pages know.
	pub fn name(self) -> &'static str {
		match self {
			Status::Idle => "idle",
			Status::Connecting => "connecting",
			Status::Connected => "connected",
			Status::Offline => "offline",
			Status::Disabled => "disabled",
			Status::Asking => "asking",
			Status::Refused => "refused",
			Status::Error => "error",
		}
	}

	/// Voicenux cannot be reached, and it is not just starting.
	pub fn is_failure(self) -> bool {
		matches!(self, Status::Offline | Status::Disabled | Status::Asking | Status::Refused | Status::Error)
	}
}

/// What changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
	/// Everything: connected, or gone.
	All,
	State,
	Voices,
	Sounds,
	Playing,
}

pub struct VoicenuxClient {
	pub host: String,
	pub port: u16,
	running: Box<dyn Fn() -> bool>,
	status: Cell<Status>,
	last_error: RefCell<String>,
	version: RefCell<Value>,
	api: Cell<Option<f64>>,
	data: RefCell<Data>,
	ws: RefCell<Option<Socket>>,
	pending: RefCell<HashMap<u64, oneshot::Sender<Res<Value>>>>,
	next_id: Cell<u64>,
	serial: Cell<u64>,
	retry: Cell<usize>,
	retry_timer: RefCell<Option<Timer>>,
	stopped: Cell<bool>,
	me: Weak<VoicenuxClient>,
	/// (status, error)
	pub on_status: Emitter<(Status, String)>,
	pub on_change: Emitter<Change>,
	#[cfg(test)]
	pub fake: RefCell<Option<Fake>>,
}

impl VoicenuxClient {
	pub fn new() -> Rc<Self> {
		Self::with("127.0.0.1", PORT, Box::new(|| voicenux_running(Path::new("/proc"))))
	}

	pub fn with(host: &str, port: u16, running: Box<dyn Fn() -> bool>) -> Rc<Self> {
		Rc::new_cyclic(|me| VoicenuxClient {
			host: host.to_string(),
			port,
			running,
			status: Cell::new(Status::Idle),
			last_error: RefCell::new(String::new()),
			version: RefCell::new(Value::Null),
			api: Cell::new(None),
			data: RefCell::new(Data::default()),
			ws: RefCell::new(None),
			pending: RefCell::new(HashMap::new()),
			next_id: Cell::new(1),
			serial: Cell::new(0),
			retry: Cell::new(0),
			retry_timer: RefCell::new(None),
			stopped: Cell::new(true),
			me: me.clone(),
			on_status: Emitter::default(),
			on_change: Emitter::default(),
			#[cfg(test)]
			fake: RefCell::new(None),
		})
	}

	pub fn status(&self) -> Status {
		self.status.get()
	}

	pub fn last_error(&self) -> String {
		self.last_error.borrow().clone()
	}

	/// The version of Voicenux, once it said hello.
	pub fn version(&self) -> Value {
		self.version.borrow().clone()
	}

	pub fn connected(&self) -> bool {
		self.status.get() == Status::Connected
	}

	/// Ready for keys: connected and the first state is in.
	pub fn ready(&self) -> bool {
		self.connected() && self.data.borrow().state.is_some()
	}

	pub fn state(&self) -> Option<State> {
		self.data.borrow().state.clone()
	}

	pub fn voices(&self) -> Vec<Voice> {
		self.data.borrow().voices.clone()
	}

	pub fn sounds(&self) -> Vec<Sound> {
		self.data.borrow().sounds.clone()
	}

	/// The ids of the sounds that play.
	pub fn playing(&self) -> Vec<String> {
		self.data.borrow().playing.clone()
	}

	pub fn voice(&self, id: &str) -> Option<Voice> {
		self.data.borrow().voices.iter().find(|v| v.id == id).cloned()
	}

	pub fn sound(&self, id: &str) -> Option<Sound> {
		self.data.borrow().sounds.iter().find(|s| s.id == id).cloned()
	}

	fn reset(&self) {
		let mut data = self.data.borrow_mut();
		let generation = data.generation + 1;
		*data = Data { generation, ..Data::default() };
	}

	pub fn start(&self) {
		if !self.stopped.get() {
			return;
		}
		self.stopped.set(false);
		self.open();
	}

	pub fn stop(&self) {
		self.stopped.set(true);
		self.clear_retry();
		self.drop_socket();
		self.cleanup("stopped");
		self.set_status(Status::Idle, "");
	}

	/// Try again now, for example after the user turned the API on.
	pub fn reconnect(&self) {
		self.clear_retry();
		self.retry.set(0);
		if self.ws.borrow().is_some() {
			self.drop_socket();
			self.cleanup("reconnect");
		}
		if !self.stopped.get() {
			self.open();
		}
	}

	fn clear_retry(&self) {
		if let Some(timer) = self.retry_timer.borrow_mut().take() {
			timer.clear();
		}
	}

	// Close the socket without running its close handler.
	fn drop_socket(&self) {
		let socket = self.ws.borrow_mut().take();
		if let Some(socket) = socket {
			if socket.open.get() {
				let _ = socket.tx.send(Message::Close(Some(CloseFrame { code: CloseCode::Normal, reason: "".into() })));
			}
			socket.task.abort();
		}
	}

	fn set_status(&self, status: Status, error: &str) {
		let changed = self.status.get() != status || *self.last_error.borrow() != error;
		self.status.set(status);
		*self.last_error.borrow_mut() = error.to_string();
		if changed {
			self.on_status.emit(&(status, error.to_string()));
		}
	}

	fn is_current(&self, serial: u64) -> bool {
		self.ws.borrow().as_ref().is_some_and(|s| s.serial == serial)
	}

	fn open(&self) {
		if self.stopped.get() {
			return;
		}
		let Some(me) = self.me.upgrade() else { return };
		// While retrying, keep showing why the last try failed.
		if !self.status.get().is_failure() {
			self.set_status(Status::Connecting, "");
		}
		let serial = self.serial.get() + 1;
		self.serial.set(serial);
		let (tx, rx) = mpsc::unbounded_channel();
		let open = Rc::new(Cell::new(false));
		let greeted = Rc::new(Cell::new(false));
		let url = format!("ws://{}:{}", self.host, self.port);
		let task = spawn_local(run_socket(me, serial, url, rx, open.clone()));
		*self.ws.borrow_mut() = Some(Socket { serial, tx, open: open.clone(), greeted, task: task.abort_handle() });
		let me = self.me.clone();
		set_timeout(ASK_AFTER, move || {
			let Some(me) = me.upgrade() else { return };
			if !me.is_current(serial) || open.get() {
				return;
			}
			if (me.running)() {
				me.set_status(Status::Asking, "");
			} else {
				me.no_hello();
			}
		});
	}

	/// The handshake is done: Voicenux says hello right away.
	fn wait_for_hello(&self, serial: u64) {
		let Some(greeted) = self.ws.borrow().as_ref().map(|s| s.greeted.clone()) else { return };
		let me = self.me.clone();
		set_timeout(HELLO_TIMEOUT, move || {
			let Some(me) = me.upgrade() else { return };
			if me.is_current(serial) && !greeted.get() {
				me.no_hello();
			}
		});
	}

	fn no_hello(&self) {
		self.drop_socket();
		self.cleanup("no hello");
		self.set_status(Status::Error, &format!("Something else uses port {}", self.port));
		self.schedule_retry();
	}

	fn on_message(&self, text: &str) {
		let Ok(message) = serde_json::from_str::<Value>(text) else { return };
		if let Some(event) = message["event"].as_str().filter(|e| !e.is_empty()) {
			let data = &message["data"];
			if event == "hello" {
				if let Some(socket) = self.ws.borrow().as_ref() {
					socket.greeted.set(true);
				}
				self.retry.set(0);
				*self.version.borrow_mut() = data["version"].clone();
				self.api.set(data["api"].as_f64());
			}
			self.on_event(event, data);
		} else if let Some(id) = message["id"].as_u64() {
			let Some(pending) = self.pending.borrow_mut().remove(&id) else { return };
			let result = if message["ok"] == true {
				Ok(message["result"].clone())
			} else {
				match message["error"].as_str() {
					Some(error) if !error.is_empty() => Err(Error(error.to_string())),
					_ => Err(Error::from("Voicenux said no")),
				}
			};
			let _ = pending.send(result);
		}
	}

	/// An event from Voicenux, like `{"event": "state", "data": {…}}`.
	pub fn on_event(&self, event: &str, data: &Value) {
		match event {
			// "connected" waits for the first state, so keys draw with data.
			"hello" => {}
			"state" => {
				{
					let mut d = self.data.borrow_mut();
					d.state = Some(State::deserialize(data).unwrap_or_default());
					if data["playing"].is_array() {
						d.playing = list(&data["playing"]);
					}
				}
				if self.connected() {
					self.on_change.emit(&Change::State);
					return;
				}
				match self.api.get() {
					Some(api) if api > API => self.set_status(
						Status::Connected,
						&format!("Voicenux speaks API {}, this plugin {}. Update the plugin.", js::fmt(api), js::fmt(API)),
					),
					_ => self.set_status(Status::Connected, ""),
				}
				self.on_change.emit(&Change::All);
			}
			"voices" => {
				self.data.borrow_mut().voices = list(data);
				if self.connected() {
					self.on_change.emit(&Change::Voices);
				}
			}
			"sounds" => {
				self.data.borrow_mut().sounds = list(data);
				if self.connected() {
					self.on_change.emit(&Change::Sounds);
				}
			}
			"playing" => {
				self.data.borrow_mut().playing = list(data);
				if self.connected() {
					self.on_change.emit(&Change::Playing);
				}
			}
			_ => {}
		}
	}

	fn closed(&self, serial: u64) {
		if !self.is_current(serial) {
			return;
		}
		*self.ws.borrow_mut() = None;
		let was_connected = self.connected();
		self.cleanup("connection closed");
		self.reset();
		if (self.running)() {
			self.set_status(
				Status::Disabled,
				if was_connected { "Voicenux closed the connection" } else { "Other apps may not control Voicenux" },
			);
		} else {
			self.set_status(Status::Offline, "Voicenux is not running");
		}
		if was_connected {
			self.on_change.emit(&Change::All);
		}
		self.schedule_retry();
	}

	/// Voicenux said no (403). It asks again once you turn Other apps off and on, or restart it.
	fn refused(&self, serial: u64) {
		if !self.is_current(serial) {
			return;
		}
		*self.ws.borrow_mut() = None;
		self.cleanup("not allowed");
		self.set_status(Status::Refused, "");
		self.schedule_retry();
	}

	fn schedule_retry(&self) {
		if self.stopped.get() {
			return;
		}
		self.clear_retry();
		let delay = RETRY_DELAYS[self.retry.get().min(RETRY_DELAYS.len() - 1)];
		self.retry.set(self.retry.get() + 1);
		let me = self.me.clone();
		let timer = set_timeout(delay, move || {
			if let Some(me) = me.upgrade() {
				me.open();
			}
		});
		*self.retry_timer.borrow_mut() = Some(timer);
	}

	fn cleanup(&self, reason: &str) {
		let pending: Vec<_> = self.pending.borrow_mut().drain().collect();
		for (_, sender) in pending {
			let _ = sender.send(Err(format!("Voicenux not connected ({reason})").into()));
		}
	}

	/// An action from API.md. Actions that change something answer with the new state.
	pub fn call(&self, action: &str, arg: Option<&str>) -> Reply {
		self.call_waiting(action, arg, CALL_TIMEOUT)
	}

	/// An action that takes longer, like transcribe: waits up to `ms` for the answer.
	pub fn call_waiting(&self, action: &str, arg: Option<&str>, ms: f64) -> Reply {
		#[cfg(test)]
		if self.fake.borrow().is_some() {
			return self.fake_call(action, arg);
		}
		let socket = self.ws.borrow();
		let Some(socket) = socket.as_ref().filter(|s| s.open.get()) else {
			return Box::pin(async { Err(Error::from("Voicenux not connected")) });
		};
		let id = self.next_id.get();
		self.next_id.set(id + 1);
		let (tx, rx) = oneshot::channel();
		self.pending.borrow_mut().insert(id, tx);
		let mut message = crate::obj! { "id" => id, "action" => action };
		if let Some(arg) = arg {
			message["arg"] = arg.into();
		}
		let _ = socket.tx.send(Message::Text(js::stringify(&message).into()));
		// The time starts now, not when the reply is first awaited.
		let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(ms / 1000.0);
		let me = self.me.clone();
		Box::pin(async move {
			let result = match tokio::time::timeout_at(deadline, rx).await {
				Ok(Ok(result)) => result?,
				Ok(Err(_)) => return Err(Error::from("Voicenux not connected")),
				Err(_) => {
					if let Some(me) = me.upgrade() {
						me.pending.borrow_mut().remove(&id);
					}
					return Err(Error::from("Voicenux did not answer in time"));
				}
			};
			// The new state comes as an event too, but this way the key sees it before it draws.
			if result.get("voice_changer").is_some()
				&& let Some(me) = me.upgrade()
			{
				me.on_event("state", &result);
			}
			Ok(result)
		})
	}

	/// The picture, if Voicenux sent it already.
	pub fn cached_icon(&self, name: &str) -> Option<String> {
		match self.data.borrow().icons.get(name) {
			Some(Icon::Done(png)) => png.clone(),
			_ => None,
		}
	}

	/// The picture of a voice or sound as base64 PNG, or None.
	pub fn icon(&self, name: &str) -> Pin<Box<dyn Future<Output = Option<String>>>> {
		if name.is_empty() {
			return Box::pin(async { None });
		}
		let (tx, rx) = oneshot::channel();
		let generation = {
			let mut data = self.data.borrow_mut();
			match data.icons.get_mut(name) {
				Some(Icon::Done(png)) => {
					let png = png.clone();
					return Box::pin(async move { png });
				}
				Some(Icon::Loading(waiting)) => {
					waiting.push(tx);
					return Box::pin(async move { rx.await.ok().flatten() });
				}
				None => {
					data.icons.insert(name.to_string(), Icon::Loading(vec![tx]));
					data.generation
				}
			}
		};
		let reply = self.call("icon", Some(name));
		let me = self.me.clone();
		let name = name.to_string();
		spawn_local(async move {
			let result = reply.await;
			let Some(me) = me.upgrade() else { return };
			let png = result.as_ref().ok().and_then(|r| r.get("png")).and_then(Value::as_str).map(String::from);
			let waiting = {
				let mut data = me.data.borrow_mut();
				if data.generation != generation {
					return;
				}
				let waiting = match data.icons.remove(&name) {
					Some(Icon::Loading(waiting)) => waiting,
					_ => Vec::new(),
				};
				// A failed try is not kept: the next key asks again.
				if result.is_ok() {
					data.icons.insert(name, Icon::Done(png.clone()));
				}
				waiting
			};
			for tx in waiting {
				let _ = tx.send(png.clone());
			}
		});
		Box::pin(async move { rx.await.ok().flatten() })
	}

	#[cfg(test)]
	fn fake_call(&self, action: &str, arg: Option<&str>) -> Reply {
		let mut fake = self.fake.borrow_mut();
		let Some(fake) = fake.as_mut() else { return Box::pin(async { Ok(Value::Null) }) };
		// No pictures, and the tests only look at what the keys do.
		if action == "icon" {
			return Box::pin(async { Ok(Value::Null) });
		}
		fake.calls.push((action.to_string(), arg.map(String::from)));
		if fake.old
			&& matches!(action, "sound-restart" | "sound-toggle" | "sound-overlap" | "listen" | "transcribe" | "find-sound" | "find-voice")
		{
			let error = format!("unknown action \"{action}\", see API.md");
			return Box::pin(async move { Err(Error(error)) });
		}
		let voice = arg.and_then(|id| self.voice(id));
		let mut data = self.data.borrow_mut();
		let state = data.state.get_or_insert_with(State::default);
		let flip = |now: bool| match arg {
			Some("on") => true,
			Some("off") => false,
			_ => !now,
		};
		match action {
			"voice" => {
				state.voice = voice;
				state.voice_changer = true;
			}
			"voice-changer" => state.voice_changer = flip(state.voice_changer),
			"mute" => state.mute = flip(state.mute),
			"sound-volume" => state.sound_volume = arg.and_then(|a| a.parse().ok()),
			"listen" => state.listening = flip(state.listening),
			"transcribe" => {
				state.listening = false;
				let text = Value::from(fake.said.clone());
				return Box::pin(async move { Ok(crate::obj! { "text" => text }) });
			}
			// Found sounds stay off the soundboard, like in Voicenux: "found-1" has the most likes.
			"find-sound" => {
				let arg = arg.unwrap_or_default();
				let (name, rank) = match serde_json::from_str::<Value>(arg) {
					Ok(Value::Object(o)) => (o["words"].as_str().unwrap_or_default().to_string(), o["rank"].as_u64().unwrap_or(1)),
					_ => (arg.to_string(), 1),
				};
				let id = format!("found-{rank}");
				return Box::pin(async move { Ok(crate::obj! { "id" => id, "name" => name, "icon" => "collision", "rank" => rank }) });
			}
			"find-voice" => {
				let said = arg.unwrap_or_default().trim_matches(|c: char| c.is_ascii_punctuation()).to_lowercase();
				let found = data.voices.iter().find(|v| v.name.to_lowercase() == said).cloned();
				return Box::pin(async move {
					let v = found.ok_or_else(|| Error(format!("no voice called \"{said}\"")))?;
					Ok(crate::obj! { "id" => v.id, "name" => v.name, "icon" => v.icon })
				});
			}
			_ => {}
		}
		Box::pin(async { Ok(Value::Null) })
	}

	/// For tests: a Voicenux that is connected, with this state, these voices and sounds.
	#[cfg(test)]
	pub fn fake_connected(&self, state: State, voices: Vec<Voice>, sounds: Vec<Sound>) {
		*self.fake.borrow_mut() = Some(Fake::default());
		self.status.set(Status::Connected);
		let mut data = self.data.borrow_mut();
		data.state = Some(state);
		data.voices = voices;
		data.sounds = sounds;
	}

	#[cfg(test)]
	pub fn set_playing(&self, playing: Vec<String>) {
		self.data.borrow_mut().playing = playing;
	}

	#[cfg(test)]
	pub fn set_sounds(&self, sounds: Vec<Sound>) {
		self.data.borrow_mut().sounds = sounds;
	}

	#[cfg(test)]
	pub fn set_state(&self, f: impl FnOnce(&mut State)) {
		if let Some(state) = self.data.borrow_mut().state.as_mut() {
			f(state);
		}
	}
}

async fn run_socket(me: Rc<VoicenuxClient>, serial: u64, url: String, mut rx: mpsc::UnboundedReceiver<Message>, open: Rc<Cell<bool>>) {
	let handshake = tokio::time::timeout(Duration::from_secs_f64(HANDSHAKE_TIMEOUT / 1000.0), tokio_tungstenite::connect_async(url));
	let stream = match handshake.await {
		Ok(Ok((stream, _))) => stream,
		Ok(Err(tungstenite::Error::Http(response))) if response.status() == 403 => {
			me.refused(serial);
			return;
		}
		_ => {
			me.closed(serial);
			return;
		}
	};
	if !me.is_current(serial) {
		return;
	}
	open.set(true);
	me.wait_for_hello(serial);
	let (mut sink, mut stream) = stream.split();
	let writer = spawn_local(async move {
		while let Some(message) = rx.recv().await {
			if sink.send(message).await.is_err() {
				break;
			}
		}
	});
	while let Some(message) = stream.next().await {
		if !me.is_current(serial) {
			writer.abort();
			return;
		}
		match message {
			Ok(Message::Text(text)) => me.on_message(&text),
			Ok(Message::Close(_)) | Err(_) => break,
			Ok(_) => {}
		}
	}
	open.set(false);
	writer.abort();
	me.closed(serial);
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn connected_once_the_first_state_is_in() {
		let client = VoicenuxClient::with("127.0.0.1", PORT, Box::new(|| false));
		let changes = Rc::new(RefCell::new(Vec::new()));
		let seen = changes.clone();
		client.on_change.on(move |change| seen.borrow_mut().push(*change));
		client.on_event("hello", &json!({ "app": "voicenux", "version": "0.1.0", "api": 1 }));
		assert!(!client.connected());
		client.on_event("state", &json!({ "voice": { "id": "robot" }, "playing": [] }));
		assert_eq!(client.status(), Status::Connected);
		client.on_event("voices", &json!([{ "id": "deep", "name": "Deep", "icon": "bear", "favorite": true }]));
		client.on_event("playing", &json!(["airhorn"]));
		assert_eq!(client.playing(), ["airhorn"]);
		assert_eq!(client.voice("deep").map(|v| v.name), Some("Deep".to_string()));
		assert_eq!(*changes.borrow(), [Change::All, Change::Voices, Change::Playing]);
	}

	#[test]
	fn lists_leave_out_items_that_are_not_what_they_should_be() {
		let client = VoicenuxClient::with("127.0.0.1", PORT, Box::new(|| false));
		client.on_event("state", &json!({ "voice": null, "sound_volume": 55, "mute": true, "playing": ["airhorn", 3] }));
		let state = client.state().unwrap_or_default();
		assert_eq!((state.voice, state.sound_volume, state.mute, state.hear_sounds), (None, Some(55.0), true, false));
		assert_eq!(client.playing(), ["airhorn"]);
		client.on_event("sounds", &json!([{ "id": "coin", "name": "Coin", "icon": "coin" }, "not a sound"]));
		assert_eq!(client.sounds(), [Sound { id: "coin".into(), name: "Coin".into(), icon: "coin".into(), favorite: false }]);
	}

	#[test]
	fn finds_a_running_voicenux_by_its_process_name() {
		let proc = std::env::temp_dir().join(format!("proc-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&proc);
		std::fs::create_dir_all(proc.join("12")).unwrap();
		std::fs::write(proc.join("12").join("comm"), "bash\n").unwrap();
		assert!(!voicenux_running(&proc));
		std::fs::create_dir_all(proc.join("34")).unwrap();
		std::fs::write(proc.join("34").join("comm"), "voicenux\n").unwrap();
		assert!(voicenux_running(&proc));
		let _ = std::fs::remove_dir_all(&proc);
	}

	#[test]
	fn waits_while_voicenux_asks() {
		use tokio::io::AsyncWriteExt;
		let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
		tokio::task::LocalSet::new().block_on(&rt, async {
			let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
			let port = listener.local_addr().unwrap().port();
			let client = VoicenuxClient::with("127.0.0.1", port, Box::new(|| true));
			client.start();
			// Voicenux holds the handshake while it asks you.
			let (mut socket, _) = listener.accept().await.unwrap();
			tokio::time::sleep(Duration::from_secs_f64((HELLO_TIMEOUT + 500.0) / 1000.0)).await;
			assert_eq!(client.status(), Status::Asking);
			// You said no.
			socket.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n").await.unwrap();
			tokio::time::sleep(Duration::from_millis(200)).await;
			assert_eq!(client.status(), Status::Refused);
			client.stop();
		});
	}
}
