// The SVG icons in images/, from the glyphs in src/icons.rs.
use std::fs;
use std::process::Command;

use voicenux_plugin::icons::{GLYPHS, GlyphOpts, action_icon_svg, colors, glyph};
use voicenux_plugin::key_images::KEY_IMAGES;
use voicenux_plugin::render::{GlyphKey, glyph_key_svg};

use crate::{Result, plugin_dir, root, write};

/// Writes images/ (and pi/lib/shared.js, which holds the glyphs too).
pub fn run() -> Result {
	let images = plugin_dir().join("images");
	let _ = fs::remove_dir_all(&images);
	for (name, _) in GLYPHS.iter() {
		write(&images.join("actions").join(format!("{name}.svg")), action_icon_svg(name, "0 0 24 24"))?;
	}
	for (name, spec) in KEY_IMAGES {
		write(
			&images.join("keys").join(format!("{name}.svg")),
			glyph_key_svg(spec.glyph, &GlyphKey { color: spec.color.unwrap_or(colors::WHITE), ..Default::default() }) + "\n",
		)?;
	}

	// The plugin icon is the icon of Voicenux itself (xtask/assets/voicenux.svg, from the Voicenux
	// repo, unchanged). The category icon in the action list is tiny, so there it is the white bars.
	let icon = fs::read(root().join("xtask").join("assets").join("voicenux.svg")).map_err(|e| format!("voicenux.svg: {e}"))?;
	write(&images.join("plugin.svg"), icon)?;
	write(&images.join("category.svg"), action_icon_svg("plugin", "0 0 24 24"))?;
	// Dials: icon in the touch strip, and the image in the dial slot of OpenDeck.
	for name in ["voice", "sound", "volume"] {
		write(&images.join("encoder").join(format!("{name}.svg")), action_icon_svg(name, "-4 -4 32 32"))?;
		write(&images.join("encoder").join(format!("{name}-key.svg")), action_icon_svg(name, "-8 -8 40 40"))?;
	}
	write(&root().join(voicenux_plugin::pi::SHARED_JS), voicenux_plugin::pi::shared_js())?;
	println!("icons written to {}", images.display());
	Ok(())
}

const PX: f64 = 20.0; // pixels per grid unit
const PAD: f64 = 4.0; // grid units around the 24 box, for strokes that stick out
const OPTICAL: f64 = 0.5; // 0 = middle of the box, 1 = center of mass

/// Measures every glyph and writes src/glyph_fit.rs, so icons can be centered and sized evenly.
///
/// Centered by its box alone, a glyph with its weight on one side looks off: a play triangle seems
/// too far left, a speaker too. So the center used is halfway between the middle of the box and the
/// center of mass of the ink (the optical center).
pub fn fit_glyphs() -> Result {
	let tmp = std::env::temp_dir().join(format!("glyphs-{}", std::process::id()));
	fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
	let mut lines = Vec::new();
	for (name, _) in GLYPHS.iter() {
		let file = tmp.join(format!("{name}.svg"));
		let size = (24.0 + PAD * 2.0) * PX;
		let body = glyph(name, &GlyphOpts { fit: false, color: "#000000", ..Default::default() });
		let svg = format!(
			r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="{} {} {} {}">{body}</svg>"#,
			-PAD,
			-PAD,
			24.0 + PAD * 2.0,
			24.0 + PAD * 2.0
		);
		write(&file, svg)?;
		// Raw 8-bit gray pixels: 0 is ink, 255 is paper.
		let output = Command::new("magick")
			.args(["-background", "white"])
			.arg(&file)
			.args(["-flatten", "-colorspace", "gray", "-depth", "8", "gray:-"])
			.output()
			.map_err(|e| format!("ImageMagick (magick) is needed: {e}"))?;
		if !output.status.success() {
			return Err(format!("magick failed for {name}: {}", String::from_utf8_lossy(&output.stderr)));
		}
		let raw = output.stdout;
		// ImageMagick may render at another size than asked, so measure against what it made.
		let width = (raw.len() as f64).sqrt().round() as usize;
		let px = width as f64 / (24.0 + PAD * 2.0);
		let (mut x0, mut y0, mut x1, mut y1): (f64, f64, f64, f64) = (width as f64, width as f64, -1.0, -1.0);
		let (mut mass, mut mx, mut my) = (0.0, 0.0, 0.0);
		for y in 0..width {
			for x in 0..width {
				let ink = (255.0 - f64::from(raw[y * width + x])) / 255.0;
				if ink < 0.1 {
					continue;
				}
				let (xf, yf) = (x as f64, y as f64);
				x0 = x0.min(xf);
				y0 = y0.min(yf);
				x1 = x1.max(xf);
				y1 = y1.max(yf);
				mass += ink;
				mx += ink * (xf + 0.5);
				my += ink * (yf + 0.5);
			}
		}
		let (bx, by) = ((x0 + x1 + 1.0) / 2.0, (y0 + y1 + 1.0) / 2.0);
		let cx = bx + OPTICAL * (mx / mass - bx);
		let cy = by + OPTICAL * (my / mass - by);
		let round = |v: f64| voicenux_plugin::js::round(v * 1000.0) / 1000.0;
		lines.push(format!(
			"\t(\"{name}\", Fit {{ cx: {:?}, cy: {:?}, w: {:?}, h: {:?} }}),",
			round(cx / px - PAD),
			round(cy / px - PAD),
			round((x1 - x0 + 1.0) / px),
			round((y1 - y0 + 1.0) / px)
		));
	}
	let _ = fs::remove_dir_all(&tmp);
	let out = root().join("src").join("glyph_fit.rs");
	let code = format!(
		"// Written by `cargo xtask fit-glyphs`: center and size of every glyph, in grid units.\nuse crate::icons::Fit;\n\npub static FIT: &[(&str, Fit)] = &[\n{}\n];\n",
		lines.join("\n")
	);
	write(&out, code)?;
	println!("measured {} glyphs -> src/glyph_fit.rs", lines.len());
	Ok(())
}
