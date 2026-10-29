// Keys that listen: hold the key and say what you want. While you hold it, Voicenux listens, and
// others do not hear you.
// Say a Sound finds a sound online, on MyInstants: the one most people liked. Voicenux keeps it off
// the soundboard. A press plays it, a double press takes the one with the next most likes.
// Say a Voice only picks out of your voices in Voicenux, nothing is searched online. It uses the
// voice right away, and a press turns it on or off.
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::time::Instant;

use super::sounds::play_as;
use super::{Behavior, DOUBLE_PRESS_MS, KeyRc, Kind, Press, defaults};
use crate::i18n::t;
use crate::icons::colors;
use crate::render::{Frame, GlyphKey, PictureKey, glyph_key, picture_key, searching_key};
use crate::timers::Timer;
use crate::{Error, Res};

/// Shorter is a press. Voicenux listens from the start anyway, so no word is lost meanwhile.
const SAY_AFTER_MS: f64 = 400.0;
/// The first time, Voicenux downloads its speech recognition (about 970 MB).
const TRANSCRIBE_MS: f64 = 600_000.0;
const FIND_MS: f64 = 120_000.0;
/// How often the key draws the lens again while it looks.
const FRAME_MS: f64 = 50.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum What {
	Sound,
	Voice,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SaySoundSettings {
	/// The id of the sound it found. Not in the list of sounds: Voicenux keeps it off the
	/// soundboard, unless it was there already. So the key keeps its name and picture too.
	sound: String,
	sound_name: String,
	sound_icon: String,
	/// What you said, to find the next sound for it.
	sound_words: String,
	/// Which one of the sounds found it is: 1 has the most likes.
	sound_rank: f64,
	/// How a press plays it: "restart", "toggle" or "overlap".
	play: String,
	show_name: bool,
	name_at: String,
	show_playing: bool,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SayVoiceSettings {
	/// The id of the voice it found.
	voice: String,
	/// "toggle" (the voice on or off) or "use".
	press: String,
	show_name: bool,
	name_at: String,
	show_frame: bool,
}

/// What both keys have.
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Listening {
	/// What you speak: "auto" (English or the language of Voicenux), "en" or "de".
	language: String,
	name_at: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
	#[default]
	Idle,
	Listening,
	Searching,
}

#[derive(Default)]
struct Saying {
	/// When the key went down. None while it is up.
	down: Option<Instant>,
	/// Why Voicenux does not listen to this press, like an older Voicenux.
	deaf: Option<Error>,
	phase: Phase,
	/// What you said, on the key while it searches.
	said: String,
	look: Option<Timer>,
	/// Draws the lens again and again while it looks, since when.
	spin: Option<(Timer, Instant)>,
	/// When the last press played the sound, for a double press.
	played: Option<Instant>,
	/// This press came right after that one.
	second: bool,
}

struct Say(What);

impl Say {
	async fn play(&self, k: &KeyRc) -> Res<()> {
		if self.0 == What::Voice {
			return self.use_voice(k).await;
		}
		let s: SaySoundSettings = k.settings();
		if s.sound.is_empty() {
			return Err(Error::from("Hold the key and say a sound"));
		}
		let mode = if matches!(s.play.as_str(), "toggle" | "overlap") { s.play.as_str() } else { "restart" };
		play_as(k, mode, &s.sound).await.map_err(|error| {
			// Voicenux keeps the last 50 sounds it found, one this old is gone.
			if error.message().starts_with("no sound called") {
				Error::from("This sound is gone. Hold the key to find a new one.")
			} else {
				error
			}
		})
	}

	async fn use_voice(&self, k: &KeyRc) -> Res<()> {
		let s: SayVoiceSettings = k.settings();
		if s.voice.is_empty() {
			return Err(Error::from("Hold the key and say one of your voices"));
		}
		let voice = k.client.voice(&s.voice).ok_or("This voice is gone from Voicenux")?;
		let vn = k.vn();
		if s.press != "use" && vn.voice_id() == Some(voice.id.as_str()) && vn.voice_changer && !k.is_in_multi_action() {
			k.call("voice-changer", Some("off")).await?;
		} else {
			k.call("voice", Some(&voice.id)).await?;
		}
		k.draw();
		Ok(())
	}

	fn show(&self, k: &KeyRc, phase: Phase, said: &str) {
		{
			let mut x = k.own::<Saying>();
			if phase == Phase::Searching && x.spin.is_none() {
				x.spin = Some((k.every(FRAME_MS, |k| k.draw()), Instant::now()));
			}
			if phase != Phase::Searching
				&& let Some((spin, _)) = x.spin.take()
			{
				spin.clear();
			}
			x.phase = phase;
			x.said = said.to_string();
		}
		k.draw();
	}

	/// What you said, then the sound or voice for it.
	async fn find(&self, k: &KeyRc) -> Res<()> {
		let s: Listening = k.settings();
		let language = Some(s.language.as_str()).filter(|l| !l.is_empty() && *l != "auto");
		// For a voice, Voicenux listens for the names of your voices instead of sounds.
		let voices = json!({ "language": language.unwrap_or("auto"), "expect": "voices" }).to_string();
		let arg = if self.0 == What::Voice { Some(voices.as_str()) } else { language };
		let heard = k.client.call_waiting("transcribe", arg, TRANSCRIBE_MS).await?;
		let said = heard["text"].as_str().unwrap_or_default().trim().to_string();
		// What Voicenux heard in the other language: a German "vine boom" is "Weinbumm".
		let also: Vec<&str> = heard["alternatives"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(Value::as_str)
			.map(str::trim)
			.filter(|a| !a.is_empty())
			.collect();
		if said.is_empty() {
			return Err(Error::from(match self.0 {
				What::Sound => "Nothing heard. Hold the key and say a sound.",
				What::Voice => "Nothing heard. Hold the key and say one of your voices.",
			}));
		}
		self.show(k, Phase::Searching, &said);
		match self.0 {
			What::Sound => {
				let arg = if also.is_empty() { said.clone() } else { json!({ "words": said, "alternatives": also }).to_string() };
				let found = k.client.call_waiting("find-sound", Some(&arg), FIND_MS).await?;
				keep_sound(k, &found, &said)
			}
			What::Voice => {
				let mut found = k.call("find-voice", Some(&said)).await;
				for other in also {
					if found.is_ok() {
						break;
					}
					if let Ok(voice) = k.call("find-voice", Some(other)).await {
						found = Ok(voice);
					}
				}
				let found = found.map_err(|error| match error.message().strip_prefix("no voice called ") {
					Some(name) => Error(format!("No voice called {name} in Voicenux. Voices are not searched online.")),
					None => error,
				})?;
				let id = found["id"].as_str().filter(|id| !id.is_empty()).ok_or("Voicenux found no voice")?;
				k.call("voice", Some(id)).await?;
				k.save_settings(&[("voice", id.into())]);
				Ok(())
			}
		}
	}

	/// A double press: the sound with the next most likes for what you said. The first press of
	/// the two played the old one, it stops.
	async fn next(&self, k: &KeyRc) -> Res<()> {
		let s: SaySoundSettings = k.settings();
		if s.sound_words.is_empty() {
			return self.play(k).await;
		}
		let _ = k.call("stop-sound", Some(&s.sound)).await;
		self.show(k, Phase::Searching, &s.sound_words);
		let rank = s.sound_rank.max(1.0) as u64 + 1;
		let arg = json!({ "words": s.sound_words, "rank": rank }).to_string();
		let found = k.client.call_waiting("find-sound", Some(&arg), FIND_MS).await;
		self.show(k, Phase::Idle, "");
		keep_sound(k, &found?, &s.sound_words)?;
		k.clear_error();
		k.show_ok();
		Ok(())
	}
}

/// The key keeps the sound Voicenux found, with the words that found it and where it ranks.
fn keep_sound(k: &KeyRc, found: &Value, said: &str) -> Res<()> {
	let id = found["id"].as_str().filter(|id| !id.is_empty()).ok_or("Voicenux found no sound")?;
	let words = found["words"].as_str().filter(|w| !w.is_empty()).unwrap_or(said);
	k.save_settings(&[
		("sound", id.into()),
		("soundName", found["name"].clone()),
		("soundIcon", found["icon"].clone()),
		("soundWords", words.into()),
		("soundRank", found["rank"].as_u64().unwrap_or(1).into()),
	]);
	Ok(())
}

/// Voicenux 0.3.0 and older cannot listen.
fn newer(error: Error) -> Error {
	if error.message().contains("unknown action") { Error::from("Saying needs a newer Voicenux") } else { error }
}

#[async_trait(?Send)]
impl Behavior for Say {
	fn render_offline(&self, k: &KeyRc) {
		k.glyph_for_title("", "below");
	}

	fn render(&self, k: &KeyRc) {
		let at = k.settings::<Listening>().name_at;
		let (phase, said, since) = {
			let x = k.own::<Saying>();
			(x.phase, x.said.clone(), x.spin.as_ref().map(|(_, since)| *since))
		};
		match phase {
			Phase::Listening => {
				let title = t("Listening");
				k.own_title(&title);
				return k
					.set_image(&glyph_key("mic", &GlyphKey { color: colors::RED, title: &title, title_at: &at, ..Default::default() }));
			}
			// The lens looks around, with what you said once it is known.
			Phase::Searching => {
				k.own_title(&said);
				let t = since.map_or(0.0, |since| since.elapsed().as_secs_f64());
				return k.set_image(&searching_key(t, &GlyphKey { title: &said, title_at: &at, ..Default::default() }));
			}
			Phase::Idle => {}
		}
		match self.0 {
			What::Sound => render_sound(k),
			What::Voice => render_voice(k),
		}
	}

	fn on_disappear(&self, k: &KeyRc) {
		let listening = {
			let mut x = k.own::<Saying>();
			if let Some(look) = x.look.take() {
				look.clear();
			}
			if let Some((spin, _)) = x.spin.take() {
				spin.clear();
			}
			x.down.take().is_some() && x.deaf.take().is_none()
		};
		if listening {
			// Sent right away, no need to wait for the answer.
			drop(k.client.call("listen", Some("off")));
		}
	}

	async fn on_press(&self, k: &KeyRc, _press: Press) -> Res<()> {
		if k.is_in_multi_action() {
			return self.play(k).await;
		}
		let now = Instant::now();
		let busy = {
			let mut x = k.own::<Saying>();
			x.down = Some(now);
			x.deaf = None;
			x.second = x.played.take().is_some_and(|at| at.elapsed().as_secs_f64() * 1000.0 < DOUBLE_PRESS_MS);
			x.phase != Phase::Idle
		};
		if busy {
			k.own::<Saying>().deaf = Some(Error::from("Still looking for the last one"));
			return Ok(());
		}
		if let Err(error) = k.call("listen", Some("on")).await {
			// A press still works.
			k.own::<Saying>().deaf = Some(newer(error));
			return Ok(());
		}
		let what = self.0;
		let look = k.after(SAY_AFTER_MS, move |k| {
			let held = {
				let x = k.own::<Saying>();
				x.down == Some(now) && x.phase == Phase::Idle
			};
			if held {
				Say(what).show(&k, Phase::Listening, "");
			}
		});
		k.own::<Saying>().look = Some(look);
		Ok(())
	}

	async fn on_release(&self, k: &KeyRc) -> Res<()> {
		let (down, deaf, second) = {
			let mut x = k.own::<Saying>();
			if let Some(look) = x.look.take() {
				look.clear();
			}
			(x.down.take(), x.deaf.take(), std::mem::take(&mut x.second))
		};
		// In a multi action the press did it already.
		let Some(down) = down else { return Ok(()) };
		if down.elapsed().as_secs_f64() * 1000.0 < SAY_AFTER_MS {
			let double = second && self.0 == What::Sound;
			// Right away, before waiting for Voicenux: a second press may come meanwhile.
			if self.0 == What::Sound && !double {
				k.own::<Saying>().played = Some(Instant::now());
			}
			if deaf.is_none() {
				let _ = k.call("listen", Some("off")).await;
			}
			if double {
				return self.next(k).await;
			}
			return self.play(k).await;
		}
		if let Some(error) = deaf {
			return Err(error);
		}
		self.show(k, Phase::Searching, "");
		let found = self.find(k).await;
		self.show(k, Phase::Idle, "");
		found.map_err(newer)?;
		k.clear_error();
		k.show_ok();
		Ok(())
	}
}

/// The sound it found, with the lens: hold the key for another one.
fn render_sound(k: &KeyRc) {
	let s: SaySoundSettings = k.settings();
	if s.sound.is_empty() {
		return k.glyph_for_title("", "below");
	}
	// A sound of the soundboard may have a new name by now.
	let (name, icon) = match k.client.sound(&s.sound) {
		Some(sound) => (sound.name, sound.icon),
		None => (s.sound_name, s.sound_icon),
	};
	let title = if s.show_name { name.as_str() } else { "" };
	k.own_title(title);
	let playing = k.client.playing().contains(&s.sound);
	let png = k.picture(&icon);
	k.set_image(&picture_key(&PictureKey {
		png: png.as_deref(),
		fallback: "sound",
		title,
		title_at: &s.name_at,
		frame: (s.show_playing && playing).then_some(Frame::On),
		badge: "saysound",
		..Default::default()
	}));
}

/// The voice it found, with a frame while it is in use, like a voice key, and the lens.
fn render_voice(k: &KeyRc) {
	let s: SayVoiceSettings = k.settings();
	if s.voice.is_empty() {
		return k.glyph_for_title("", "below");
	}
	let Some(voice) = k.client.voice(&s.voice) else {
		k.own_title("");
		return k.set_image(&glyph_key("voice", &GlyphKey { color: colors::GRAY, ..Default::default() }));
	};
	let vn = k.vn();
	let in_use = vn.voice_id() == Some(voice.id.as_str());
	let frame = (s.show_frame && in_use).then_some(if vn.voice_changer { Frame::On } else { Frame::Off });
	let title = if s.show_name { voice.name.as_str() } else { "" };
	k.own_title(title);
	let png = k.picture(&voice.icon);
	k.set_image(&picture_key(&PictureKey {
		png: png.as_deref(),
		title,
		title_at: &s.name_at,
		frame,
		badge: "saysound",
		..Default::default()
	}));
}

pub static SAY_SOUND: Kind = Kind {
	uuid: "de.felitendo.voicenux.saysound",
	name: "SaySound",
	defaults: || {
		defaults(json!({
			"sound": "",
			"soundName": "",
			"soundIcon": "",
			"soundWords": "",
			"soundRank": 0,
			"play": "restart",
			"language": "auto",
			"showName": true,
			"nameAt": "below",
			"showPlaying": true,
		}))
	},
	behavior: &Say(What::Sound),
	new_state: || Box::new(Saying::default()),
	..Kind::BASE
};

pub static SAY_VOICE: Kind = Kind {
	uuid: "de.felitendo.voicenux.sayvoice",
	name: "SayVoice",
	defaults: || {
		defaults(json!({
			"voice": "",
			"press": "toggle",
			"language": "auto",
			"showName": true,
			"nameAt": "below",
			"showFrame": true,
		}))
	},
	behavior: &Say(What::Voice),
	new_state: || Box::new(Saying::default()),
	..Kind::BASE
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::testing::*;
	use crate::settings::testing::assert_defaults;

	#[test]
	fn every_setting_has_a_default() {
		assert_defaults::<SaySoundSettings>(&[&SAY_SOUND]);
		assert_defaults::<SayVoiceSettings>(&[&SAY_VOICE]);
		assert_defaults::<Listening>(&[&SAY_SOUND, &SAY_VOICE]);
	}

	fn said(k: &KeyRc, words: &str) {
		if let Some(fake) = k.client.fake.borrow_mut().as_mut() {
			fake.said = words.into();
		}
	}

	async fn hold(k: &KeyRc, ms: f64) {
		k.key_down(false).await;
		crate::timers::sleep(ms).await;
		k.key_up().await;
	}

	#[tokio::test(start_paused = true)]
	async fn holding_a_say_key_finds_the_sound_and_a_press_plays_it() {
		local(async {
			let (_p, k) = key(&SAY_SOUND, json!({}));
			said(&k, "Vine boom");
			k.key_down(false).await;
			assert!(k.vn().listening, "Voicenux listens from the start");
			crate::timers::sleep(SAY_AFTER_MS + 100.0).await;
			assert_eq!(k.own::<Saying>().phase, Phase::Listening);
			k.key_up().await;
			assert_eq!(last(&k), ["find-sound", "Vine boom"]);
			let s: SaySoundSettings = k.settings();
			assert_eq!((s.sound.as_str(), s.sound_name.as_str(), s.sound_icon.as_str()), ("found-1", "Vine boom", "collision"));
			assert_eq!((s.sound_words.as_str(), s.sound_rank), ("Vine boom", 1.0));
			assert!(k.client.sound("found-1").is_none(), "not on the soundboard");
			assert_eq!(k.own::<Saying>().phase, Phase::Idle);
			assert!(k.own::<Saying>().spin.is_none(), "the lens rests again");
			let saved =
				k.sd.sent.as_ref().unwrap().borrow().iter().any(|m| m["event"] == "setSettings" && m["payload"]["sound"] == "found-1");
			assert!(saved, "OpenDeck keeps the sound");
			// A press stops listening and plays it.
			hold(&k, 50.0).await;
			assert_eq!(calls(&k)[calls(&k).len() - 2].0, "listen");
			assert_eq!(last(&k), ["sound-restart", "found-1"]);
			assert!(!k.vn().listening);
		})
		.await;
	}

	#[tokio::test(start_paused = true)]
	async fn a_say_key_tries_what_voicenux_heard_in_english_too() {
		local(async {
			let (_p, k) = key(&SAY_SOUND, json!({}));
			if let Some(fake) = k.client.fake.borrow_mut().as_mut() {
				fake.said = "Weinbumm".into();
				fake.also = vec!["Vine boom".into()];
				fake.unknown = vec!["Weinbumm".into()];
			}
			hold(&k, 1000.0).await;
			assert_eq!(last(&k), ["find-sound", r#"{"words":"Weinbumm","alternatives":["Vine boom"]}"#]);
			let s: SaySoundSettings = k.settings();
			assert_eq!((s.sound.as_str(), s.sound_words.as_str()), ("found-1", "Vine boom"), "the next one goes by the English");
		})
		.await;
	}

	#[tokio::test(start_paused = true)]
	async fn a_double_press_takes_the_sound_with_the_next_most_likes() {
		local(async {
			let (_p, k) = key(&SAY_SOUND, json!({ "sound": "found-1", "soundWords": "vine boom", "soundRank": 1 }));
			hold(&k, 50.0).await;
			assert_eq!(last(&k), ["sound-restart", "found-1"]);
			crate::timers::sleep(100.0).await;
			hold(&k, 50.0).await;
			let names: Vec<String> = calls(&k).iter().map(|c| c.0.clone()).collect();
			assert!(names.ends_with(&["listen".into(), "listen".into(), "stop-sound".into(), "find-sound".into()]), "{names:?}");
			assert_eq!(last(&k), ["find-sound", r#"{"words":"vine boom","rank":2}"#]);
			let s: SaySoundSettings = k.settings();
			assert_eq!((s.sound.as_str(), s.sound_rank), ("found-2", 2.0));
			// Later, a press plays again.
			crate::timers::sleep(DOUBLE_PRESS_MS + 100.0).await;
			hold(&k, 50.0).await;
			assert_eq!(last(&k), ["sound-restart", "found-2"]);
		})
		.await;
	}

	#[tokio::test(start_paused = true)]
	async fn a_say_key_says_why_it_found_nothing() {
		local(async {
			let (_p, k) = key(&SAY_SOUND, json!({ "language": "de" }));
			hold(&k, 50.0).await;
			assert_eq!(k.last_error().as_deref(), Some("Hold the key and say a sound"));
			hold(&k, 1000.0).await;
			assert_eq!(last(&k), ["transcribe", "de"]);
			assert_eq!(k.last_error().as_deref(), Some("Nothing heard. Hold the key and say a sound."));
			assert_eq!(k.settings::<SaySoundSettings>().sound, "");
			// An older Voicenux cannot listen, but it still plays.
			let (_p, old) = key(&SAY_SOUND, json!({ "sound": "airhorn" }));
			old_voicenux(&old);
			hold(&old, 50.0).await;
			assert_eq!(last(&old), ["sound", "airhorn"]);
			hold(&old, 1000.0).await;
			assert_eq!(old.last_error().as_deref(), Some("Saying needs a newer Voicenux"));
		})
		.await;
	}

	#[tokio::test(start_paused = true)]
	async fn a_say_voice_key_uses_one_of_your_voices_and_a_press_turns_it_off() {
		local(async {
			let (_p, k) = key(&SAY_VOICE, json!({}));
			said(&k, "Deep.");
			hold(&k, 1000.0).await;
			assert_eq!(last(&k), ["voice", "deep"]);
			let heard = calls(&k).into_iter().find(|c| c.0 == "transcribe").and_then(|c| c.1);
			assert_eq!(heard.as_deref(), Some(r#"{"language":"auto","expect":"voices"}"#), "it listens for voices");
			assert_eq!(k.settings::<SayVoiceSettings>().voice, "deep");
			assert_eq!(k.vn().voice_id(), Some("deep"));
			// In use: a press turns the voice changer off, the next one uses the voice again.
			hold(&k, 50.0).await;
			assert_eq!(last(&k), ["voice-changer", "off"]);
			hold(&k, 50.0).await;
			assert_eq!(last(&k), ["voice", "deep"]);
			// Voices are not searched online.
			said(&k, "Gandalf");
			hold(&k, 1000.0).await;
			assert_eq!(k.last_error().as_deref(), Some(r#"No voice called "gandalf" in Voicenux. Voices are not searched online."#));
			assert_eq!(k.settings::<SayVoiceSettings>().voice, "deep");
		})
		.await;
	}
}
