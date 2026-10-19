// End-to-end test: presses keys on the virtual deck (through OpenDeck) and checks Voicenux.
// Needs: Voicenux with "Let other apps control Voicenux" on, OpenDeck with this plugin and the
// virtual deck from FeloDeck/obs-tools, and the profile from test/e2e/layout.json
// (cargo xtask profile test/e2e/layout.json, with OpenDeck stopped meanwhile).
//
//   cargo xtask e2e [test name filter]
use std::time::{Duration, Instant};

use serde_json::Value;
use ureq::Agent;

use crate::Result;

fn sleep(ms: u64) {
	std::thread::sleep(Duration::from_millis(ms));
}

// encodeURIComponent
fn encode(text: &str) -> String {
	text.bytes()
		.map(|b| if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
		.collect()
}

struct Env {
	agent: Agent,
	deck: String,
	api: String,
}

impl Env {
	fn new() -> Env {
		// Voicenux answers errors with status 400 and the reason in the body.
		let agent: Agent = Agent::config_builder().http_status_as_error(false).build().into();
		Env {
			agent,
			deck: std::env::var("VIRTUAL_DECK_URL").unwrap_or_else(|_| "http://127.0.0.1:57190".into()),
			api: std::env::var("VOICENUX_URL").unwrap_or_else(|_| "http://127.0.0.1:41967/api".into()),
		}
	}

	fn json(&self, url: &str, post: bool) -> Result<Value> {
		let response = if post { self.agent.post(url).send_empty() } else { self.agent.get(url).call() };
		let text = response.map_err(|e| format!("{url}: {e}"))?.body_mut().read_to_string().map_err(|e| format!("{url}: {e}"))?;
		Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
	}

	fn deck_post(&self, path: &str) -> Result {
		self.json(&format!("{}{path}", self.deck), true).map(|_| ())
	}

	fn press(&self, key: usize, ms: u64) -> Result {
		self.deck_post(&format!("/api/key/{key}/press?ms={ms}"))
	}

	fn rotate(&self, dial: usize, ticks: i32) -> Result {
		self.deck_post(&format!("/api/dial/{dial}/rotate?ticks={ticks}"))
	}

	fn press_dial(&self, dial: usize) -> Result {
		self.deck_post(&format!("/api/dial/{dial}/press"))
	}

	fn key_image(&self, key: usize) -> Result<Value> {
		Ok(self.json(&format!("{}/api/state", self.deck), false)?["keys"][key].clone())
	}

	/// The state of Voicenux.
	fn vn(&self) -> Result<Value> {
		Ok(self.json(&format!("{}/state", self.api), false)?["result"].clone())
	}

	fn voice(&self) -> Result<String> {
		Ok(self.vn()?["voice"]["id"].as_str().unwrap_or("").to_string())
	}

	fn playing(&self) -> Result<Vec<String>> {
		Ok(self.vn()?["playing"].as_array().map(|p| p.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default())
	}

	fn voices(&self) -> Result<Vec<Value>> {
		Ok(self.json(&format!("{}/voices", self.api), false)?["result"].as_array().cloned().unwrap_or_default())
	}

	fn act(&self, action: &str, arg: Option<&str>) -> Result<Value> {
		let url = match arg {
			Some(arg) => format!("{}/{action}/{}", self.api, encode(arg)),
			None => format!("{}/{action}", self.api),
		};
		let answer = self.json(&url, true)?;
		if answer["ok"] != true {
			return Err(format!("{action}: {}", answer["error"]));
		}
		Ok(answer["result"].clone())
	}
}

fn wait_for(what: &str, timeout: u64, mut check: impl FnMut() -> Result<bool>) -> Result {
	let until = Instant::now() + Duration::from_millis(timeout);
	while Instant::now() < until {
		if check()? {
			return Ok(());
		}
		sleep(80);
	}
	Err(format!("timeout waiting for {what}"))
}

fn check(condition: bool, message: &str) -> Result {
	if condition { Ok(()) } else { Err(message.into()) }
}

struct Suite {
	filter: String,
	count: usize,
	failed: usize,
}

impl Suite {
	fn test(&mut self, name: &str, test: impl FnOnce() -> Result) {
		if !self.filter.is_empty() && !name.contains(&self.filter) {
			return;
		}
		self.count += 1;
		let start = Instant::now();
		match test() {
			Ok(()) => println!("ok   {name} ({} ms)", start.elapsed().as_millis()),
			Err(error) => {
				self.failed += 1;
				println!("FAIL {name}: {error}");
			}
		}
	}
}

// The next voice after `from` in the list, `by` steps on.
fn after(list: &[Value], from: &str, by: usize) -> String {
	let at = list.iter().position(|v| v["id"] == from).unwrap_or(0);
	list.get((at + by) % list.len().max(1)).and_then(|v| v["id"].as_str()).unwrap_or("").to_string()
}

pub fn run(args: &[String]) -> Result {
	let e = Env::new();
	let mut s = Suite { filter: args.first().cloned().unwrap_or_default(), count: 0, failed: 0 };

	// A known start: Robot, everything as layout.json expects it.
	e.act("voice", Some("robot"))?;
	e.act("mute", Some("off"))?;
	e.act("stop-sounds", None)?;
	e.act("sound-volume", Some("80"))?;
	sleep(600);

	s.test("a voice key uses its voice and gets the frame", || {
		let before = e.key_image(1)?;
		e.press(1, 80)?;
		wait_for("voice = deep", 4000, || Ok(e.voice()? == "deep"))?;
		wait_for("key 1 shows the frame", 4000, || Ok(e.key_image(1)? != before))?;
		wait_for("key 0 loses its frame", 4000, || Ok(e.key_image(0)? != before))
	});

	s.test("pressed again, the toggle voice key goes back to your own voice", || {
		e.press(1, 80)?;
		wait_for("voice changer off", 4000, || Ok(e.vn()?["voice_changer"] == false))?;
		e.press(1, 80)?;
		wait_for("voice changer on with deep", 4000, || {
			let state = e.vn()?;
			Ok(state["voice_changer"] == true && state["voice"]["id"] == "deep")
		})
	});

	s.test("push to change voice: changed only while the key is held", || {
		e.act("voice-changer", Some("off"))?;
		sleep(300);
		let off = e.key_image(5)?;
		e.deck_post("/api/key/5/down")?;
		wait_for("voice changer on", 4000, || Ok(e.vn()?["voice_changer"] == true))?;
		wait_for("key 5 turns white", 4000, || Ok(e.key_image(5)? != off))?;
		e.deck_post("/api/key/5/up")?;
		wait_for("voice changer off", 4000, || Ok(e.vn()?["voice_changer"] == false))?;
		e.act("voice-changer", Some("on")).map(|_| ())
	});

	s.test("a voice used while held comes back when you let go", || {
		e.act("voice", Some("robot"))?;
		// Voicenux tells the plugin about changes made elsewhere a little later.
		sleep(300);
		e.deck_post("/api/key/2/down")?;
		wait_for("voice = chipmunk", 4000, || Ok(e.voice()? == "chipmunk"))?;
		sleep(300);
		e.deck_post("/api/key/2/up")?;
		wait_for("voice = robot", 4000, || Ok(e.voice()? == "robot"))?;
		e.act("voice-changer", Some("off"))?;
		sleep(300);
		e.press(2, 300)?;
		wait_for("voice changer off again", 4000, || {
			let state = e.vn()?;
			Ok(state["voice"]["id"] == "robot" && state["voice_changer"] == false)
		})?;
		e.act("voice-changer", Some("on")).map(|_| ())
	});

	s.test("mute key mutes and unmutes", || {
		e.press(6, 80)?;
		wait_for("muted", 4000, || Ok(e.vn()?["mute"] == true))?;
		e.press(6, 80)?;
		wait_for("unmuted", 4000, || Ok(e.vn()?["mute"] == false))
	});

	s.test("switch keys flip hear myself and hear sounds", || {
		for (key, field) in [(7, "hear_myself"), (9, "hear_sounds")] {
			let before = e.vn()?[field] == true;
			e.press(key, 80)?;
			wait_for(&format!("{field} flips"), 4000, || Ok((e.vn()?[field] == true) != before))?;
			e.press(key, 80)?;
			wait_for(&format!("{field} back"), 4000, || Ok((e.vn()?[field] == true) == before))?;
		}
		Ok(())
	});

	s.test("next voice key uses the next voice", || {
		let list = e.voices()?;
		let favorites: Vec<Value> = list.iter().filter(|v| v["favorite"] == true).cloned().collect();
		let ring = if favorites.is_empty() { list } else { favorites };
		let expected = after(&ring, &e.voice()?, 1);
		e.press(11, 80)?;
		wait_for(&format!("voice = {expected}"), 4000, || Ok(e.voice()? == expected))
	});

	s.test("random voice key picks another voice", || {
		let start = e.voice()?;
		for _ in 0..4 {
			e.press(10, 80)?;
			sleep(300);
			if e.voice()? != start {
				return Ok(());
			}
		}
		Err("the voice stayed the same four times".into())
	});

	s.test("a sound key plays its sound and gets the frame", || {
		let before = e.key_image(3)?;
		e.press(3, 80)?;
		wait_for("bruh plays", 4000, || Ok(e.playing()?.iter().any(|p| p == "bruh")))?;
		wait_for("key 3 shows the frame", 4000, || Ok(e.key_image(3)? != before))?;
		wait_for("key 3 back without frame", 8000, || Ok(e.key_image(3)? == before))
	});

	s.test("a toggle sound key stops its sound", || {
		e.press(4, 80)?;
		wait_for("sad violin plays", 4000, || Ok(e.playing()?.iter().any(|p| p == "sad-violin")))?;
		sleep(300);
		e.press(4, 80)?;
		wait_for("sad violin stops", 4000, || Ok(!e.playing()?.iter().any(|p| p == "sad-violin")))
	});

	s.test("stop sounds key stops everything and is white while sounds play", || {
		// The sound of the test before may still be going out.
		wait_for("nothing plays", 4000, || Ok(e.playing()?.is_empty()))?;
		sleep(400);
		let idle = e.key_image(12)?;
		e.act("sound", Some("sad-violin"))?;
		wait_for("key 12 turns white", 4000, || Ok(e.key_image(12)? != idle))?;
		e.press(12, 80)?;
		wait_for("nothing plays", 4000, || Ok(e.playing()?.is_empty()))?;
		wait_for("key 12 gray again", 4000, || Ok(e.key_image(12)? == idle))
	});

	s.test("random sound key plays a sound and shows the next one", || {
		e.act("stop-sounds", None)?;
		wait_for("nothing plays", 4000, || Ok(e.playing()?.is_empty()))?;
		sleep(400);
		let shown = e.key_image(8)?;
		e.press(8, 80)?;
		wait_for("a sound plays", 4000, || Ok(e.playing()?.len() == 1))?;
		wait_for("key 8 shows another sound", 4000, || Ok(e.key_image(8)? != shown))?;
		e.act("stop-sounds", None).map(|_| ())
	});

	s.test("soundboard volume key turns it down", || {
		e.press(13, 80)?;
		wait_for("volume 60", 4000, || Ok(e.vn()?["sound_volume"] == 60))?;
		e.act("sound-volume", Some("80")).map(|_| ())
	});

	s.test("a key without a voice does nothing", || {
		let before = e.vn()?;
		e.press(14, 80)?;
		sleep(400);
		check(e.vn()? == before, "the state changed")
	});

	s.test("voice dial picks on turn and uses on press", || {
		e.act("voice", Some("robot"))?;
		sleep(300);
		let expected = after(&e.voices()?, "robot", 2);
		e.rotate(0, 2)?;
		sleep(400);
		check(e.voice()? == "robot", "turning changed the voice already")?;
		e.press_dial(0)?;
		wait_for(&format!("voice = {expected}"), 4000, || Ok(e.voice()? == expected))?;
		e.press_dial(0)?;
		wait_for("voice changer off", 4000, || Ok(e.vn()?["voice_changer"] == false))?;
		e.act("voice-changer", Some("on")).map(|_| ())
	});

	s.test("soundboard dial turns the volume and stops sounds", || {
		e.rotate(1, 1)?;
		wait_for("volume 90", 4000, || Ok(e.vn()?["sound_volume"] == 90))?;
		e.rotate(1, -3)?;
		wait_for("volume 60", 4000, || Ok(e.vn()?["sound_volume"] == 60))?;
		e.act("sound", Some("sad-violin"))?;
		wait_for("plays", 4000, || Ok(!e.playing()?.is_empty()))?;
		e.press_dial(1)?;
		wait_for("stopped", 4000, || Ok(e.playing()?.is_empty()))?;
		e.act("sound-volume", Some("80")).map(|_| ())
	});

	e.act("voice", Some("robot"))?;
	println!("\n{}/{} passed", s.count - s.failed, s.count);
	if s.failed > 0 { Err(format!("{} failed", s.failed)) } else { Ok(()) }
}
