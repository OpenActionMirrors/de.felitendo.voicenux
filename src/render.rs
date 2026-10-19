// Key images as SVG data URLs (OpenDeck draws them on a 144x144 canvas).
// Same look as the other FeloDeck plugins: no background, a bold glyph in the middle. Voices and
// sounds show their picture from Voicenux instead.
// pi/lib/render.js draws the same pictures for the settings pages, keep the two alike.
use base64::Engine;

use crate::i18n::t;
use crate::icons::{GlyphOpts, colors, escape_xml, glyph, glyph_height};
use crate::js::{fmt, max, min, round};

pub const SIZE: f64 = 144.0;
const FONT: &str = "'Inter','Noto Sans','DejaVu Sans','Liberation Sans',sans-serif";
// Like the titles OpenDeck draws (font size 11 is about 22 px on a key), but bold to read it on
// the deck. The same in the other FeloDeck plugins.
const TITLE_FONT: &str = "'Liberation Sans',Arial,sans-serif";
// The glyph takes 24 of 40 units, like viewBox="-8 -8 40 40".
const GLYPH_SIZE: f64 = (SIZE * 24.0) / 40.0;
const GLYPH_OFFSET: f64 = (SIZE - GLYPH_SIZE) / 2.0;
// Pictures of voices and sounds are drawn with some air of their own, so they get more room.
const PICTURE_SIZE: f64 = 100.0;
const PICTURE_WITH_TITLE: f64 = 82.0;
// Space between a glyph (or picture) and its title.
const TITLE_GAP: f64 = 16.0;

pub fn svg_url(svg: &str) -> String {
	format!("data:image/svg+xml;base64,{}", base64::engine::general_purpose::STANDARD.encode(svg))
}

fn svg(content: &str) -> String {
	let s = fmt(SIZE);
	format!(
		r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{s}" height="{s}" viewBox="0 0 {s} {s}">{content}</svg>"#
	)
}

fn text(value: &str, y: f64, size: f64, color: &str) -> String {
	format!(
		r#"<text x="{}" y="{}" font-family="{FONT}" font-size="{}" font-weight="700" fill="{color}" text-anchor="middle">{}</text>"#,
		fmt(SIZE / 2.0),
		fmt(y),
		fmt(size),
		escape_xml(value)
	)
}

/// Length as JavaScript counts it, in UTF-16 units.
fn js_len(text: &str) -> usize {
	text.encode_utf16().count()
}

/// Font size that keeps a line inside the key. Capital letters are wider.
pub fn fit_size(value: &str, max_size: f64, width: f64) -> f64 {
	let length = max(1.0, js_len(value) as f64);
	let per_char = if value == value.to_uppercase() && value.chars().any(char::is_alphabetic) { 0.68 } else { 0.58 };
	max(12.0, min(max_size, (width / (length * per_char)).floor()))
}

/// A title the key draws itself, in the font of an OpenDeck title.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleLine {
	pub line: String,
	pub size: f64,
	/// How tall digits and capitals are, for placing the line.
	pub ink: f64,
}

/// 22 px like OpenDeck draws them, smaller for long text, and cut with "…" when even the smallest
/// size is too wide.
pub fn title_line(value: &str, width: f64) -> TitleLine {
	let mut line = crate::js::trim(value).to_string();
	let per_char = 0.56;
	let size = max(15.0, min(22.0, (width / (max(1.0, js_len(&line) as f64) * per_char)).floor()));
	let fits = (width / (size * per_char)).floor();
	if js_len(&line) as f64 > fits {
		// Cut like String.slice: by UTF-16 units. Half an emoji becomes U+FFFD, as in the JS version.
		let units: Vec<u16> = line.encode_utf16().take(max(1.0, fits - 1.0) as usize).collect();
		let start = String::from_utf16_lossy(&units);
		line = format!("{}…", start.trim_end_matches(crate::js::is_js_space));
	}
	TitleLine { line, size, ink: round(size * 0.72) }
}

fn title_text(line: &TitleLine, y: f64, color: &str) -> String {
	format!(
		r#"<text x="{}" y="{}" font-family="{TITLE_FONT}" font-size="{}" font-weight="700" fill="{color}" text-anchor="middle">{}</text>"#,
		fmt(SIZE / 2.0),
		fmt(round(y * 10.0) / 10.0),
		fmt(line.size),
		escape_xml(&line.line)
	)
}

// Something of the given height with a title above or below it, together in the middle of the key.
// Returns where it starts and where the title stands.
fn with_title_layout(height: f64, line: &TitleLine, above: bool) -> (f64, f64) {
	let top = (SIZE - height - TITLE_GAP - line.ink) / 2.0;
	if above { (top + line.ink + TITLE_GAP, top + line.ink) } else { (top, SIZE - top) }
}

fn setup_dot() -> String {
	format!(r#"<circle cx="120" cy="24" r="10" fill="{}"/>"#, colors::AMBER)
}

/// What a glyph key shows besides the glyph.
#[derive(Clone)]
pub struct GlyphKey<'a> {
	pub color: &'a str,
	/// A yellow dot in the corner, for a key that still needs its settings.
	pub setup: bool,
	/// A line of text "above" or "below" the glyph (title_at), close to it, and glyph and title
	/// together in the middle of the key. OpenDeck cannot move its own title, so keys draw it here.
	pub title: &'a str,
	pub title_at: &'a str,
}

impl Default for GlyphKey<'_> {
	fn default() -> Self {
		GlyphKey { color: colors::WHITE, setup: false, title: "", title_at: "below" }
	}
}

pub fn glyph_key_svg(name: &str, o: &GlyphKey) -> String {
	let mut y = GLYPH_OFFSET;
	let mut label = String::new();
	if !o.title.is_empty() {
		let line = title_line(o.title, 128.0);
		let height = (glyph_height(name) * GLYPH_SIZE) / 24.0;
		let (top, baseline) = with_title_layout(height, &line, o.title_at == "above");
		y = GLYPH_OFFSET + top - (SIZE - height) / 2.0;
		label = title_text(&line, baseline, colors::WHITE);
	}
	let dot = if o.setup { setup_dot() } else { String::new() };
	svg(&(glyph(name, &GlyphOpts { x: GLYPH_OFFSET, y, size: GLYPH_SIZE, color: o.color, ..Default::default() }) + &label + &dot))
}

pub fn glyph_key(name: &str, o: &GlyphKey) -> String {
	svg_url(&glyph_key_svg(name, o))
}

/// The frame around a voice in use or a sound that plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Frame {
	/// The gradient of Voicenux.
	On,
	/// Gray: the voice is picked, but the voice changer is off.
	Off,
}

fn frame(kind: Option<Frame>) -> String {
	let Some(kind) = kind else { return String::new() };
	let (stroke, defs) = match kind {
		Frame::Off => (colors::GRAY.to_string(), String::new()),
		Frame::On => (
			"url(#vn)".to_string(),
			format!(
				r#"<defs><linearGradient id="vn" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{}"/><stop offset="0.5" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient></defs>"#,
				colors::BLUE,
				colors::SKY,
				colors::CYAN
			),
		),
	};
	let side = fmt(SIZE - 8.0);
	format!(r#"{defs}<rect x="4" y="4" width="{side}" height="{side}" rx="16" fill="none" stroke="{stroke}" stroke-width="8"/>"#)
}

/// The picture of a voice or sound (base64 PNG from Voicenux), with a title and a frame.
#[derive(Clone)]
pub struct PictureKey<'a> {
	/// Without a picture (Voicenux sent none), the glyph `fallback` stands in for it.
	pub png: Option<&'a str>,
	pub fallback: &'a str,
	pub title: &'a str,
	pub title_at: &'a str,
	pub frame: Option<Frame>,
	/// Drawn faded, like a gray glyph.
	pub dim: bool,
	pub setup: bool,
}

impl Default for PictureKey<'_> {
	fn default() -> Self {
		PictureKey { png: None, fallback: "voice", title: "", title_at: "below", frame: None, dim: false, setup: false }
	}
}

pub fn picture_key_svg(o: &PictureKey) -> String {
	let Some(png) = o.png.filter(|p| !p.is_empty()) else {
		let color = if o.dim { colors::GRAY } else { colors::WHITE };
		let key = glyph_key_svg(o.fallback, &GlyphKey { color, setup: o.setup, title: o.title, title_at: o.title_at });
		return key.replacen("</svg>", &format!("{}</svg>", frame(o.frame)), 1);
	};
	let size = if o.title.is_empty() { PICTURE_SIZE } else { PICTURE_WITH_TITLE };
	let mut top = (SIZE - size) / 2.0;
	let mut label = String::new();
	if !o.title.is_empty() {
		let line = title_line(o.title, if o.frame.is_some() { 112.0 } else { 128.0 });
		let (start, baseline) = with_title_layout(size, &line, o.title_at == "above");
		top = start;
		label = title_text(&line, baseline, if o.dim { colors::MUTED } else { colors::WHITE });
	}
	let href = format!("data:image/png;base64,{png}");
	let fade = if o.dim { r#" opacity="0.4""# } else { "" };
	let picture = format!(
		r#"<image x="{}" y="{}" width="{size}" height="{size}"{fade} href="{href}" xlink:href="{href}"/>"#,
		fmt((SIZE - size) / 2.0),
		fmt(round(top * 10.0) / 10.0),
		size = fmt(size)
	);
	let dot = if o.setup { setup_dot() } else { String::new() };
	svg(&(picture + &label + &frame(o.frame) + &dot))
}

pub fn picture_key(o: &PictureKey) -> String {
	svg_url(&picture_key_svg(o))
}

/// Why Voicenux cannot be reached, in a word or two. Keys show it with a glyph, dials as text.
pub fn status_label(status: &str) -> String {
	t(match status {
		"disabled" => "API off",
		"error" => "Voicenux error",
		_ => "Voicenux off",
	})
}

/// Shown on every key while Voicenux cannot be reached.
pub fn status_key(status: &str) -> String {
	let warn = status == "disabled" || status == "error";
	let color = if warn { colors::AMBER } else { colors::GRAY };
	let mut content = glyph(if warn { "plug" } else { "power" }, &GlyphOpts { x: 44.0, y: 22.0, size: 56.0, color, ..Default::default() });
	let label = status_label(status);
	content += &text(&label, 112.0, fit_size(&label, 20.0, 132.0), if warn { colors::AMBER } else { colors::MUTED });
	svg_url(&svg(&content))
}

/// The icon in the touch strip of a dial, like images/encoder/<name>.svg.
/// setup: gray with a yellow dot, like the setup image of a key.
pub fn dial_icon(name: &str, setup: bool, color: Option<&str>) -> String {
	let color = color.unwrap_or(if setup { colors::GRAY } else { colors::WHITE });
	let mut content = glyph(name, &GlyphOpts { color, ..Default::default() });
	if setup {
		content += &format!(r#"<circle cx="25" cy="-1" r="2.8" fill="{}"/>"#, colors::AMBER);
	}
	svg_url(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="-4 -4 32 32">{content}</svg>"#))
}

/// The picture of a voice for the touch strip of a dial. OpenDeck draws no pictures inside an SVG
/// there, so it gets the PNG itself.
pub fn dial_picture(png: Option<&str>) -> String {
	match png.filter(|p| !p.is_empty()) {
		Some(png) => format!("data:image/png;base64,{png}"),
		None => dial_icon("voice", false, Some(colors::GRAY)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn decode(url: &str) -> String {
		let data = url.split(',').nth(1).unwrap_or("");
		String::from_utf8(base64::engine::general_purpose::STANDARD.decode(data).unwrap_or_default()).unwrap_or_default()
	}

	#[test]
	fn pictures_get_a_frame_in_the_colors_of_voicenux_gray_while_it_is_off() {
		let on = picture_key_svg(&PictureKey { png: Some("AAAA"), title: "Robot", frame: Some(Frame::On), ..Default::default() });
		assert!(on.contains("linearGradient"));
		assert!(on.contains("data:image/png;base64,AAAA"));
		assert!(on.contains(">Robot<"));
		assert!(
			picture_key_svg(&PictureKey { png: Some("AAAA"), frame: Some(Frame::Off), ..Default::default() })
				.contains(r##"stroke="#6B6B6B""##)
		);
		assert!(!picture_key_svg(&PictureKey { png: Some("AAAA"), ..Default::default() }).contains("<rect"));
		// Without a picture the glyph stands in.
		assert!(!picture_key_svg(&PictureKey { title: "Robot", ..Default::default() }).contains("<image"));
		assert!(status_key("disabled").starts_with("data:image/svg+xml;base64,"));
		assert!(decode(&status_key("disabled")).contains("API off"));
	}

	#[test]
	fn keys_have_the_glyph_in_the_middle_and_long_titles_get_cut() {
		let key = decode(&glyph_key("voice", &GlyphKey::default()));
		assert!(!key.contains(r#"<rect width="144""#), "keys have no background");
		assert!(key.contains("translate(28.8 28.8)"), "glyph sits in the middle");
		assert_eq!(title_line("Robot", 128.0).size, 22.0);
		assert!(title_line("A very very very long voice name", 128.0).line.ends_with('…'));
	}
}
