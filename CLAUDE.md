# CLAUDE.md

## Style

- Keep everything short: replies, explanations, comments, docs.
- Use simple English: short sentences, common words.
- Avoid em dashes (—). Do not just swap them for "-" either. Rewrite the sentence instead, for
  example with a comma, a colon, brackets or two sentences.

## Commits

- English only.
- Conventional Commits prefix: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `ci:`, `build:`, `test:`.
- Subject as short as possible. Imperative, lowercase, no trailing period.
- Body only when something genuinely cannot be inferred from the diff.
- Never add `Co-Authored-By`, "Generated with" or any other AI attribution to commits or PR descriptions.
- Do not commit or push before I have checked the changes locally and said they are fine. Then
  commit and push. Several attempts at the same thing make one commit, and attempts that did not
  work make none. Different things done in one session get a commit each. This also goes for
  releases and tags.

## Project

- An OpenDeck plugin for the voice changer [Voicenux](https://github.com/voicenux/voicenux). It is
  built like [OBS Tools](https://github.com/FeloDeck/obs-tools): same base, settings pages, icons
  and tools. When something is fixed or improved in one, look if the other needs it too.
- The plugin is Rust (`src/`) and talks to OpenDeck through the `openaction` crate. It runs on one
  thread: every event is a task of its own (tokio `LocalSet`). Never hold a `RefCell` borrow across
  an `.await`. Settings come from the settings pages as loose JSON ("5" for 5, null for ""). Each
  action reads them into its own struct with `k.settings::<T>()` (`#[derive(Deserialize, Default)]`
  and `#[serde(default, rename_all = "camelCase")]`): `src/settings.rs` reads every field the way
  JavaScript would. Use `String`, `bool`, `f64` or `serde_json::Value` fields, nothing that can fail.
- `make package` builds for x86_64 and aarch64 with an old glibc (2.28), so it runs on older
  distributions too. It needs zig, `cargo-zigbuild` and the Rust library for aarch64 (Arch:
  `rust-aarch64-gnu`). `make install` only builds for this PC. Dev tools are Rust too:
  `cargo xtask` lists them.
- The binary is `voicenux-plugin`. Never name it `voicenux`: the plugin looks for a process of that
  name to tell "API off" from "not running".
- Plugin ID is `de.felitendo.voicenux` (folder `de.felitendo.voicenux.sdPlugin`). Do not change
  it: OpenDeck keys profiles and settings by it.
- It talks to Voicenux over the Voicenux API (`API.md` in the Voicenux repo): one WebSocket on
  `127.0.0.1:41967` for all keys, in `src/voicenux.rs`. Voicenux sends the state, the voices,
  the sounds and what plays, and every change after that. Keys never poll. The API is off until the
  user turns on **Settings → Other apps → Let other apps control Voicenux**. A refused connection
  while a `voicenux` process runs means just that (status `disabled`), and the keys say "API off".
- Pictures of voices and sounds are the Fluent Emoji of Voicenux. The plugin asks Voicenux for them
  (`icon`) and keeps them while it is connected. Never put them in this repo. The settings pages
  load them straight from `http://127.0.0.1:41967/api/icon/NAME`.
- New features of Voicenux that a key should use go into the API first (in the Voicenux repo, with
  `API.md`), not around it.
- `manifest.json`, `de.json`, `pi/lib/shared.js` and the icons in `images/` are generated. Change
  the actions in `src/manifest.rs` or the glyphs in `src/icons.rs`, then run `make manifest` or
  `make icons`. `make icons` first measures every glyph (`cargo xtask fit-glyphs`, needs
  ImageMagick), so all icons end up the same optical size and optically centered. The action order
  in `src/manifest.rs` is the order in OpenDeck: most used first. The plugin icon is the icon of
  Voicenux (`xtask/assets/voicenux.svg`).
- The settings pages stay JavaScript (`pi/`): they run in OpenDeck's web view. What they share with
  the plugin (German texts, glyphs) comes from Rust through `pi/lib/shared.js`. `pi/lib/render.js`
  draws the same key pictures as `src/render.rs`: change both, `make check` compares them.
- Settings pages come from `pi/schemas.js`. A new setting needs a default in the `defaults` of the
  action (its `Kind` in `src/actions/`), a field in its settings struct and a field in the schema.
  The test `every_setting_has_a_default` checks that each field has a default. The status line is on top; while Voicenux cannot be reached, a box below
  it says what to do, with a button that runs `voicenux open settings`. The settings of the key
  follow in sections (`section` in `pi/schemas.js`): what the key is about, Controls, On the key,
  and closed sections for the rest. Use the own `dropdown()`, `checkbox()` and number stepper in
  `pi/pi.js`, never native `<select>` or checkboxes: OpenDeck's WebKitGTK draws those in the GTK
  style.
- An action that can do more than one thing lists it in `ops` of its `Kind` (the press choice is in
  the setting `press_key`), and its schema uses `gestures()`. Press, hold and double press then
  offer the same choices, and `src/actions/key/` does the key handling.
- Settings a key cannot work without go in `required`, so the key shows the setup image (gray
  glyph, yellow dot) until they are set. Errors an action returns show up on its settings page:
  write them as short English sentences and add the German text.
- Everything a user can read comes in English and German. Write the English text in the code and
  wrap it in `t()` (`src/i18n.rs`, or `pi/lib/i18n.js` on a settings page), then add the German
  text to `src/i18n/de.rs` and run `make manifest`. Use the words of Voicenux (Stimmverzerrer, Mich
  hören, Hintergrund, Sounds) and "du". Action names and tooltips for the OpenDeck list are in
  `src/manifest.rs` (it writes `de.json`). Messages the plugin sends
  to a settings page stay English, the page translates them with `translateMessage()`.
  `make check` fails when a German text is missing.
- Keys look like the other FeloDeck plugins: no background, bold white glyph in the middle, gray
  when off. Voice and sound keys show their picture instead, with a frame in the gradient of
  Voicenux (deep blue to cyan) while the voice is in use or the sound plays.
- Test in the real apps. First check if a real Stream Deck is plugged in (`lsusb | grep -i 0fd9`,
  Elgato's USB vendor ID). If one is, test on it, but only in new profiles of its own, with a name
  that is not taken yet (for example "Voicenux Test"). Never change or delete the profiles that
  are already on it, they are in use. Note which profile was selected before
  (`~/.config/opendeck/profiles/<device>.json`) and select it again when done. `cargo xtask
  profile` only writes profiles for the virtual deck.
- Without a real Stream Deck, use the virtual deck of OBS Tools (`tools/virtual-deck` in
  FeloDeck/obs-tools, `make install-virtual-deck` there). Put the keys on it with
  `cargo xtask profile test/e2e/layout.json "Voicenux Test"` while OpenDeck is stopped, then run
  `cargo xtask e2e`. It writes the selected profile, so select the old one again after.
- Test against a Voicenux of its own when one already runs: Voicenux finds its PipeWire nodes by
  name, so two of them in one PipeWire get mixed up. Give the test one its own `XDG_CONFIG_HOME`
  (with `api = true`), its own `dbus-daemon`, a `kwin_wayland --virtual` and its own PipeWire
  (`PIPEWIRE_CORE=vn-pw pipewire`, Voicenux with `PIPEWIRE_REMOTE=vn-pw`), and start it with
  `voicenux open voices`. Socket paths must stay short (108 bytes), so give it an own
  `XDG_RUNTIME_DIR` like `/run/user/<uid>/vntest`, not one in a deep temp folder.
- `make check` runs the tests, also of the settings pages (in boa, a JavaScript engine in Rust, so
  no Node is needed).

## Releases

1. Read `git log <last tag>..HEAD` and pick the version: only fixes → patch, something new → minor,
   something that breaks or needs the user to act → major.
2. Bump `VERSION`, run `make manifest` and add the entry at the top of `CHANGELOG.md`, in one commit
   (`chore: 1.4.0`).
3. Push, then push the tag: `git tag v1.4.0 && git push origin main v1.4.0`.
4. `make package` (both CPUs in one file), then `gh release create v1.4.0` with the entry from `CHANGELOG.md` and a short
   install part as notes, and only the plugin file, named `Voicenux-v1.4.0.streamDeckPlugin`. Never
   attach a `.zip` as well: the plugin store of OpenDeck then asks which file to download.

A minor or major release starts with a welcome sentence and has a `### Highlights` list. A patch
release is just a sentence or two. Write for users: what they notice, not how the code does it.

## README

- The virtual deck is a test tool. The README does not mention it.
- Pictures use full links (`https://raw.githubusercontent.com/voicenux/voicenux-plugin/main/...`).
  The OpenDeck plugin store (marketplace.tacto.live) shows the README on its own site, where
  relative paths do not load.
- New UI (a key, a settings page) gets a screenshot in the README, next to the
  text about it. A real screenshot of it running, in English.
- When a screen changes, take its screenshot again. The README never shows an old one.
