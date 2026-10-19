// Writes the SVG icons in images/ from the glyphs in plugin/lib/icons.js.
// Run after changing a glyph: node scripts/build-icons.mjs
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { COLORS, GLYPHS, actionIconSvg } from "../de.felitendo.voicenux.sdPlugin/plugin/lib/icons.js";
import { glyphKeySvg } from "../de.felitendo.voicenux.sdPlugin/plugin/lib/render.js";
import { KEY_IMAGES } from "../de.felitendo.voicenux.sdPlugin/plugin/lib/key-images.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, "..", "de.felitendo.voicenux.sdPlugin", "images");

const write = (file, content) => {
	fs.mkdirSync(path.dirname(file), { recursive: true });
	fs.writeFileSync(file, content);
};

fs.rmSync(root, { recursive: true, force: true });
for (const name of Object.keys(GLYPHS)) write(path.join(root, "actions", `${name}.svg`), actionIconSvg(name));
for (const [name, spec] of Object.entries(KEY_IMAGES)) {
	write(path.join(root, "keys", `${name}.svg`), glyphKeySvg(spec.glyph, { color: spec.color ?? COLORS.white }) + "\n");
}

// The plugin icon is the icon of Voicenux itself (scripts/assets/voicenux.svg, from the Voicenux
// repo, unchanged). The category icon in the action list is tiny, so there it is the white bars.
write(path.join(root, "plugin.svg"), fs.readFileSync(path.join(here, "assets", "voicenux.svg"), "utf8"));
write(path.join(root, "category.svg"), actionIconSvg("plugin"));
// Dials: icon in the touch strip, and the image in the dial slot of OpenDeck.
for (const name of ["voice", "volume"]) {
	write(path.join(root, "encoder", `${name}.svg`), actionIconSvg(name, "-4 -4 32 32"));
	write(path.join(root, "encoder", `${name}-key.svg`), actionIconSvg(name, "-8 -8 40 40"));
}

console.log(`icons written to ${root}`);
