// Glyphs drawn on a 24x24 grid. Bold, like the other FeloDeck plugins: a body is always filled
// (details are cut out of it), round 2.4px strokes only for lines like arrows and waves.
// Used for the static icons (`cargo xtask icons`), the settings pages and keys drawn at runtime.
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::glyph_fit::FIT;
use crate::js::{fmt, round};

pub mod colors {
	pub const WHITE: &str = "#FFFFFF";
	pub const GRAY: &str = "#6B6B6B";
	pub const MUTED: &str = "#9A9AA2";
	pub const RED: &str = "#E5484D";
	pub const GREEN: &str = "#30A46C";
	pub const AMBER: &str = "#F5A524";
	// The gradient of Voicenux, deep blue to cyan.
	pub const BLUE: &str = "#2563EB";
	pub const SKY: &str = "#3B82F6";
	pub const CYAN: &str = "#22D3EE";
}

pub static COLORS: &[(&str, &str)] = &[
	("white", colors::WHITE),
	("gray", colors::GRAY),
	("muted", colors::MUTED),
	("red", colors::RED),
	("green", colors::GREEN),
	("amber", colors::AMBER),
	("blue", colors::BLUE),
	("sky", colors::SKY),
	("cyan", colors::CYAN),
];

const FILL: &str = r#"fill="currentColor""#;

// A filled body with details cut out of it. Cuts are lines in the stroke width, or shapes with
// fill="#000"; add puts parts back on top of a cut.
fn cut(id: &str, body: &str, cuts: &str, add: &str) -> String {
	format!(
		r##"<mask id="cut-{id}"><g fill="#FFF" stroke="none">{body}</g><g fill="none" stroke="#000">{cuts}</g><g fill="none" stroke="#FFF">{add}</g></mask><rect x="-4" y="-4" width="32" height="32" {FILL} stroke="none" mask="url(#cut-{id})"/>"##
	)
}

fn bar(x: f64, y: f64, h: f64) -> String {
	format!(r#"<rect x="{}" y="{}" width="3" height="{}" rx="1.5" {FILL} stroke="none"/>"#, fmt(x), fmt(y), fmt(h))
}

fn pip(x: f64, y: f64) -> String {
	format!(r##"<circle cx="{}" cy="{}" r="1.75" fill="#000" stroke="none"/>"##, fmt(x), fmt(y))
}

fn build() -> Vec<(&'static str, String)> {
	let fill = FILL;
	let speaker = format!(r#"<path d="M3.5 9.2h3.2L11.5 5v14l-4.8-4.2H3.5a1 1 0 0 1-1-1v-3.6a1 1 0 0 1 1-1z" {fill} stroke-width="1.6"/>"#);
	let capsule = r#"<rect x="8" y="1.8" width="8" height="13" rx="4"/>"#;
	let cups = format!(
		r#"<path d="M3 13.5h3.2a1.3 1.3 0 0 1 1.3 1.3v5.4a1.3 1.3 0 0 1-1.3 1.3H4.3A1.3 1.3 0 0 1 3 20.2zM21 13.5h-3.2a1.3 1.3 0 0 0-1.3 1.3v5.4a1.3 1.3 0 0 0 1.3 1.3h1.9a1.3 1.3 0 0 0 1.3-1.3z" {fill} stroke-width="1.4"/>"#
	);
	let band = r#"<path d="M4.3 15.5v-3a7.7 7.7 0 0 1 15.4 0v3"/>"#;
	let stand = r#"<path d="M4.6 11.2a7.4 7.4 0 0 0 14.8 0M12 18.6v3.2"/>"#;
	vec![
		// Voicenux itself: the five bars of its icon.
		("plugin", bar(2.0, 6.0, 12.0) + &bar(6.75, 9.5, 8.5) + &bar(10.5, 13.0, 5.0) + &bar(14.25, 9.5, 8.5) + &bar(19.0, 6.0, 12.0)),
		(
			"voicechanger",
			bar(2.0, 3.5, 17.0) + &bar(6.75, 7.5, 9.0) + &bar(10.5, 10.0, 4.0) + &bar(14.25, 7.5, 9.0) + &bar(19.0, 3.5, 17.0),
		),
		// A mask: a voice that is not yours.
		(
			"voice",
			cut(
				"voice",
				r##"<path d="M3 4.2c2.6-1.6 15.4-1.6 18 0v6.6c0 6.2-4.3 10.6-9 10.6S3 17 3 10.8z" stroke="#FFF" stroke-width="1.2" stroke-linejoin="round"/>"##,
				r#"<path d="M6.6 9.6c.8-.9 2.4-.9 3.2 0M14.2 9.6c.8-.9 2.4-.9 3.2 0M8 14.6c2 2.3 6 2.3 8 0"/>"#,
				"",
			),
		),
		// Two beamed notes.
		(
			"sound",
			format!(
				r#"<path d="M8.6 17.6V6.2l11-2.4v11.4" stroke-width="2.6"/><path d="M8.6 6.2l11-2.4v3.6l-11 2.4z" {fill} stroke-width="1.4"/><ellipse cx="6" cy="17.8" rx="3.4" ry="2.8" {fill} stroke="none"/><ellipse cx="17" cy="15.4" rx="3.4" ry="2.8" {fill} stroke="none"/>"#
			),
		),
		("mic", capsule.replace("/>", &format!(r#" {fill} stroke="none"/>"#)) + stand),
		// A lens over a face: find one of your voices.
		(
			"sayvoice",
			format!(
				r#"<circle cx="10" cy="10" r="7.6"/><path d="m15.5 15.5 5.4 5.4" stroke-width="3.4"/><circle cx="7.6" cy="8.4" r="1.3" {fill} stroke="none"/><circle cx="12.4" cy="8.4" r="1.3" {fill} stroke="none"/><path d="M7.1 11.6c1.6 1.8 4.2 1.8 5.8 0" stroke-width="2"/>"#
			),
		),
		// A lens over a sound wave: find a sound.
		(
			"saysound",
			r#"<circle cx="10" cy="10" r="7.6"/><path d="m15.5 15.5 5.4 5.4" stroke-width="3.4"/><path d="M6.8 8.6v2.8M10 6.4v7.2M13.2 8.6v2.8" stroke-width="2.2"/>"#.to_string(),
		),
		(
			"micOff",
			cut(
				"micOff",
				&format!(r##"{capsule}<path d="M4.6 11.2a7.4 7.4 0 0 0 14.8 0M12 18.6v3.2" fill="none" stroke="#FFF"/>"##),
				r#"<path d="m3 3 18 18" stroke-width="6"/>"#,
				r#"<path d="m3 3 18 18"/>"#,
			),
		),
		("headphones", format!("{band}{cups}")),
		(
			"hearsounds",
			format!(r#"{band}{cups}<path d="M12.6 19.4v-8.2l2.6.8" stroke-width="2"/><circle cx="11" cy="19.6" r="2.1" {fill} stroke="none"/>"#),
		),
		(
			"background",
			r#"<path d="M2.5 7.5c1.6-1.2 3.1-1.2 4.7 0s3.1 1.2 4.7 0 3.1-1.2 4.7 0 3.1 1.2 4.7 0M2.5 12.5c1.6-1.2 3.1-1.2 4.7 0s3.1 1.2 4.7 0 3.1-1.2 4.7 0 3.1 1.2 4.7 0M2.5 17.5c1.6-1.2 3.1-1.2 4.7 0s3.1 1.2 4.7 0 3.1-1.2 4.7 0 3.1 1.2 4.7 0"/>"#
				.to_string(),
		),
		(
			"random",
			cut(
				"random",
				r#"<rect x="2.5" y="2.5" width="19" height="19" rx="4.4"/>"#,
				&(pip(7.4, 7.4) + &pip(16.6, 7.4) + &pip(12.0, 12.0) + &pip(7.4, 16.6) + &pip(16.6, 16.6)),
				"",
			),
		),
		(
			"next",
			format!(r#"<path d="M5 6l9.5 6L5 18z" {fill}/><rect x="16.3" y="5" width="3.2" height="14" rx="1.2" {fill} stroke="none"/>"#),
		),
		(
			"previous",
			format!(r#"<path d="M19 6 9.5 12l9.5 6z" {fill}/><rect x="4.5" y="5" width="3.2" height="14" rx="1.2" {fill} stroke="none"/>"#),
		),
		("stop", format!(r#"<rect x="5.5" y="5.5" width="13" height="13" rx="2.2" {fill} stroke="none"/>"#)),
		("volume", format!(r#"{speaker}<path d="M15.2 8.8a4.6 4.6 0 0 1 0 6.4M18.3 5.8a9 9 0 0 1 0 12.4"/>"#)),
		("volumeUp", format!(r#"{speaker}<path d="M15.5 12h6M18.5 9v6"/>"#)),
		("volumeDown", format!(r#"{speaker}<path d="M15.5 12h6"/>"#)),
		("volumeSet", format!(r#"{speaker}<path d="M15.5 10h6M15.5 14h6"/>"#)),
		("power", r#"<path d="M12 3v9"/><path d="M18.4 6.6a9 9 0 1 1-12.8 0"/>"#.to_string()),
		// Like the icon of "Other apps" in the settings of Voicenux.
		(
			"plug",
			format!(r#"<path d="M9 2.5v4.5M15 2.5v4.5"/><path d="M5.5 7h13v3.6a6.5 6.5 0 0 1-13 0z" {fill} stroke-width="1.6"/><path d="M12 17.1v4.4"/>"#),
		),
		(
			"star",
			format!(r#"<path d="m12 2.8 2.8 5.8 6.3.9-4.6 4.4 1.1 6.3-5.6-3-5.6 3 1.1-6.3-4.6-4.4 6.3-.9z" {fill} stroke-width="1.6"/>"#),
		),
	]
}

/// Every glyph by name, in the order they are written above.
pub static GLYPHS: LazyLock<Vec<(&'static str, String)>> = LazyLock::new(build);

static INDEX: LazyLock<HashMap<&'static str, usize>> =
	LazyLock::new(|| GLYPHS.iter().enumerate().map(|(i, (name, _))| (*name, i)).collect());

pub fn glyph_body(name: &str) -> Option<&'static str> {
	INDEX.get(name).map(|&i| GLYPHS[i].1.as_str())
}

pub fn escape_xml(text: &str) -> String {
	let mut out = String::with_capacity(text.len());
	for c in text.chars() {
		match c {
			'<' => out.push_str("&lt;"),
			'>' => out.push_str("&gt;"),
			'&' => out.push_str("&amp;"),
			'"' => out.push_str("&quot;"),
			'\'' => out.push_str("&apos;"),
			_ => out.push(c),
		}
	}
	out
}

/// The glyph of a soundboard volume key for each mode.
pub fn volume_glyph(mode: &str) -> Option<&'static str> {
	VOLUME_GLYPHS.iter().find(|(m, _)| *m == mode).map(|(_, g)| *g)
}

pub static VOLUME_GLYPHS: &[(&str, &str)] = &[("up", "volumeUp"), ("down", "volumeDown"), ("set", "volumeSet")];

/// Center and size of a glyph as measured, in grid units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
	pub cx: f64,
	pub cy: f64,
	pub w: f64,
	pub h: f64,
}

pub fn fit_of(name: &str) -> Option<Fit> {
	FIT.iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
}

// How much of the 24 grid a glyph may use: wide ones up to MAX, compact ones less,
// so a circle does not look bigger than a wide eye.
const MAX: f64 = 20.5;
const AREA: f64 = 17.5;

fn fit_scale(b: &Fit) -> f64 {
	crate::js::min(MAX / crate::js::max(b.w, b.h), AREA / (b.w * b.h).sqrt())
}

/// How tall a glyph is drawn, in units of the 24 grid.
pub fn glyph_height(name: &str) -> f64 {
	fit_of(name).map_or(24.0, |b| b.h * fit_scale(&b))
}

fn n3(v: f64) -> f64 {
	round(v * 1000.0) / 1000.0
}

// The fitted box of a glyph: centered on (12, 12) and with an even optical size.
fn fit_transform(name: &str) -> (String, f64) {
	let Some(b) = fit_of(name) else { return (String::new(), 1.0) };
	let k = fit_scale(&b);
	(format!("translate(12 12) scale({}) translate({} {})", fmt(n3(k)), fmt(n3(-b.cx)), fmt(n3(-b.cy))), k)
}

pub struct GlyphOpts<'a> {
	pub x: f64,
	pub y: f64,
	pub size: f64,
	pub color: &'a str,
	pub stroke: f64,
	/// false draws it as written, used to measure it.
	pub fit: bool,
}

impl Default for GlyphOpts<'_> {
	fn default() -> Self {
		GlyphOpts { x: 0.0, y: 0.0, size: 24.0, color: colors::WHITE, stroke: 2.4, fit: true }
	}
}

/// A glyph placed at (x, y) with the given size in pixels.
pub fn glyph(name: &str, o: &GlyphOpts) -> String {
	let body = glyph_body(name).or_else(|| glyph_body("plugin")).unwrap_or("");
	let (inner, scale) = if o.fit { fit_transform(name) } else { (String::new(), 1.0) };
	// Keep the stroke the same on screen, whatever the glyph was scaled by.
	let width = n3(o.stroke / scale);
	let color = o.color;
	let outer = format!(
		r#"<g transform="translate({} {}) scale({})" fill="none" stroke="{color}" color="{color}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round">"#,
		fmt(n3(o.x)),
		fmt(n3(o.y)),
		fmt(n3(o.size / 24.0)),
		fmt(width)
	);
	if inner.is_empty() { format!("{outer}{body}</g>") } else { format!(r#"{outer}<g transform="{inner}">{body}</g></g>"#) }
}

/// Icon for the action list: white glyph, no background.
pub fn action_icon_svg(name: &str, view_box: &str) -> String {
	format!(
		r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="{view_box}">{}</svg>"#,
		glyph(name, &GlyphOpts::default())
	) + "\n"
}
