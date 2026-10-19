import { FIT } from "./glyph-fit.js";

// Glyphs drawn on a 24x24 grid. Bold, like the other FeloDeck plugins: a body is always filled
// (details are cut out of it), round 2.4px strokes only for lines like arrows and waves.
// Used for the static icons (scripts/build-icons.mjs) and for keys drawn at runtime.

const fill = `fill="currentColor"`;
// A filled body with details cut out of it. Cuts are lines in the stroke width, or shapes with
// fill="#000"; add puts parts back on top of a cut.
const cut = (id, body, cuts, add = "") =>
	`<mask id="cut-${id}"><g fill="#FFF" stroke="none">${body}</g><g fill="none" stroke="#000">${cuts}</g><g fill="none" stroke="#FFF">${add}</g></mask>` +
	`<rect x="-4" y="-4" width="32" height="32" ${fill} stroke="none" mask="url(#cut-${id})"/>`;
const speaker = `<path d="M3.5 9.2h3.2L11.5 5v14l-4.8-4.2H3.5a1 1 0 0 1-1-1v-3.6a1 1 0 0 1 1-1z" ${fill} stroke-width="1.6"/>`;
const bar = (x, y, h) => `<rect x="${x}" y="${y}" width="3" height="${h}" rx="1.5" ${fill} stroke="none"/>`;
const capsule = `<rect x="8" y="1.8" width="8" height="13" rx="4"/>`;
const cups = `<path d="M3 13.5h3.2a1.3 1.3 0 0 1 1.3 1.3v5.4a1.3 1.3 0 0 1-1.3 1.3H4.3A1.3 1.3 0 0 1 3 20.2zM21 13.5h-3.2a1.3 1.3 0 0 0-1.3 1.3v5.4a1.3 1.3 0 0 0 1.3 1.3h1.9a1.3 1.3 0 0 0 1.3-1.3z" ${fill} stroke-width="1.4"/>`;
const mask = `<path d="M3 4.2c2.6-1.6 15.4-1.6 18 0v6.6c0 6.2-4.3 10.6-9 10.6S3 17 3 10.8z" stroke="#FFF" stroke-width="1.2" stroke-linejoin="round"/>`;
const face = `<path d="M6.6 9.6c.8-.9 2.4-.9 3.2 0M14.2 9.6c.8-.9 2.4-.9 3.2 0M8 14.6c2 2.3 6 2.3 8 0"/>`;
const pip = (x, y) => `<circle cx="${x}" cy="${y}" r="1.75" fill="#000" stroke="none"/>`;

export const GLYPHS = {
	// Voicenux itself: the five bars of its icon.
	plugin: bar(2, 6, 12) + bar(6.75, 9.5, 8.5) + bar(10.5, 13, 5) + bar(14.25, 9.5, 8.5) + bar(19, 6, 12),
	voicechanger: bar(2, 3.5, 17) + bar(6.75, 7.5, 9) + bar(10.5, 10, 4) + bar(14.25, 7.5, 9) + bar(19, 3.5, 17),
	// A mask: a voice that is not yours.
	voice: cut("voice", mask, face),
	// The mask struck through: no voice on, your own one.
	ownVoice: cut("ownVoice", mask, `${face}<path d="m3 3 18 18" stroke-width="6"/>`, `<path d="m3 3 18 18"/>`),
	// Two beamed notes.
	sound: `<path d="M8.6 17.6V6.2l11-2.4v11.4" stroke-width="2.6"/><path d="M8.6 6.2l11-2.4v3.6l-11 2.4z" ${fill} stroke-width="1.4"/><ellipse cx="6" cy="17.8" rx="3.4" ry="2.8" ${fill} stroke="none"/><ellipse cx="17" cy="15.4" rx="3.4" ry="2.8" ${fill} stroke="none"/>`,
	mic: `${capsule.replace("/>", ` ${fill} stroke="none"/>`)}<path d="M4.6 11.2a7.4 7.4 0 0 0 14.8 0M12 18.6v3.2"/>`,
	micOff: cut("micOff", `${capsule}<path d="M4.6 11.2a7.4 7.4 0 0 0 14.8 0M12 18.6v3.2" fill="none" stroke="#FFF"/>`, `<path d="m3 3 18 18" stroke-width="6"/>`, `<path d="m3 3 18 18"/>`),
	headphones: `<path d="M4.3 15.5v-3a7.7 7.7 0 0 1 15.4 0v3"/>${cups}`,
	hearsounds: `<path d="M4.3 15.5v-3a7.7 7.7 0 0 1 15.4 0v3"/>${cups}<path d="M12.6 19.4v-8.2l2.6.8" stroke-width="2"/><circle cx="11" cy="19.6" r="2.1" ${fill} stroke="none"/>`,
	background: `<path d="M2.5 7.5c1.6-1.2 3.1-1.2 4.7 0s3.1 1.2 4.7 0 3.1-1.2 4.7 0 3.1 1.2 4.7 0M2.5 12.5c1.6-1.2 3.1-1.2 4.7 0s3.1 1.2 4.7 0 3.1-1.2 4.7 0 3.1 1.2 4.7 0M2.5 17.5c1.6-1.2 3.1-1.2 4.7 0s3.1 1.2 4.7 0 3.1-1.2 4.7 0 3.1 1.2 4.7 0"/>`,
	random: cut("random", `<rect x="2.5" y="2.5" width="19" height="19" rx="4.4"/>`, pip(7.4, 7.4) + pip(16.6, 7.4) + pip(12, 12) + pip(7.4, 16.6) + pip(16.6, 16.6)),
	next: `<path d="M5 6l9.5 6L5 18z" ${fill}/><rect x="16.3" y="5" width="3.2" height="14" rx="1.2" ${fill} stroke="none"/>`,
	previous: `<path d="M19 6 9.5 12l9.5 6z" ${fill}/><rect x="4.5" y="5" width="3.2" height="14" rx="1.2" ${fill} stroke="none"/>`,
	stop: `<rect x="5.5" y="5.5" width="13" height="13" rx="2.2" ${fill} stroke="none"/>`,
	volume: `${speaker}<path d="M15.2 8.8a4.6 4.6 0 0 1 0 6.4M18.3 5.8a9 9 0 0 1 0 12.4"/>`,
	volumeUp: `${speaker}<path d="M15.5 12h6M18.5 9v6"/>`,
	volumeDown: `${speaker}<path d="M15.5 12h6"/>`,
	volumeSet: `${speaker}<path d="M15.5 10h6M15.5 14h6"/>`,
	power: `<path d="M12 3v9"/><path d="M18.4 6.6a9 9 0 1 1-12.8 0"/>`,
	// Like the icon of "Other apps" in the settings of Voicenux.
	plug: `<path d="M9 2.5v4.5M15 2.5v4.5"/><path d="M5.5 7h13v3.6a6.5 6.5 0 0 1-13 0z" ${fill} stroke-width="1.6"/><path d="M12 17.1v4.4"/>`,
	star: `<path d="m12 2.8 2.8 5.8 6.3.9-4.6 4.4 1.1 6.3-5.6-3-5.6 3 1.1-6.3-4.6-4.4 6.3-.9z" ${fill} stroke-width="1.6"/>`,
};

export const COLORS = {
	white: "#FFFFFF",
	gray: "#6B6B6B",
	muted: "#9A9AA2",
	red: "#E5484D",
	green: "#30A46C",
	amber: "#F5A524",
	// The gradient of Voicenux, deep blue to cyan.
	blue: "#2563EB",
	sky: "#3B82F6",
	cyan: "#22D3EE",
};

export function escapeXml(text) {
	return String(text ?? "").replace(/[<>&"']/g, (c) => ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", '"': "&quot;", "'": "&apos;" })[c]);
}

// The glyph of a soundboard volume key for each mode.
export const VOLUME_GLYPHS = { up: "volumeUp", down: "volumeDown", set: "volumeSet" };

// How much of the 24 grid a glyph may use: wide ones up to MAX, compact ones less,
// so a circle does not look bigger than a wide eye.
const MAX = 20.5;
const AREA = 17.5;

const fitScale = (box) => Math.min(MAX / Math.max(box.w, box.h), AREA / Math.sqrt(box.w * box.h));

// How tall a glyph is drawn, in units of the 24 grid.
export function glyphHeight(name) {
	const box = FIT[name];
	return box ? box.h * fitScale(box) : 24;
}

// The fitted box of a glyph: centered on (12, 12) and with an even optical size.
// FIT holds the measured bounds, see scripts/fit-glyphs.mjs.
function fitTransform(name) {
	const box = FIT[name];
	if (!box) return { inner: "", scale: 1 };
	const k = fitScale(box);
	const n = (v) => Math.round(v * 1000) / 1000;
	return { inner: `translate(12 12) scale(${n(k)}) translate(${n(-box.cx)} ${n(-box.cy)})`, scale: k };
}

// A glyph placed at (x, y) with the given size in pixels.
// fit: false draws it as written, used to measure it.
export function glyph(name, { x = 0, y = 0, size = 24, color = COLORS.white, stroke = 2.4, fit = true } = {}) {
	const body = GLYPHS[name] ?? GLYPHS.plugin;
	const n = (v) => Math.round(v * 1000) / 1000;
	const { inner, scale } = fit ? fitTransform(name) : { inner: "", scale: 1 };
	// Keep the stroke the same on screen, whatever the glyph was scaled by.
	const width = n(stroke / scale);
	const outer = `<g transform="translate(${n(x)} ${n(y)}) scale(${n(size / 24)})" fill="none" stroke="${color}" color="${color}" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round">`;
	return inner ? `${outer}<g transform="${inner}">${body}</g></g>` : `${outer}${body}</g>`;
}

// Icon for the action list: white glyph, no background.
export function actionIconSvg(name, viewBox = "0 0 24 24") {
	return `<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="${viewBox}">${glyph(name)}</svg>\n`;
}
