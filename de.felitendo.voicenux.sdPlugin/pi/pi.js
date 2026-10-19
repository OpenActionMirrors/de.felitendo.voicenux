// Settings page for every Voicenux action. The fields come from schemas.js.
import { setLanguage, t, translateMessage } from "../plugin/lib/i18n.js";
import { CONTROLS, LOOK, SCHEMAS } from "./schemas.js";

const API = "http://127.0.0.1:41967";

let ws = null;
let context = "";
let action = "";
let schema = { fields: [] };
let settings = {};
let conn = null; // last "state" message from the plugin
let requestCounter = 0;
const pending = new Map();
const ui = {
	// Panels of the fields (see section in schemas.js) the user opened or closed.
	open: {},
	openMsg: null,
};

// What the pictures of the key need: the lists, the voice in use, and where pictures come from.
const page = {
	get state() {
		return conn?.voicenux ?? null;
	},
	list(name) {
		return conn?.lists?.[name] ?? [];
	},
	find(list, value) {
		return (conn?.lists?.[list] ?? []).find((item) => item.value === value) ?? null;
	},
	currentVoice() {
		const voice = conn?.voicenux?.voice;
		return voice ? { value: voice.id, label: voice.name, icon: voice.icon } : null;
	},
	pictureUrl(icon) {
		return icon ? `${API}/api/icon/${encodeURIComponent(icon)}` : null;
	},
};

// ---------- connection to OpenDeck ----------

window.connectElgatoStreamDeckSocket = (port, uuid, registerEvent, info, actionInfo) => {
	context = uuid;
	document.documentElement.lang = setLanguage(JSON.parse(info || "{}").application?.language);
	const parsed = JSON.parse(actionInfo || "{}");
	action = parsed.action;
	schema = SCHEMAS[action] ?? { fields: [] };
	settings = { ...(parsed.payload?.settings ?? {}) };
	ws = new WebSocket(`ws://127.0.0.1:${port}`);
	ws.addEventListener("open", () => {
		ws.send(JSON.stringify({ event: registerEvent, uuid }));
		send({ type: "hello" });
	});
	ws.addEventListener("message", (event) => onMessage(JSON.parse(event.data)));
	render();
};

function send(payload) {
	if (ws?.readyState !== WebSocket.OPEN) return;
	ws.send(JSON.stringify({ event: "sendToPlugin", action, context, payload }));
}

function request(payload) {
	const requestId = `r${++requestCounter}`;
	return new Promise((resolve) => {
		pending.set(requestId, resolve);
		send({ ...payload, requestId });
		setTimeout(() => {
			if (pending.delete(requestId)) resolve({ type: "timeout" });
		}, 15000);
	});
}

let saveTimer = null;
function save(now = false) {
	clearTimeout(saveTimer);
	const write = () => ws?.readyState === WebSocket.OPEN && ws.send(JSON.stringify({ event: "setSettings", context, payload: settings }));
	if (now) write();
	else saveTimer = setTimeout(write, 250);
}

function onMessage(message) {
	if (message.event === "didReceiveSettings") {
		settings = { ...settings, ...(message.payload?.settings ?? {}) };
		if (!isTyping()) render();
		return;
	}
	if (message.event !== "sendToPropertyInspector") return;
	const payload = message.payload ?? {};
	if (payload.requestId && pending.has(payload.requestId)) {
		pending.get(payload.requestId)(payload);
		pending.delete(payload.requestId);
	}
	if (payload.type === "state") {
		const first = !conn;
		const was = conn?.connection?.status;
		conn = payload;
		// The plugin knows the defaults, so its copy wins on the first load.
		if (payload.settings && first) settings = { ...settings, ...payload.settings };
		// The help is done once the connection works.
		if (conn.connection.status === "connected" && was !== "connected") ui.openMsg = null;
		if (!isTyping()) render();
		else renderStatus();
	}
}

function isTyping() {
	const el = document.activeElement;
	return !!el && (el.tagName === "TEXTAREA" || (el.tagName === "INPUT" && ["text", "number", "password"].includes(el.type)));
}

// ---------- tiny DOM helper ----------

function h(tag, attrs = {}, ...children) {
	const el = document.createElement(tag);
	for (const [key, value] of Object.entries(attrs ?? {})) {
		if (value === undefined || value === null || value === false) continue;
		if (key === "class") el.className = value;
		else if (key === "style") el.style.cssText = value;
		else if (key.startsWith("on")) el.addEventListener(key.slice(2).toLowerCase(), value);
		else if (key in el) el[key] = value;
		else el.setAttribute(key, value === true ? "" : value);
	}
	for (const child of children.flat(Infinity)) {
		if (child === null || child === undefined || child === false) continue;
		el.append(child instanceof Node ? child : document.createTextNode(String(child)));
	}
	return el;
}

// **bold** and _italic_ inside a (translated) sentence.
function rich(text) {
	return text.split(/(\*\*[^*]+\*\*|_[^_]+_)/).map((part) => {
		if (part.startsWith("**")) return h("b", {}, part.slice(2, -2));
		if (part.length > 2 && part.startsWith("_")) return h("i", {}, part.slice(1, -1));
		return part;
	});
}

function row(label, control, { hint, top = false } = {}) {
	return h("div", { class: `row${top ? " top" : ""}` }, h("div", { class: "name" }, label ?? ""), h("div", { class: "control" }, control), hint ? h("div", { class: "hint-line" }, hint) : null);
}

function msg(m) {
	return m ? h("div", { class: `msg ${m.kind ?? ""}` }, m.text) : null;
}

// ---------- own controls ----------
// WebKitGTK (OpenDeck) draws native drop-downs and checkboxes in the GTK style, which does not fit.
// These look like the rest of the page and work with mouse and keyboard.

const CHECK = `<svg width="13" height="10" viewBox="0 0 13 10"><path d="M1.6 5.3 4.8 8.4 11.4 1.6" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

function checkMark(on) {
	const el = h("span", { class: "dd-check" });
	if (on) el.innerHTML = CHECK;
	return el;
}

// The picture of a voice or sound, small, in front of its name.
function picture(icon) {
	const url = page.pictureUrl(icon);
	return url ? h("img", { class: "dd-pic", src: url, alt: "", loading: "lazy" }) : null;
}

const CHEVRON = `<svg width="10" height="6" viewBox="0 0 10 6"><path d="M1 1l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>`;
let openDropdown = null;
let renderPending = false;

function closeDropdown() {
	if (!openDropdown) return;
	const current = openDropdown;
	openDropdown = null;
	current.close();
	if (renderPending) {
		renderPending = false;
		render();
	}
}

document.addEventListener("mousedown", (e) => {
	if (openDropdown && !openDropdown.root.contains(e.target)) closeDropdown();
});
window.addEventListener("blur", () => closeDropdown());

// options: [{ value, label, icon, missing }]
function dropdown({ value, options, placeholder = t("Choose…"), onChange, focusKey }) {
	const current = options.find((o) => o.value === value);
	const root = h("div", { class: "dropdown" });
	const button = h(
		"button",
		{
			type: "button",
			class: `dd-button${current?.missing ? " missing" : ""}`,
			"data-focus-key": focusKey,
			onclick: () => (openDropdown?.root === root ? closeDropdown() : open()),
			onkeydown: (e) => {
				if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
					e.preventDefault();
					open();
				}
			},
		},
		current?.icon ? picture(current.icon) : null,
		h("span", { class: `dd-label${current ? "" : " placeholder"}` }, current ? current.label : placeholder),
	);
	button.insertAdjacentHTML("beforeend", `<span class="dd-chevron">${CHEVRON}</span>`);
	root.append(button);

	function open() {
		closeDropdown();
		root.classList.add("open");
		const pop = h("div", { class: "dd-pop", role: "listbox" });
		const search = options.length > 10 ? h("input", { type: "text", class: "dd-search", placeholder: t("Search…") }) : null;
		const list = h("div", { class: "dd-list" });
		let shown = options;
		let active = Math.max(0, options.findIndex((o) => o.value === value));
		const pick = (option) => {
			closeDropdown();
			if (option && option.value !== value) onChange(option.value);
			const again = focusKey && document.querySelector(`[data-focus-key="${CSS.escape(focusKey)}"]`);
			(again || button).focus();
		};
		const mark = () => {
			[...list.children].forEach((el, i) => el.classList.toggle("active", i === active));
			list.children[active]?.scrollIntoView({ block: "nearest" });
		};
		const draw = () => {
			list.replaceChildren(
				...shown.map((o, i) =>
					h(
						"div",
						{
							class: `dd-item${o.value === value ? " selected" : ""}${o.missing ? " missing" : ""}`,
							role: "option",
							onmousedown: (e) => (e.preventDefault(), pick(o)),
							onmouseenter: () => ((active = i), mark()),
						},
						checkMark(o.value === value),
						o.icon ? picture(o.icon) : null,
						h("span", { class: "dd-text" }, o.label),
						o.favorite ? h("span", { class: "dd-star", title: t("Favorite") }, "★") : null,
					),
				),
			);
			if (!shown.length) list.append(h("div", { class: "dd-empty" }, t("Nothing found")));
			mark();
		};
		const onKey = (e) => {
			const handled = () => (e.preventDefault(), e.stopPropagation());
			if (e.key === "ArrowDown" || e.key === "ArrowUp") {
				handled();
				active = Math.min(shown.length - 1, Math.max(0, active + (e.key === "ArrowDown" ? 1 : -1)));
				mark();
			} else if (e.key === "Enter" || (e.key === " " && document.activeElement !== search)) {
				// Stop it here, or the button would open the list again.
				handled();
				pick(shown[active]);
			} else if (e.key === "Escape" || e.key === "Tab") {
				handled();
				closeDropdown();
				button.focus();
			} else if (search && e.key.length === 1 && document.activeElement !== search) {
				search.focus();
			}
		};
		if (search) {
			search.addEventListener("input", () => {
				const q = search.value.trim().toLowerCase();
				shown = q ? options.filter((o) => String(o.label).toLowerCase().includes(q)) : options;
				active = 0;
				draw();
			});
			pop.append(search);
		}
		pop.append(list);
		root.append(pop);
		draw();
		// Open upwards when there is more room above, and never taller than the page.
		const rect = button.getBoundingClientRect();
		const below = window.innerHeight - rect.bottom - 8;
		const above = rect.top - 8;
		const up = below < 160 && above > below;
		pop.classList.toggle("up", up);
		list.style.maxHeight = `${Math.max(90, Math.min(240, (up ? above : below) - (search ? 40 : 8)))}px`;
		mark();
		document.addEventListener("keydown", onKey, true);
		if (search) search.focus();
		openDropdown = {
			root,
			close: () => {
				document.removeEventListener("keydown", onKey, true);
				pop.remove();
				root.classList.remove("open");
			},
		};
	}
	return root;
}

function checkbox(checked, text, onChange) {
	return h(
		"label",
		{ class: "check" },
		h("input", { type: "checkbox", checked: !!checked, onchange: (e) => onChange(e.target.checked) }),
		h("span", { class: "check-box" }),
		text ? h("span", { class: "check-text" }, text) : null,
	);
}

// ---------- rendering ----------

function render() {
	if (openDropdown) {
		renderPending = true;
		return;
	}
	const app = document.getElementById("app");
	const focus = document.activeElement?.dataset?.focusKey;
	const parts = [renderStatusCard(), problem() ? renderGuide() : null, renderKeyError(), schema.help ? h("p", { class: "note" }, t(schema.help)) : null, ...renderFields()];
	app.replaceChildren(...parts.flat(Infinity).filter(Boolean));
	// WebKitGTK in OpenDeck sometimes paints only parts of the page after a big change and keeps old
	// pixels elsewhere. A tiny change of the opacity makes it paint the whole page again.
	app.style.opacity = app.style.opacity === "0.999" ? "" : "0.999";
	if (focus) document.querySelector(`[data-focus-key="${CSS.escape(focus)}"]`)?.focus();
}

function renderStatus() {
	const old = document.querySelector(".status");
	if (old) old.replaceWith(renderStatusCard());
}

function statusText() {
	if (!conn) return { kind: "busy", title: t("Checking connection to Voicenux…") };
	const c = conn.connection;
	switch (c.status) {
		case "connected":
			// All good: a quiet line. A newer API than the plugin knows shows as a hint.
			return { kind: "ok", quiet: true, title: t("Connected to Voicenux {version}", { version: c.version ?? "" }).trim(), hint: c.error ? translateMessage(c.error) : "" };
		case "disabled":
			return { kind: "warn", title: t("Voicenux does not let other apps in"), hint: t("Turn it on in the settings of Voicenux.") };
		case "offline":
			return { kind: "bad", title: t("Voicenux is not running"), hint: c.installed ? t("Start Voicenux, the keys connect by themselves.") : t("Install Voicenux first.") };
		case "error":
			return { kind: "bad", title: t("Voicenux error"), hint: c.error ? translateMessage(c.error) : "" };
		default:
			return { kind: "warn busy", title: t("Connecting to Voicenux…") };
	}
}

function problem() {
	return ["disabled", "offline", "error"].includes(conn?.connection?.status);
}

function renderStatusCard() {
	const s = statusText();
	if (s.quiet) {
		return h("section", { class: `status ${s.kind} quiet`, "aria-live": "polite" }, h("span", { class: "status-dot" }), h("span", { class: "status-title" }, s.title), h("span", { class: "status-hint" }, s.hint));
	}
	return h(
		"section",
		{ class: `status ${s.kind}`, "aria-live": "polite" },
		h("span", { class: "status-dot" }),
		h("div", {}, h("div", { class: "status-title" }, s.title), s.hint ? h("div", { class: "status-hint" }, s.hint) : null),
	);
}

// Only shown while Voicenux cannot be reached.
function renderGuide() {
	const c = conn.connection;
	if (!c.installed) {
		return h(
			"section",
			{ class: "connection" },
			h("div", { class: "box-title" }, t("Voicenux is not installed")),
			h("p", { class: "note" }, t("This plugin controls the Voicenux voice changer. Install it, then start it once.")),
			h("div", { class: "buttons" }, h("button", { class: "btn primary", onclick: () => send({ type: "openUrl", url: "https://github.com/voicenux/voicenux#install" }) }, t("How to install"))),
		);
	}
	return h(
		"section",
		{ class: "connection" },
		h("div", { class: "box-title" }, t("How to connect Voicenux")),
		h(
			"ol",
			{ class: "steps" },
			h("li", {}, rich(t("Click **Open Voicenux settings** below. It starts Voicenux if it is not running."))),
			h("li", {}, rich(t("Under **Other apps**, turn on **Let other apps control Voicenux**."))),
		),
		h("p", { class: "note" }, t("The status at the top turns green as soon as the connection works.")),
		h("div", { class: "buttons" }, h("button", { class: "btn primary", onclick: openVoicenux }, t("Open Voicenux settings")), h("button", { class: "btn", onclick: () => send({ type: "retry" }) }, t("Try again"))),
		msg(ui.openMsg),
	);
}

async function openVoicenux() {
	const reply = await request({ type: "openVoicenux", page: "settings" });
	ui.openMsg = reply.error ? { kind: "bad", text: translateMessage(reply.error) } : { kind: "ok", text: t("Voicenux opens on your desktop…") };
	render();
}

// Why the key showed its alert last time. It goes away after the next press that works.
function renderKeyError() {
	if (!conn?.keyError) return null;
	return h("div", { class: "msg bad key-error" }, t("Last error: {error}", { error: translateMessage(conn.keyError) }));
}

function visible(field) {
	return !field.showIf || field.showIf(settings);
}

// What the key is about at the top, then what it does, how it looks, and the closed sections.
function renderFields() {
	const top = [];
	const look = [];
	const panels = new Map();
	for (const field of schema.fields) {
		if (!visible(field)) continue;
		const el = renderField(field);
		if (!el) continue;
		if (!field.section) top.push(el);
		else if (field.section === LOOK) look.push(el);
		else {
			if (!panels.has(field.section)) panels.set(field.section, []);
			panels.get(field.section).push(el);
		}
	}
	const out = [...top];
	if (panels.has(CONTROLS)) out.push(section(CONTROLS, true, panels.get(CONTROLS)));
	if (look.length) out.push(section(schema.lookTitle ?? "On the key", true, look));
	for (const [title, fields] of panels) if (title !== CONTROLS) out.push(section(title, false, fields));
	return out;
}

// A small heading that folds the fields below it. The fields keep the grid of the rows above.
function section(title, open, fields) {
	const id = `${action}:${title}`;
	return h("details", { class: "section", open: ui.open[id] ?? open, ontoggle: (e) => (ui.open[id] = e.target.open) }, h("summary", { class: "heading" }, t(title)), ...fields);
}

// Several settings at once, from one choice.
function setValues(values) {
	Object.assign(settings, values);
	save(true);
	render();
}

// Pictures on one page share the ids of their gradients and clips. Give each picture its own.
function ownIds(svg, suffix) {
	return svg.replace(/\bid="([^"]+)"/g, `id="$1-${suffix}"`).replace(/url\(#([^)]+)\)/g, `url(#$1-${suffix})`);
}

function setValue(key, value, rerender = false) {
	settings[key] = value;
	save(rerender);
	if (rerender) render();
}

function numberInput(field) {
	const { key } = field;
	const input = h("input", {
		type: "number",
		value: settings[key] ?? "",
		min: field.min,
		max: field.max,
		step: field.step,
		"data-focus-key": key,
		oninput: (e) => setValue(key, e.target.value === "" ? settings[key] : Number(e.target.value)),
		onchange: () => (save(true), render()),
	});
	// − and + instead of the tiny native arrows.
	const bump = (direction) => {
		const step = Number(field.step) > 0 ? Number(field.step) : 1;
		const from = input.value === "" ? Number(field.min ?? 0) || 0 : Number(input.value);
		let next = Math.round((from + direction * step) / step) * step;
		if (field.min !== undefined) next = Math.max(Number(field.min), next);
		if (field.max !== undefined) next = Math.min(Number(field.max), next);
		input.value = next;
		setValue(key, next);
		save(true);
	};
	return h(
		"div",
		{ class: "stepper" },
		h("button", { type: "button", class: "step-btn", tabindex: -1, onclick: () => bump(-1), "aria-label": t("Less") }, "−"),
		input,
		h("button", { type: "button", class: "step-btn", tabindex: -1, onclick: () => bump(1), "aria-label": t("More") }, "+"),
	);
}

function selectInput(field) {
	const fixed = field.options;
	const value = settings[field.key] ?? "";
	// Fixed options are our texts, lists hold names from Voicenux (already in its language).
	const options = fixed ? fixed.map(([v, label]) => ({ value: v, label: t(label) })) : [...(conn?.lists?.[field.list] ?? [])];
	if (value && !options.some((o) => o.value === value)) {
		const known = fixed || conn?.connection?.status === "connected";
		options.push({ value, label: known ? t("{value} (not found)", { value }) : value, missing: known });
	}
	const offline = conn?.connection?.status !== "connected";
	const placeholder = t(field.list && offline && !options.length ? "Voicenux not connected" : "Choose…");
	return dropdown({ value, options, placeholder, focusKey: field.key, onChange: (v) => setValue(field.key, v, true) });
}

function renderField(field) {
	const { key } = field;
	const label = field.label ? t(field.label) : field.label;
	const hint = field.hint ? t(field.hint) : field.hint;
	switch (field.type) {
		case "note":
			return h("p", { class: "note" }, t(field.text));
		case "select":
			return row(label, selectInput(field), { hint });
		case "number":
			return row(label, [numberInput(field), field.unit ? h("span", { class: "unit" }, t(field.unit)) : null], { hint });
		case "checkbox":
			return row(label, checkbox(!!settings[key], field.text ? t(field.text) : "", (on) => setValue(key, on, true)));
		case "segmented":
			return row(
				label,
				h(
					"div",
					{ class: "segmented" },
					field.options.map(([v, text]) => h("button", { class: (settings[key] ?? field.options[0][0]) === v ? "on" : "", onclick: () => setValue(key, v, true) }, t(text))),
				),
			);
		case "tiles": {
			const current = field.value ? field.value(settings) : (settings[key] ?? field.options[0][0]);
			return row(
				label,
				h(
					"div",
					{ class: "tiles", role: "radiogroup" },
					field.options.map(([v, text], i) => {
						const on = current === v;
						const pick = () => (field.apply ? setValues(field.apply(v)) : setValue(key, v, true));
						return h(
							"button",
							{ class: `tile${on ? " on" : ""}`, role: "radio", "aria-checked": String(on), onclick: pick },
							h("span", { class: "tile-key", innerHTML: ownIds(field.preview(v, settings, page), `${key}-${i}`) }),
							h("span", { class: "tile-name" }, t(text)),
						);
					}),
				),
				{ top: true },
			);
		}
	}
	return null;
}
