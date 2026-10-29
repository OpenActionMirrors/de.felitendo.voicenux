// Key images as SVG data URLs (OpenDeck draws them on a 144x144 canvas).
// Same look as the other FeloDeck plugins: no background, a bold glyph in the middle. Voices and
// sounds show their picture from Voicenux instead.
// pi/lib/render.js draws the same pictures for the settings pages, keep the two alike.
use base64::Engine;

use crate::i18n::t;
use crate::icons::{GlyphOpts, colors, escape_xml, glyph, glyph_height};
use crate::js::{fmt, max, min, round};
use crate::voicenux::Status;

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
// The small glyph on a picture, and how far its box reaches past the corner of the picture.
const BADGE: f64 = 46.0;
const BADGE_OUT: f64 = 10.0;

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

/// Length as pi/lib/render.js counts it, in UTF-16 units.
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
		// Cut by UTF-16 units, like pi/lib/render.js. Half an emoji becomes U+FFFD there too.
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
	svg(&glyph_key_content(name, o))
}

fn glyph_key_content(name: &str, o: &GlyphKey) -> String {
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
	glyph(name, &GlyphOpts { x: GLYPH_OFFSET, y, size: GLYPH_SIZE, color: o.color, ..Default::default() }) + &label + &dot
}

pub fn glyph_key(name: &str, o: &GlyphKey) -> String {
	svg_url(&glyph_key_svg(name, o))
}

// The edge a light runs round while Say a Sound looks: where the frame of a voice in use is.
const EDGE_SIDE: f64 = SIZE - 8.0;
const EDGE_CORNER: f64 = 16.0;
const EDGE_STRAIGHT: f64 = EDGE_SIDE - 2.0 * EDGE_CORNER;
const EDGE_QUARTER: f64 = std::f64::consts::FRAC_PI_2 * EDGE_CORNER;
const EDGE: f64 = 4.0 * (EDGE_STRAIGHT + EDGE_QUARTER);

/// Say a Sound while it looks: the lens stays, and a light runs round the edge of the key. Its
/// head is cyan and glows, its tail turns deep blue and fades, like the gradient of Voicenux.
/// `t` is how long it has looked, in seconds. Keys draw it again and again, a Stream Deck shows
/// no moving pictures.
pub fn searching_key_svg(t: f64, o: &GlyphKey) -> String {
	const PIECES: usize = 36;
	const TAIL: f64 = 200.0;
	// Once round in 1.5 s.
	let head = (t / 1.5).fract() * EDGE;
	let piece = TAIL / PIECES as f64;
	let edge = |color: &str, opacity: f64, width: f64, from: f64, length: f64| {
		format!(
			r#"<rect x="4" y="4" width="{s}" height="{s}" rx="{EDGE_CORNER}" fill="none" stroke="{color}" stroke-opacity="{}" stroke-width="{width}" stroke-dasharray="{} {}" stroke-dashoffset="{}"/>"#,
			fmt(n2(opacity)),
			fmt(n2(length)),
			fmt(n2(EDGE - length)),
			fmt(n2((-from).rem_euclid(EDGE))),
			s = fmt(EDGE_SIDE)
		)
	};
	// The glow: the front of the light, blurred.
	let mut light = format!(
		r#"<filter id="glow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="4"/></filter><g filter="url(#glow)" opacity="0.7">{}</g>"#,
		edge(colors::CYAN, 1.0, 10.0, head - 50.0, 50.0)
	);
	// Short pieces one after the other, a little over each other so no seam shows. Keys are black
	// behind, so the tail fades into black in its colors: see-through pieces would show where
	// they lie over each other.
	for i in 0..PIECES {
		let k = i as f64 / (PIECES - 1) as f64;
		let color = if k < 0.5 { mix(colors::CYAN, colors::SKY, k * 2.0) } else { mix(colors::SKY, colors::BLUE, k * 2.0 - 1.0) };
		let color = mix("#000000", &color, (1.0 - k).powf(1.5));
		light += &edge(&color, 1.0, 8.0, head - (i + 1) as f64 * piece, piece + 0.6);
	}
	// A round head.
	let (x, y) = on_edge(head);
	light += &format!(r#"<circle cx="{}" cy="{}" r="4" fill="{}"/>"#, fmt(n2(x)), fmt(n2(y)), colors::CYAN);
	svg(&(light + &glyph_key_content("saysound", o)))
}

/// The point `d` along the edge, from the start of its top side on, clockwise like SVG draws it.
fn on_edge(d: f64) -> (f64, f64) {
	let d = d.rem_euclid(EDGE);
	let side = (d / (EDGE_STRAIGHT + EDGE_QUARTER)).floor();
	let u = d - side * (EDGE_STRAIGHT + EDGE_QUARTER);
	// On the top side, then turned round the middle of the key a quarter for each side.
	let (x, y) = if u < EDGE_STRAIGHT {
		(4.0 + EDGE_CORNER + u, 4.0)
	} else {
		let a = (u - EDGE_STRAIGHT) / EDGE_CORNER;
		(4.0 + EDGE_SIDE - EDGE_CORNER + EDGE_CORNER * a.sin(), 4.0 + EDGE_CORNER - EDGE_CORNER * a.cos())
	};
	let c = SIZE / 2.0;
	(0..side as usize).fold((x, y), |(x, y), _| (c - (y - c), c + (x - c)))
}

/// A color between `a` and `b` ("#RRGGBB"): `a` at 0, `b` at 1.
fn mix(a: &str, b: &str, k: f64) -> String {
	let channel = |c: &str, i: usize| u8::from_str_radix(&c[1 + 2 * i..3 + 2 * i], 16).unwrap_or(0) as f64;
	let rgb: Vec<String> = (0..3).map(|i| format!("{:02X}", round(channel(a, i) + (channel(b, i) - channel(a, i)) * k) as u8)).collect();
	format!("#{}", rgb.concat())
}

pub fn searching_key(t: f64, o: &GlyphKey) -> String {
	svg_url(&searching_key_svg(t, o))
}

fn n2(v: f64) -> f64 {
	round(v * 100.0) / 100.0
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
	/// A small glyph in the top right corner, like the lens of Say a Sound. "" for none.
	pub badge: &'a str,
}

impl Default for PictureKey<'_> {
	fn default() -> Self {
		PictureKey { png: None, fallback: "voice", title: "", title_at: "below", frame: None, dim: false, setup: false, badge: "" }
	}
}

/// A small glyph on the bottom right corner (right, bottom) of a picture.
fn badge(name: &str, right: f64, bottom: f64) -> String {
	if name.is_empty() {
		return String::new();
	}
	glyph(name, &GlyphOpts { x: right - BADGE + BADGE_OUT, y: bottom - BADGE + BADGE_OUT, size: BADGE, ..Default::default() })
}

pub fn picture_key_svg(o: &PictureKey) -> String {
	let size = if o.title.is_empty() { PICTURE_SIZE } else { PICTURE_WITH_TITLE };
	let mut top = (SIZE - size) / 2.0;
	let mut label = String::new();
	if !o.title.is_empty() {
		let line = title_line(o.title, if o.frame.is_some() { 112.0 } else { 128.0 });
		let (start, baseline) = with_title_layout(size, &line, o.title_at == "above");
		top = start;
		label = title_text(&line, baseline, if o.dim { colors::MUTED } else { colors::WHITE });
	}
	// Also on the glyph that stands in for the picture.
	let corner = badge(o.badge, (SIZE + size) / 2.0, top + size);
	let Some(png) = o.png.filter(|p| !p.is_empty()) else {
		let color = if o.dim { colors::GRAY } else { colors::WHITE };
		let key = glyph_key_svg(o.fallback, &GlyphKey { color, setup: o.setup, title: o.title, title_at: o.title_at });
		return key.replacen("</svg>", &format!("{}{corner}</svg>", frame(o.frame)), 1);
	};
	let href = format!("data:image/png;base64,{png}");
	let fade = if o.dim { r#" opacity="0.4""# } else { "" };
	let picture = format!(
		r#"<image x="{}" y="{}" width="{size}" height="{size}"{fade} href="{href}" xlink:href="{href}"/>"#,
		fmt((SIZE - size) / 2.0),
		fmt(round(top * 10.0) / 10.0),
		size = fmt(size)
	);
	let dot = if o.setup { setup_dot() } else { String::new() };
	svg(&(picture + &label + &frame(o.frame) + &corner + &dot))
}

pub fn picture_key(o: &PictureKey) -> String {
	svg_url(&picture_key_svg(o))
}

/// Why Voicenux cannot be reached, in a word or two. Keys show it with a glyph, dials as text.
pub fn status_label(status: Status) -> String {
	t(match status {
		Status::Disabled => "API off",
		Status::Asking => "Allow?",
		Status::Refused => "Not allowed",
		Status::Error => "Voicenux error",
		_ => "Voicenux off",
	})
}

/// Shown on every key while Voicenux cannot be reached.
pub fn status_key(status: Status) -> String {
	let warn = matches!(status, Status::Disabled | Status::Asking | Status::Refused | Status::Error);
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
/// Without one, the glyph `fallback` stands in, in gray.
pub fn dial_picture(png: Option<&str>, fallback: &str) -> String {
	match png.filter(|p| !p.is_empty()) {
		Some(png) => format!("data:image/png;base64,{png}"),
		None => dial_icon(fallback, false, Some(colors::GRAY)),
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
		assert!(status_key(Status::Disabled).starts_with("data:image/svg+xml;base64,"));
		assert!(decode(&status_key(Status::Disabled)).contains("API off"));
	}

	#[test]
	fn keys_have_the_glyph_in_the_middle_and_long_titles_get_cut() {
		let key = decode(&glyph_key("voice", &GlyphKey::default()));
		assert!(!key.contains(r#"<rect width="144""#), "keys have no background");
		assert!(key.contains("translate(28.8 28.8)"), "glyph sits in the middle");
		assert_eq!(title_line("Robot", 128.0).size, 22.0);
		assert!(title_line("A very very very long voice name", 128.0).line.ends_with('…'));
	}

	#[test]
	fn a_light_runs_round_while_it_looks() {
		let o = GlyphKey::default();
		let frames: Vec<String> = [0.0, 0.3, 0.6, 1.0].iter().map(|t| searching_key_svg(*t, &o)).collect();
		assert!(frames.windows(2).all(|w| w[0] != w[1]), "it moves");
		assert_eq!(searching_key_svg(0.25, &o), searching_key_svg(1.75, &o), "round once in 1.5 s");
		// The lens stays as it is.
		let lens = glyph_key_content("saysound", &o);
		assert!(frames.iter().all(|f| f.contains(&lens)));
		let titled = searching_key_svg(0.5, &GlyphKey { title: "Vine boom", ..Default::default() });
		assert!(titled.contains(">Vine boom</text>"));
	}

	#[test]
	fn the_light_stays_on_the_edge() {
		let near = |(x, y): (f64, f64), (ex, ey): (f64, f64)| (x - ex).abs() < 0.01 && (y - ey).abs() < 0.01;
		assert!(near(on_edge(0.0), (20.0, 4.0)), "it starts at the top, after the corner");
		let quarter = EDGE / 4.0;
		assert!(near(on_edge(EDGE_STRAIGHT / 2.0), (72.0, 4.0)));
		assert!(near(on_edge(quarter + EDGE_STRAIGHT / 2.0), (140.0, 72.0)), "then down the right side");
		assert!(near(on_edge(2.0 * quarter + EDGE_STRAIGHT / 2.0), (72.0, 140.0)));
		assert!(near(on_edge(3.0 * quarter + EDGE_STRAIGHT / 2.0), (4.0, 72.0)));
		assert!(near(on_edge(EDGE), on_edge(0.0)));
		assert_eq!(mix("#000000", "#FF8040", 0.5), "#804020");
	}
}
