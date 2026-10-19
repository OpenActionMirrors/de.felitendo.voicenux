// End-to-end test: presses keys on the virtual deck (through OpenDeck) and checks Voicenux.
// Needs: Voicenux with "Let other apps control Voicenux" on, OpenDeck with this plugin and the
// virtual deck from FeloDeck/obs-tools, and the profile from layout.json.
//
//   node test/e2e/run.mjs [test name filter]
const DECK = process.env.VIRTUAL_DECK_URL || "http://127.0.0.1:57190";
const API = process.env.VOICENUX_URL || "http://127.0.0.1:41967/api";
const filter = process.argv[2] ?? "";

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const post = (url) => fetch(url, { method: "POST" });
const press = (key, ms = 80) => post(`${DECK}/api/key/${key}/press?ms=${ms}`);
const rotate = (dial, ticks) => post(`${DECK}/api/dial/${dial}/rotate?ticks=${ticks}`);
const pressDial = (dial) => post(`${DECK}/api/dial/${dial}/press`);
const keyImage = async (key) => (await (await fetch(`${DECK}/api/state`)).json()).keys[key];
const vn = async () => (await (await fetch(`${API}/state`)).json()).result;
const voices = async () => (await (await fetch(`${API}/voices`)).json()).result;
const act = async (action, arg) => {
	const answer = await (await post(`${API}/${action}${arg === undefined ? "" : `/${encodeURIComponent(arg)}`}`)).json();
	if (!answer.ok) throw new Error(`${action}: ${answer.error}`);
	return answer.result;
};

async function waitFor(what, fn, timeout = 4000) {
	const until = Date.now() + timeout;
	let last;
	while (Date.now() < until) {
		last = await fn();
		if (last) return last;
		await sleep(80);
	}
	throw new Error(`timeout waiting for ${what} (last: ${JSON.stringify(last)})`);
}

const results = [];
async function test(name, fn) {
	if (filter && !name.includes(filter)) return;
	const start = Date.now();
	try {
		await fn();
		results.push({ name, ok: true });
		console.log(`ok   ${name} (${Date.now() - start} ms)`);
	} catch (error) {
		results.push({ name, ok: false, error: error.message });
		console.log(`FAIL ${name}: ${error.message}`);
	}
}

function assert(condition, message) {
	if (!condition) throw new Error(message);
}

// ---------- tests ----------

// A known start: Robot, everything as in layout.json expects it.
await act("voice", "robot");
await act("mute", "off");
await act("stop-sounds");
await act("sound-volume", 80);
await sleep(600);

await test("a voice key uses its voice and gets the frame", async () => {
	const before = await keyImage(1);
	await press(1);
	await waitFor("voice = deep", async () => (await vn()).voice.id === "deep");
	await waitFor("key 1 shows the frame", async () => (await keyImage(1)) !== before);
	await waitFor("key 0 loses its frame", async () => (await keyImage(0)) !== before);
});

await test("pressed again, the toggle voice key goes back to your own voice", async () => {
	await press(1);
	await waitFor("voice changer off", async () => (await vn()).voice_changer === false);
	await press(1);
	await waitFor("voice changer on with deep", async () => {
		const s = await vn();
		return s.voice_changer && s.voice.id === "deep";
	});
});

await test("push to change voice: changed only while the key is held", async () => {
	await act("voice-changer", "off");
	await sleep(300);
	const off = await keyImage(5);
	await post(`${DECK}/api/key/5/down`);
	await waitFor("voice changer on", async () => (await vn()).voice_changer === true);
	await waitFor("key 5 turns white", async () => (await keyImage(5)) !== off);
	await post(`${DECK}/api/key/5/up`);
	await waitFor("voice changer off", async () => (await vn()).voice_changer === false);
	await act("voice-changer", "on");
});

await test("a voice used while held comes back when you let go", async () => {
	await act("voice", "robot");
	// Voicenux tells the plugin about changes made elsewhere a little later.
	await sleep(300);
	await post(`${DECK}/api/key/2/down`);
	await waitFor("voice = chipmunk", async () => (await vn()).voice.id === "chipmunk");
	await sleep(300);
	await post(`${DECK}/api/key/2/up`);
	await waitFor("voice = robot", async () => (await vn()).voice.id === "robot");
	await act("voice-changer", "off");
	await sleep(300);
	await press(2, 300);
	await waitFor("voice changer off again", async () => {
		const s = await vn();
		return s.voice.id === "robot" && !s.voice_changer;
	});
	await act("voice-changer", "on");
});

await test("own voice key turns the voice off and is gray then", async () => {
	await act("voice", "robot");
	await sleep(300);
	const on = await keyImage(8);
	await press(8);
	await waitFor("voice changer off", async () => (await vn()).voice_changer === false);
	await waitFor("key 8 turns gray", async () => (await keyImage(8)) !== on);
	await act("voice-changer", "on");
	await waitFor("key 8 white again", async () => (await keyImage(8)) === on);
});

await test("mute key mutes and unmutes", async () => {
	await press(6);
	await waitFor("muted", async () => (await vn()).mute === true);
	await press(6);
	await waitFor("unmuted", async () => (await vn()).mute === false);
});

await test("switch keys flip hear myself and hear sounds", async () => {
	for (const [key, field] of [
		[7, "hear_myself"],
		[9, "hear_sounds"],
	]) {
		const before = (await vn())[field];
		await press(key);
		await waitFor(`${field} flips`, async () => (await vn())[field] === !before);
		await press(key);
		await waitFor(`${field} back`, async () => (await vn())[field] === before);
	}
});

await test("next voice key uses the next voice", async () => {
	const list = await voices();
	const favorites = list.filter((v) => v.favorite);
	const ring = favorites.length ? favorites : list;
	const now = (await vn()).voice.id;
	const expected = ring[(ring.findIndex((v) => v.id === now) + 1) % ring.length].id;
	await press(11);
	await waitFor(`voice = ${expected}`, async () => (await vn()).voice.id === expected);
});

await test("random voice key picks another voice", async () => {
	const start = (await vn()).voice.id;
	for (let i = 0; i < 4; i++) {
		await press(10);
		await sleep(300);
		if ((await vn()).voice.id !== start) return;
	}
	throw new Error("the voice stayed the same four times");
});

await test("a sound key plays its sound and gets the frame", async () => {
	const before = await keyImage(3);
	await press(3);
	await waitFor("bruh plays", async () => (await vn()).playing.includes("bruh"));
	await waitFor("key 3 shows the frame", async () => (await keyImage(3)) !== before);
	await waitFor("key 3 back without frame", async () => (await keyImage(3)) === before, 8000);
});

await test("a toggle sound key stops its sound", async () => {
	await press(4);
	await waitFor("sad violin plays", async () => (await vn()).playing.includes("sad-violin"));
	await sleep(300);
	await press(4);
	await waitFor("sad violin stops", async () => !(await vn()).playing.includes("sad-violin"));
});

await test("stop sounds key stops everything and is white while sounds play", async () => {
	// The sound of the test before may still be going out.
	await waitFor("nothing plays", async () => (await vn()).playing.length === 0);
	await sleep(400);
	const idle = await keyImage(12);
	await act("sound", "sad-violin");
	await waitFor("key 12 turns white", async () => (await keyImage(12)) !== idle);
	await press(12);
	await waitFor("nothing plays", async () => (await vn()).playing.length === 0);
	await waitFor("key 12 gray again", async () => (await keyImage(12)) === idle);
});

await test("soundboard volume key turns it down", async () => {
	await press(13);
	await waitFor("volume 60", async () => (await vn()).sound_volume === 60);
	await act("sound-volume", 80);
});

await test("a key without a voice does nothing", async () => {
	const before = await vn();
	await press(14);
	await sleep(400);
	assert(JSON.stringify(await vn()) === JSON.stringify(before), "the state changed");
});

await test("voice dial picks on turn and uses on press", async () => {
	await act("voice", "robot");
	await sleep(300);
	const list = await voices();
	const expected = list[(list.findIndex((v) => v.id === "robot") + 2) % list.length].id;
	await rotate(0, 2);
	await sleep(400);
	assert((await vn()).voice.id === "robot", "turning changed the voice already");
	await pressDial(0);
	await waitFor(`voice = ${expected}`, async () => (await vn()).voice.id === expected);
	await pressDial(0);
	await waitFor("voice changer off", async () => (await vn()).voice_changer === false);
	await act("voice-changer", "on");
});

await test("soundboard dial turns the volume and stops sounds", async () => {
	await rotate(1, 1);
	await waitFor("volume 90", async () => (await vn()).sound_volume === 90);
	await rotate(1, -3);
	await waitFor("volume 60", async () => (await vn()).sound_volume === 60);
	await act("sound", "sad-violin");
	await waitFor("plays", async () => (await vn()).playing.length > 0);
	await pressDial(1);
	await waitFor("stopped", async () => (await vn()).playing.length === 0);
	await act("sound-volume", 80);
});

await act("voice", "robot");
const failed = results.filter((r) => !r.ok);
console.log(`\n${results.length - failed.length}/${results.length} passed`);
process.exit(failed.length ? 1 : 0);
