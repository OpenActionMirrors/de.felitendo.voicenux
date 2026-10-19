// The settings pages are JavaScript. These tests run them with boa (a JavaScript engine in Rust):
// every text needs a German version, and the pictures of keys there must look like the real keys.
use std::path::{Path, PathBuf};
use std::rc::Rc;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::module::SimpleModuleLoader;
use boa_engine::{Context, Module, Source, js_string};
use regex::Regex;
use serde_json::{Value, json};
use voicenux_plugin::render::{Frame, GlyphKey, PictureKey};
use voicenux_plugin::{i18n, icons, manifest, pi, render};

fn root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn pi_dir() -> PathBuf {
	root().join("de.felitendo.voicenux.sdPlugin").join("pi")
}

/// Runs a module next to the settings page and returns its export `result` as JSON.
fn run_js(code: &str) -> Value {
	let dir = pi_dir();
	let loader = Rc::new(SimpleModuleLoader::new(&dir).expect("pi folder"));
	let mut context = Context::builder().module_loader(loader.clone()).build().expect("context");
	let path = dir.join("__test.js");
	let module = Module::parse(Source::from_bytes(code).with_path(&path), None, &mut context).expect("test module parses");
	loader.insert(path, module.clone());
	let promise = module.load_link_evaluate(&mut context);
	let _ = context.run_jobs();
	match promise.state() {
		PromiseState::Fulfilled(_) => {}
		PromiseState::Rejected(error) => panic!("JavaScript failed: {}", error.display()),
		PromiseState::Pending => panic!("JavaScript did not finish"),
	}
	let namespace = module.namespace(&mut context);
	let result =
		namespace.get(js_string!("result"), &mut context).expect("result").to_string(&mut context).expect("text").to_std_string_escaped();
	serde_json::from_str(&result).expect("result is JSON")
}

#[test]
fn shared_js_is_up_to_date() {
	let file = std::fs::read_to_string(root().join(pi::SHARED_JS)).unwrap_or_default();
	assert!(file == pi::shared_js(), "pi/lib/shared.js is old. Run: make manifest");
}

#[test]
fn every_action_has_a_settings_page_and_known_choices() {
	let schemas = run_js(
		r#"
		import { SCHEMAS } from "./schemas.js";
		const out = {};
		for (const [uuid, schema] of Object.entries(SCHEMAS)) {
			out[uuid] = (schema.fields ?? []).filter((f) => f.key && f.options).map((f) => [f.key, f.options.map(([v]) => v)]);
		}
		export const result = JSON.stringify(out);
		"#,
	);
	let kinds = voicenux_plugin::actions::all();
	let mut uuids: Vec<String> = kinds.iter().map(|k| k.uuid.to_string()).collect();
	let mut listed: Vec<String> = manifest::ACTIONS.iter().map(|a| format!("{}.{}", manifest::ID, a.id)).collect();
	uuids.sort();
	listed.sort();
	assert_eq!(uuids, listed, "every action needs a manifest entry");
	for kind in kinds {
		let fields = schemas[kind.uuid].as_array().unwrap_or_else(|| panic!("no settings page for {}", kind.uuid));
		// Press, hold and double press offer only choices the key knows.
		for field in fields {
			let key = field[0].as_str().unwrap_or("");
			if !["hold", "double", kind.press_key].contains(&key) || kind.ops.is_empty() {
				continue;
			}
			for value in field[1].as_array().into_iter().flatten().filter_map(Value::as_str) {
				let known = kind.ops.contains(&value) || value == "none" || (value == "repeat" && kind.can_repeat);
				assert!(known, "{}: {key} offers {value}", kind.name);
			}
		}
	}
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
	for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
		let path = entry.path();
		if path.is_dir() {
			rust_files(&path, out);
		} else if path.extension().is_some_and(|e| e == "rs") {
			out.push(path);
		}
	}
}

// The code of a file without its test modules (a #[cfg(test)] field does not end the code).
fn without_tests(code: &str) -> &str {
	let module = code.match_indices("#[cfg(test)]\n").map(|(i, _)| i).find(|&i| {
		let next = &code[i + "#[cfg(test)]\n".len()..];
		next.starts_with("mod ") || next.starts_with("pub(crate) mod ")
	});
	match module {
		Some(i) => &code[..i],
		None => code,
	}
}

// The part between the parentheses that start at `open`.
fn span(code: &str, open: usize) -> &str {
	let mut depth = 0;
	let mut in_string = false;
	let bytes = code.as_bytes();
	let mut i = open;
	while i < bytes.len() {
		match bytes[i] {
			b'\\' if in_string => i += 1,
			b'"' => in_string = !in_string,
			b'(' if !in_string => depth += 1,
			b')' if !in_string => {
				depth -= 1;
				if depth == 0 {
					return &code[open..=i];
				}
			}
			_ => {}
		}
		i += 1;
	}
	&code[open..]
}

fn unescape(literal: &str) -> String {
	serde_json::from_str::<String>(&format!("\"{literal}\"")).unwrap_or_else(|_| literal.to_string())
}

// Every text a user can see needs a German translation, with the same {placeholders}.
#[test]
fn every_text_has_a_german_translation() {
	let mut texts: Vec<String> = Vec::new();
	// The settings pages: labels, hints, options and sections of every field.
	let schema_texts = run_js(
		r#"
		import { SCHEMAS } from "./schemas.js";
		const texts = new Set();
		const add = (value) => typeof value === "string" && value && texts.add(value);
		for (const schema of Object.values(SCHEMAS)) {
			add(schema.help);
			add(schema.lookTitle);
			for (const field of schema.fields ?? []) {
				for (const key of ["label", "text", "hint", "unit"]) add(field[key]);
				// Section titles. "look" is a name in the code, the page shows "On the key" or lookTitle.
				if (field.section !== "look") add(field.section);
				for (const [, label] of field.options ?? []) add(label);
			}
		}
		export const result = JSON.stringify([...texts]);
		"#,
	);
	texts.extend(schema_texts.as_array().expect("texts").iter().filter_map(|t| t.as_str().map(String::from)));
	// t("...") in the settings pages.
	let t_call = Regex::new(r#"\bt\("((?:[^"\\]|\\.)+)""#).expect("regex");
	for file in ["pi.js", "schemas.js"] {
		let page = std::fs::read_to_string(pi_dir().join(file)).expect("settings page");
		texts.extend(t_call.captures_iter(&page).map(|c| unescape(&c[1])));
	}
	let mut files = Vec::new();
	rust_files(&root().join("src"), &mut files);
	let literal = Regex::new(r#""((?:[^"\\]|\\.)+)""#).expect("regex");
	let placeholder = Regex::new(r"\{[^{}]*\}").expect("regex");
	let call = Regex::new(r"\bt\(").expect("regex");
	let mut errors = Vec::new();
	for file in files.iter().filter(|f| !f.ends_with("de.rs")) {
		let code = std::fs::read_to_string(file).expect("source");
		let code = without_tests(&code);
		// t(...) in the plugin, also t(if on { "On" } else { "Off" }). Texts start with a capital,
		// the words they are picked by do not.
		for found in call.find_iter(code) {
			for capture in literal.captures_iter(span(code, found.end() - 1)) {
				let text = unescape(&capture[1]);
				if text.starts_with(|c: char| c.is_uppercase()) {
					texts.push(text);
				}
			}
		}
		// Errors and statuses reach the settings page, which translates them. Messages are the texts
		// with a space, any {value} can be any value.
		for start in ["Err(", "Error::from(", "ok_or(", "set_status("] {
			for (at, _) in code.match_indices(start) {
				let open = at + start.len() - 1;
				for capture in literal.captures_iter(span(code, open)) {
					let text = placeholder.replace_all(&unescape(&capture[1]), "{x}").to_string();
					if text.contains(' ') {
						errors.push(text);
					}
				}
			}
		}
	}
	let de: std::collections::HashMap<&str, &str> = i18n::DE.iter().copied().collect();
	let mut missing: Vec<&String> = texts.iter().filter(|t| !de.contains_key(t.as_str()) && t.as_str() != "%").collect();
	missing.sort();
	missing.dedup();
	assert!(missing.is_empty(), "missing German texts: {missing:#?}");
	let holes = Regex::new(r"\{\w+\}").expect("regex");
	let known: Vec<String> = i18n::DE.iter().map(|(en, _)| holes.replace_all(en, "{x}").to_string()).collect();
	let mut unknown: Vec<&String> = errors.iter().filter(|e| !known.contains(e)).collect();
	unknown.sort();
	unknown.dedup();
	assert!(unknown.is_empty(), "errors without a German text: {unknown:#?}");
}

// The settings pages draw small pictures of keys with pi/lib/render.js. They must look like the keys.
#[test]
fn settings_page_pictures_match_the_keys() {
	let mut glyph_cases = Vec::new();
	for (name, _) in icons::GLYPHS.iter() {
		for options in [
			json!({}),
			json!({ "color": "#6B6B6B", "setup": true }),
			json!({ "title": "Robot" }),
			json!({ "title": "80%", "titleAt": "above" }),
			json!({ "title": "Bildschirmaufnahme (PipeWire)" }),
		] {
			glyph_cases.push(json!([name, options]));
		}
	}
	let picture_cases = json!([
		{ "png": "AAAA" },
		{ "png": "AAAA", "title": "Robot", "frame": "on" },
		{ "png": "AAAA", "title": "Deep", "titleAt": "above", "frame": "off" },
		{ "png": "AAAA", "title": "Robot", "dim": true },
		{ "png": "AAAA", "title": "A very very long voice name", "frame": "on" },
		{ "fallback": "sound", "title": "Bruh", "frame": "on" },
		{ "fallback": "voice", "setup": true, "dim": true },
	]);
	let result = run_js(&format!(
		r#"
		import {{ glyphKeySvg, pictureKeySvg }} from "./lib/render.js";
		const glyphs = {glyph_cases}.map(([name, options]) => glyphKeySvg(name, options));
		const pictures = {picture_cases}.map((options) => pictureKeySvg(options));
		export const result = JSON.stringify({{ glyphs, pictures }});
		"#,
		glyph_cases = Value::Array(glyph_cases.clone()),
	));
	let get = |o: &Value, k: &str| o.get(k).and_then(Value::as_str).map(String::from);
	let mut differences = Vec::new();
	for (case, js_svg) in glyph_cases.iter().zip(result["glyphs"].as_array().expect("glyphs")) {
		let o = &case[1];
		let (color, title, title_at) = (get(o, "color"), get(o, "title"), get(o, "titleAt"));
		let options = GlyphKey {
			color: color.as_deref().unwrap_or(icons::colors::WHITE),
			setup: o.get("setup").and_then(Value::as_bool).unwrap_or(false),
			title: title.as_deref().unwrap_or(""),
			title_at: title_at.as_deref().unwrap_or("below"),
		};
		if render::glyph_key_svg(case[0].as_str().unwrap_or(""), &options) != js_svg.as_str().unwrap_or("") {
			differences.push(case.to_string());
		}
	}
	for (o, js_svg) in picture_cases.as_array().expect("cases").iter().zip(result["pictures"].as_array().expect("pictures")) {
		let (png, fallback, title, title_at) = (get(o, "png"), get(o, "fallback"), get(o, "title"), get(o, "titleAt"));
		let options = PictureKey {
			png: png.as_deref(),
			fallback: fallback.as_deref().unwrap_or("voice"),
			title: title.as_deref().unwrap_or(""),
			title_at: title_at.as_deref().unwrap_or("below"),
			frame: match get(o, "frame").as_deref() {
				Some("on") => Some(Frame::On),
				Some("off") => Some(Frame::Off),
				_ => None,
			},
			dim: o.get("dim").and_then(Value::as_bool).unwrap_or(false),
			setup: o.get("setup").and_then(Value::as_bool).unwrap_or(false),
		};
		if render::picture_key_svg(&options) != js_svg.as_str().unwrap_or("") {
			differences.push(o.to_string());
		}
	}
	assert!(differences.is_empty(), "pictures differ from the keys: {differences:#?}");
}

// The settings pages translate the messages of the plugin like the plugin does.
#[test]
fn settings_page_texts_match_the_plugin() {
	let messages = [
		"Something else uses port 41967",
		"no voice called \"robo\"",
		"Voicenux not connected (connection closed)",
		"Voicenux speaks API 2, this plugin 1. Update the plugin.",
		"something else",
		"Voicenux is not running",
	];
	let result = run_js(&format!(
		r#"
		import {{ setLanguage, t, translateMessage }} from "./lib/i18n.js";
		const out = [];
		for (const lang of ["en", "de"]) {{
			setLanguage(lang);
			out.push({messages}.map((m) => translateMessage(m)));
			out.push([t("Connected to Voicenux {{version}}", {{ version: "0.1.0" }}), t("Voicenux off")]);
		}}
		export const result = JSON.stringify(out);
		"#,
		messages = serde_json::to_string(&messages).unwrap_or_default(),
	));
	let mut expected = Vec::new();
	for lang in ["en", "de"] {
		i18n::set_language(lang);
		expected.push(json!(messages.iter().map(|m| i18n::translate_message(m)).collect::<Vec<_>>()));
		expected.push(json!([i18n::t_with("Connected to Voicenux {version}", &[("version", "0.1.0")]), i18n::t("Voicenux off")]));
	}
	i18n::set_language("en");
	assert_eq!(result, Value::Array(expected));
}
