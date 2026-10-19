// Key images as SVG data URLs (OpenDeck draws them on a 144x144 canvas).
// Same look as the other FeloDeck plugins: no background, a bold glyph in the middle. Voices and
// sounds show their picture from Voicenux instead.
import { t } from "./i18n.js";
import { COLORS, escapeXml, glyph, glyphHeight } from "./icons.js";

export const SIZE = 144;
const FONT = `'Inter','Noto Sans','DejaVu Sans','Liberation Sans',sans-serif`;
// Like the titles OpenDeck draws (font size 11 is about 22 px on a key), but bold to read it on
// the deck. The same in the other FeloDeck plugins.
const TITLE_FONT = `'Liberation Sans',Arial,sans-serif`;
// The glyph takes 24 of 40 units, like viewBox="-8 -8 40 40".
const GLYPH_SIZE = (SIZE * 24) / 40;
const GLYPH_OFFSET = (SIZE - GLYPH_SIZE) / 2;
// Pictures of voices and sounds are drawn with some air of their own, so they get more room.
const PICTURE_SIZE = 100;
const PICTURE_WITH_TITLE = 82;
// Space between a glyph (or picture) and its title.
const TITLE_GAP = 16;

export function svgUrl(svg) {
	return "data:image/svg+xml;base64," + Buffer.from(svg).toString("base64");
}

function svg(content) {
	return `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}">${content}</svg>`;
}

function text(value, { x = SIZE / 2, y, size, color = COLORS.white, weight = 700, anchor = "middle", font = FONT }) {
	return `<text x="${x}" y="${y}" font-family="${font}" font-size="${size}" font-weight="${weight}" fill="${color}" text-anchor="${anchor}">${escapeXml(value)}</text>`;
}

// Font size that keeps a line inside the key. Capital letters are wider.
export function fitSize(value, max, width = 124) {
	const text = String(value ?? "");
	const length = Math.max(1, text.length);
	const perChar = text === text.toUpperCase() && /\p{L}/u.test(text) ? 0.68 : 0.58;
	return Math.max(12, Math.min(max, Math.floor(width / (length * perChar))));
}

// A title the key draws itself, in the font of an OpenDeck title: 22 px like OpenDeck draws them,
// smaller for long text, and cut with "…" when even the smallest size is too wide.
// ink: how tall digits and capitals are, for placing the line.
export function titleLine(value, width = 128) {
	let line = String(value ?? "").trim();
	const perChar = 0.56;
	const size = Math.max(15, Math.min(22, Math.floor(width / (Math.max(1, line.length) * perChar))));
	const fits = Math.floor(width / (size * perChar));
	if (line.length > fits) line = `${line.slice(0, Math.max(1, fits - 1)).trimEnd()}…`;
	return { line, size, ink: Math.round(size * 0.72) };
}

function titleText(line, { x = SIZE / 2, y, color = COLORS.white }) {
	return `<text x="${x}" y="${Math.round(y * 10) / 10}" font-family="${TITLE_FONT}" font-size="${line.size}" font-weight="700" fill="${color}" text-anchor="middle">${escapeXml(line.line)}</text>`;
}

// Something of the given height with a title above or below it, together in the middle of the key.
// top: where it starts. baseline: where the title stands.
function withTitleLayout(height, line, above) {
	const top = (SIZE - height - TITLE_GAP - line.ink) / 2;
	return above ? { top: top + line.ink + TITLE_GAP, baseline: top + line.ink } : { top, baseline: SIZE - top };
}

const setupDot = `<circle cx="120" cy="24" r="10" fill="${COLORS.amber}"/>`;

// A centered glyph.
// setup: a yellow dot in the corner, for a key that still needs its settings.
// title: a line of text "above" or "below" the glyph (titleAt), close to it, and glyph and title
// together in the middle of the key. OpenDeck cannot move its own title, so keys draw it here.
export function glyphKeySvg(name, { color = COLORS.white, setup = false, title = "", titleAt = "below" } = {}) {
	let y = GLYPH_OFFSET;
	let label = "";
	if (title) {
		const line = titleLine(title);
		const height = (glyphHeight(name) * GLYPH_SIZE) / 24;
		const { top, baseline } = withTitleLayout(height, line, titleAt === "above");
		y = GLYPH_OFFSET + top - (SIZE - height) / 2;
		label = titleText(line, { y: baseline });
	}
	return svg(glyph(name, { x: GLYPH_OFFSET, y, size: GLYPH_SIZE, color }) + label + (setup ? setupDot : ""));
}

export function glyphKey(name, options) {
	return svgUrl(glyphKeySvg(name, options));
}

// The frame around a voice in use or a sound that plays: the gradient of Voicenux.
// "on" is the gradient, "off" gray (the voice is picked, but the voice changer is off).
function frame(kind) {
	if (!kind) return "";
	const stroke = kind === "off" ? COLORS.gray : "url(#vn)";
	const defs = kind === "off" ? "" : `<defs><linearGradient id="vn" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${COLORS.blue}"/><stop offset="0.5" stop-color="${COLORS.sky}"/><stop offset="1" stop-color="${COLORS.cyan}"/></linearGradient></defs>`;
	return `${defs}<rect x="4" y="4" width="${SIZE - 8}" height="${SIZE - 8}" rx="16" fill="none" stroke="${stroke}" stroke-width="8"/>`;
}

// The picture of a voice or sound (base64 PNG from Voicenux), with a title and a frame.
// Without a picture (Voicenux sent none), the glyph stands in for it.
// src: a URL of the picture instead, for the settings pages. dim: drawn faded, like a gray glyph.
export function pictureKeySvg({ png = null, src = null, fallback = "voice", title = "", titleAt = "below", frame: kind = null, dim = false, setup = false }) {
	if (!png && !src) return glyphKeySvg(fallback, { title, titleAt, color: dim ? COLORS.gray : COLORS.white, setup }).replace("</svg>", `${frame(kind)}</svg>`);
	const size = title ? PICTURE_WITH_TITLE : PICTURE_SIZE;
	let top = (SIZE - size) / 2;
	let label = "";
	if (title) {
		const line = titleLine(title, kind ? 112 : 128);
		const layout = withTitleLayout(size, line, titleAt === "above");
		top = layout.top;
		label = titleText(line, { y: layout.baseline, color: dim ? COLORS.muted : COLORS.white });
	}
	const href = src ?? `data:image/png;base64,${png}`;
	const fade = dim ? ` opacity="0.4"` : "";
	const picture = `<image x="${(SIZE - size) / 2}" y="${Math.round(top * 10) / 10}" width="${size}" height="${size}"${fade} href="${href}" xlink:href="${href}"/>`;
	return svg(picture + label + frame(kind) + (setup ? setupDot : ""));
}

export function pictureKey(options) {
	return svgUrl(pictureKeySvg(options));
}

// Why Voicenux cannot be reached, in a word or two. Keys show it with a glyph, dials as text.
export function statusLabel(status) {
	return t(status === "disabled" ? "API off" : status === "error" ? "Voicenux error" : "Voicenux off");
}

// Shown on every key while Voicenux cannot be reached.
export function statusKey(status) {
	const warn = status === "disabled" || status === "error";
	const color = warn ? COLORS.amber : COLORS.gray;
	let content = glyph(warn ? "plug" : "power", { x: 44, y: 22, size: 56, color });
	const label = statusLabel(status);
	content += text(label, { y: 112, size: fitSize(label, 20, 132), color: warn ? COLORS.amber : COLORS.muted });
	return svgUrl(svg(content));
}

// The icon in the touch strip of a dial, like images/encoder/<name>.svg.
// setup: gray with a yellow dot, like the setup image of a key.
export function dialIcon(name, { setup = false, color } = {}) {
	let content = glyph(name, { color: color ?? (setup ? COLORS.gray : COLORS.white) });
	if (setup) content += `<circle cx="25" cy="-1" r="2.8" fill="${COLORS.amber}"/>`;
	return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="-4 -4 32 32">${content}</svg>`);
}

// The picture of a voice for the touch strip of a dial. OpenDeck draws no pictures inside an SVG
// there, so it gets the PNG itself.
export function dialPicture(png) {
	return png ? `data:image/png;base64,${png}` : dialIcon("voice", { color: COLORS.gray });
}
