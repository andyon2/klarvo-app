/**
 * Story 13-1 — desktop PROXY gate for the four debug-test-provider rows.
 *
 * WHY A PUPPETEER HARNESS: this repo has no JS test runner (no vitest, no jest)
 * and adding one is forbidden (project-context: no `npm install` from a story;
 * the Windows build runs `npm ci`). The documented desktop MACHINE gate for a
 * React surface is puppeteer against `npm run preview` (port 1422) in real
 * Chromium. This is that.
 *
 * WHAT IT ASSERTS, in ONE run:
 *   (a) VISIBILITY — all FOUR rows (LLM Provider, STT Provider, Debug LLM
 *       Scenario, Debug STT Scenario) are ABSENT from Settings → Advanced →
 *       System with Expert mode off, and PRESENT with it on.
 *   (b) PICKER — `debug` is not an option in the NORMAL provider picker
 *       (Settings → Recording & Audio), scanned with Expert mode BOTH OFF AND
 *       ON; and Settings → AI & Providers carries no provider picker at all —
 *       asserted POSITIVELY (the page is proven to have rendered, and every
 *       control on it is proven to be a non-provider control), never as a
 *       zero-element scan reported as a pass.
 *   (c) OPTION LISTS — the rendered options of all four rows are compared
 *       element-wise against `test-fixtures/debug-provider-scenario-vectors.json`,
 *       the same fixture the Rust and JVM tests read. The React lists are thus
 *       pinned to the Rust allowlists and canned-wire tables, not to nothing.
 *   (d) CONTROL-STATES CONTRACT — for EACH OF THE FOUR rows and every state the
 *       harness can drive (idle, hover, focus, press, open, the open listbox,
 *       its selected option and its keyboard-focused option),
 *       `getComputedStyle` equals that of the NAMED REFERENCE INSTANCE — the
 *       "Dictation language" `KSelect` in Settings → Language — on
 *       background-color, border-color, border-radius, padding, font-size,
 *       color and box-shadow. A mismatch on any property fails the gate.
 *
 * The run reports ONE check count, derived from the result records.
 *
 * INVERSION (`--invert`): THREE deliberate red runs, because a guard that has
 * never gone red proves nothing:
 *   1. states — the SAME equality assertion, with the SAME measured states,
 *      pointed at a deliberately different control (the raw `<select>` of the
 *      Log Level row).
 *   2. visibility — the SAME row-presence predicate asserted with Expert mode
 *      OFF, where the rows do not exist.
 *   3. picker — the SAME `debug`-leak scan run against a page that DOES carry a
 *      `debug` option (Advanced → System with Expert mode on), proving the scan
 *      can detect the leak it claims to rule out.
 * An inversion assertion PASSES BY GOING RED, so the inversion run is scored
 * separately: it exits 0 when every inversion group produced at least one red
 * AND no ordinary check failed. A green inversion assertion is the failure.
 *
 * WHAT IT DOES NOT EXERCISE (stated explicitly per project-context):
 *   - persistence: preview-mode writers are no-ops, so "Save" is not clicked and
 *     nothing reaches config.json. The round-trip is the Rust suite's claim
 *     (config::tests::spec_debug_provider_and_scenarios_survive_a_real_config_file_round_trip);
 *   - anything Rust: no Tauri, no provider resolution, no canned wire, no
 *     fallback ladder;
 *   - the `disabled` state of KSelect — these four rows never pass `disabled`,
 *     the Expert-mode gate removes them instead;
 *   - pixels, font rasterisation, the Windows text-scale drift — Andi's
 *     real-screen gate;
 *   - Android (the same bundle renders there, but nothing here runs on it).
 *
 * Trap #1 (documented): preview boots into Onboarding → click
 * "Setup überspringen" first, or every selector times out.
 * Trap #2 (documented): the evidence dir is inside the repo and `npm run
 * preview` is the Vite DEV server, so writing artifacts mid-run triggers an HMR
 * reload. Everything is staged in a temp dir and copied in after the browser
 * closes.
 * Trap #3 (documented): `KSelect`'s Escape handler does not stopPropagation, so
 * Escape closes the whole Settings panel — a listbox is closed by clicking its
 * trigger again.
 * Trap #4 (documented): `className.includes("bg-klarvo-surface-2")` also matches
 * `hover:bg-klarvo-surface-2`, which every enabled option carries — class tokens
 * are matched with `classList.contains`.
 *
 * Trap #5 (this run): the preview mock has `expertMode: false` and its writers
 * are no-ops, so the "picker with Expert mode ON" scan cannot be reached by
 * clicking. It is reached with the project's documented technique — a THROWAWAY
 * edit to `src/tauri-commands.ts`, measured, then restored (project-context,
 * desktop machine gate). The edit is reverted in a `finally` block and the run
 * prints the restore result; verify `git status` is clean afterwards.
 *
 * Run: node _bmad-output/implementation-artifacts/gate4-evidence/13-1/debug-rows-smoke.mjs [--invert]
 */
import puppeteer from "puppeteer";
import { writeFileSync, readFileSync, mkdtempSync, copyFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "..", "..", "..", "..");
const BASE = "http://localhost:1422";
const INVERT = process.argv.includes("--invert");
const STAGE = mkdtempSync(join(tmpdir(), "klarvo-13-1-"));
const stage = (n) => join(STAGE, n);

const MOCK_FILE = join(REPO, "src", "tauri-commands.ts");
const MOCK_ORIGINAL = readFileSync(MOCK_FILE, "utf8");
const MOCK_EXPERT_OFF = "  expertMode: false,\n  // Story 13-1: the benign scenario";
const MOCK_EXPERT_ON = "  expertMode: true,\n  // Story 13-1: the benign scenario";

const PROPS = [
  "backgroundColor",
  "borderTopColor",
  "borderRightColor",
  "borderBottomColor",
  "borderLeftColor",
  "borderTopLeftRadius",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "fontSize",
  "color",
  "boxShadow",
];

/** The four rows, in DOM order, each with the fixture entry that pins its options. */
const ROWS = [
  { label: "LLM Provider", index: 0 },
  { label: "STT Provider", index: 1 },
  { label: "Debug LLM Scenario", index: 2 },
  { label: "Debug STT Scenario", index: 3 },
];

const results = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const pass = (name, detail) => results.push({ ok: true, name, detail });
const fail = (name, detail) => results.push({ ok: false, name, detail });
const check = (c, name, detail) => (c ? pass(name, detail) : fail(name, detail));

// ---------------------------------------------------------------- fixture
// The option lists are derived from the SAME file the Rust and JVM tests read,
// so a rename on either side fails here too.
const FIXTURE = JSON.parse(
  readFileSync(join(REPO, "test-fixtures", "debug-provider-scenario-vectors.json"), "utf8"),
);
const vector = (id) => {
  const v = FIXTURE.find((x) => x.id === id);
  if (!v) throw new Error(`fixture has no vector with id=${id}`);
  return v;
};
const EXPECTED_OPTIONS = {
  "LLM Provider": vector("DEBUG-LLM-PROVIDER-OPTIONS-001").options,
  "STT Provider": vector("DEBUG-STT-PROVIDER-OPTIONS-001").options,
  "Debug LLM Scenario": FIXTURE.filter((v) => v.surface === "llm").map((v) => v.scenario),
  "Debug STT Scenario": FIXTURE.filter(
    (v) => v.surface === "stt" && v.wire?.kind !== "not_offered",
  ).map((v) => v.scenario),
};

// ------------------------------------------------------------- navigation
async function clickAria(page, label) {
  await page.waitForSelector(`[aria-label="${label}"]`, { timeout: 10000 });
  await page.click(`[aria-label="${label}"]`);
  await sleep(400);
}

async function clickByText(page, label) {
  const clicked = await page.evaluate((label) => {
    const nodes = [...document.querySelectorAll("button, a, [role=button]")];
    const m = nodes.filter((n) => (n.innerText || "").includes(label));
    if (!m.length) return false;
    m.sort((a, b) => (a.innerText || "").length - (b.innerText || "").length);
    m[0].click();
    return true;
  }, label);
  if (!clicked) throw new Error(`cannot find clickable element for "${label}"`);
  await sleep(350);
}

async function boot(page) {
  await page.goto(BASE, { waitUntil: "networkidle2", timeout: 30000 });
  const text = await page.evaluate(() => document.body.innerText);
  if (text.includes("Setup überspringen")) {
    await clickByText(page, "Setup überspringen");
    await sleep(1200);
    return "onboarding skipped";
  }
  return "app booted past onboarding";
}

async function openSettingsCategory(page, category) {
  await clickAria(page, "Toggle settings");
  await sleep(500);
  await clickByText(page, category);
  await sleep(600);
}

// ------------------------------------------------------------- inspection
/**
 * Opens every dropdown on the current page (one at a time, with real awaits — a
 * synchronous click inside page.evaluate never lets React render the portal) and
 * returns every option label found, plus every native <select> option.
 */
async function collectOptionLabels(page) {
  const out = [];
  const n = await page.evaluate(
    () => document.querySelectorAll('button[aria-haspopup="listbox"]').length,
  );
  for (let i = 0; i < n; i++) {
    out.push(...(await readOptions(page, i)));
  }
  const natives = await page.evaluate(() => {
    const o = [];
    for (const s of document.querySelectorAll("select")) {
      for (const opt of s.options) o.push(opt.value, opt.text);
    }
    return o;
  });
  out.push(...natives);
  return out;
}

/** Opens the nth KSelect, reads its option labels, closes it again. */
async function readOptions(page, i) {
  await page.evaluate((i) => {
    document.querySelectorAll('button[aria-haspopup="listbox"]')[i].click();
  }, i);
  await sleep(260);
  const labels = await page.evaluate(() =>
    [...document.querySelectorAll('ul[role="listbox"] li[role="option"]')].map((l) =>
      (l.innerText || "").trim(),
    ),
  );
  await page.evaluate((i) => {
    document.querySelectorAll('button[aria-haspopup="listbox"]')[i].click();
  }, i);
  await sleep(200);
  return labels;
}

/** All KSelect triggers currently in the tree, with their row label. */
async function listSelects(page) {
  return page.evaluate(() =>
    [...document.querySelectorAll('button[aria-haspopup="listbox"]')].map((b, i) => ({
      i,
      label: (b.closest("div")?.parentElement?.innerText || "").split("\n")[0] || "",
      text: (b.innerText || "").trim(),
    })),
  );
}

/** Reads the computed style of the nth KSelect trigger. */
async function triggerStyle(page, n) {
  return page.evaluate(
    (n, PROPS) => {
      const b = document.querySelectorAll('button[aria-haspopup="listbox"]')[n];
      if (!b) return null;
      const cs = getComputedStyle(b);
      return Object.fromEntries(PROPS.map((p) => [p, cs[p]]));
    },
    n,
    PROPS,
  );
}

/** Reads the computed style of the raw <select> in the Log Level row. */
async function rawSelectStyle(page) {
  return page.evaluate((PROPS) => {
    const s = document.querySelector("select");
    if (!s) return null;
    const cs = getComputedStyle(s);
    return Object.fromEntries(PROPS.map((p) => [p, cs[p]]));
  }, PROPS);
}

async function boxOf(page, n) {
  return page.evaluate((n) => {
    const b = document.querySelectorAll('button[aria-haspopup="listbox"]')[n];
    if (!b) return null;
    const r = b.getBoundingClientRect();
    return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
  }, n);
}

/**
 * Drives every reachable state of the nth KSelect and returns a style map per
 * state. `pressed` is observably the open state (the component has no distinct
 * pressed styling; a press opens the listbox).
 */
async function captureStates(page, n) {
  const out = {};

  await page.mouse.move(0, 0);
  await page.evaluate(() => document.activeElement?.blur());
  await sleep(120);
  out.idle = await triggerStyle(page, n);

  const box = await boxOf(page, n);
  await page.mouse.move(box.x, box.y);
  await sleep(150);
  out.hover = await triggerStyle(page, n);

  await page.mouse.move(0, 0);
  await page.evaluate((n) => {
    document.querySelectorAll('button[aria-haspopup="listbox"]')[n].focus();
  }, n);
  await sleep(150);
  out.focused = await triggerStyle(page, n);

  // press → opens the listbox
  await page.evaluate((n) => {
    document.querySelectorAll('button[aria-haspopup="listbox"]')[n].click();
  }, n);
  await sleep(250);
  out.open = await triggerStyle(page, n);
  out.openMeta = await page.evaluate((n) => {
    const b = document.querySelectorAll('button[aria-haspopup="listbox"]')[n];
    const svg = b.querySelector("svg");
    return {
      ariaExpanded: b.getAttribute("aria-expanded"),
      chevronRotated: (svg?.getAttribute("class") || "").includes("rotate-180"),
    };
  }, n);

  out.listbox = await page.evaluate((PROPS) => {
    const ul = document.querySelector('ul[role="listbox"]');
    if (!ul) return null;
    const cs = getComputedStyle(ul);
    return Object.fromEntries(PROPS.map((p) => [p, cs[p]]));
  }, PROPS);

  out.optionSelected = await page.evaluate((PROPS) => {
    const li = document.querySelector('ul[role="listbox"] li[aria-selected="true"]');
    if (!li) return null;
    const cs = getComputedStyle(li);
    return {
      ...Object.fromEntries(PROPS.map((p) => [p, cs[p]])),
      __ariaSelected: li.getAttribute("aria-selected"),
    };
  }, PROPS);

  // keyboard-focused option: ArrowDown moves the highlight off the selected one.
  await page.keyboard.press("ArrowDown");
  await sleep(180);
  // The keyboard highlight is the EXACT class token `bg-klarvo-surface-2`.
  // Matching it as a substring also hits `hover:bg-klarvo-surface-2`, which every
  // enabled option carries — that read option[0] and silently compared the
  // selected option on one control against a plain option on the other.
  out.optionKeyboardFocused = await page.evaluate((PROPS) => {
    const lis = [...document.querySelectorAll('ul[role="listbox"] li[role="option"]')];
    const li = lis.find((l) => l.classList.contains("bg-klarvo-surface-2"));
    if (!li) return null;
    const cs = getComputedStyle(li);
    return {
      ...Object.fromEntries(PROPS.map((p) => [p, cs[p]])),
      __ariaSelected: li.getAttribute("aria-selected"),
      __index: lis.indexOf(li),
    };
  }, PROPS);

  // Close by clicking the trigger again, NOT with Escape (trap #3).
  await page.evaluate((n) => {
    document.querySelectorAll('button[aria-haspopup="listbox"]')[n].click();
  }, n);
  await sleep(250);
  return out;
}

function diff(a, b) {
  if (!a || !b) return ["one side missing"];
  return PROPS.filter((p) => a[p] !== b[p]).map((p) => `${p}: ${a[p]} != ${b[p]}`);
}

const STATES = [
  "idle",
  "hover",
  "focused",
  "open",
  "listbox",
  "optionSelected",
  "optionKeyboardFocused",
];

/**
 * THE state assertion. Both the green run and inversion #1 call exactly this,
 * with exactly the same `measured`; only `reference` differs. That is what makes
 * the inversion "the same assertion pointed at a different control" rather than a
 * different assertion that happens to fail.
 */
function assertStatesEqual(measured, reference, rowLabel) {
  for (const state of STATES) {
    if (state === "optionKeyboardFocused") {
      check(
        measured[state]?.__ariaSelected === reference[state]?.__ariaSelected,
        `[${rowLabel}] keyboard-focused option compared like-for-like`,
        `new aria-selected=${measured[state]?.__ariaSelected} (idx ${measured[state]?.__index}), ` +
          `reference aria-selected=${reference[state]?.__ariaSelected} (idx ${reference[state]?.__index})`,
      );
    }
    const d = diff(measured[state], reference[state]);
    check(
      d.length === 0,
      `[${rowLabel}] state '${state}' equals the reference instance`,
      d.length ? d.join(" | ") : `all ${PROPS.length} properties equal`,
    );
  }
  check(
    measured.openMeta?.ariaExpanded === "true" && measured.openMeta?.chevronRotated === true,
    `[${rowLabel}] open state: aria-expanded=true + chevron rotated`,
    JSON.stringify(measured.openMeta),
  );
  // "pressed" has no distinct styling in the shipped component: a press opens the
  // listbox, so pressed IS the open state. Assert it explicitly, not silently.
  check(
    JSON.stringify(measured.open) === JSON.stringify(reference.open),
    `[${rowLabel}] state 'pressed' (= open) equals the reference instance`,
    "the shipped KSelect defines no distinct pressed styling",
  );
}

/** THE row-presence predicate. Green run and inversion #2 both call this. */
const rowsPresent = (bodyText) => ROWS.every((r) => bodyText.includes(r.label));

/** THE leak predicate. Green run and inversion #3 both call this. */
const leakedDebug = (labels) => labels.filter((o) => /(^|\b)debug(\b|$)/i.test(o));

// ------------------------------------------------------- throwaway mock edit
function setMockExpertMode(on) {
  const from = on ? MOCK_EXPERT_OFF : MOCK_EXPERT_ON;
  const to = on ? MOCK_EXPERT_ON : MOCK_EXPERT_OFF;
  const src = readFileSync(MOCK_FILE, "utf8");
  if (!src.includes(from)) throw new Error(`mock anchor not found while setting expertMode=${on}`);
  writeFileSync(MOCK_FILE, src.replace(from, to));
}
function restoreMock() {
  writeFileSync(MOCK_FILE, MOCK_ORIGINAL);
  return readFileSync(MOCK_FILE, "utf8") === MOCK_ORIGINAL;
}

// --------------------------------------------------------------------- run
const browser = await puppeteer.launch({
  headless: "new",
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 950 });

const pageErrors = [];
page.on("pageerror", (e) =>
  pageErrors.push(String(e.stack || e).split("\n").slice(0, 4).join(" | ")),
);

let reference = null;
const measured = {};

try {
  pass("boot", await boot(page));

  // ------------------------------------------------------------- REFERENCE
  await openSettingsCategory(page, "Language");
  const langSelects = await listSelects(page);
  check(
    langSelects.length >= 1,
    "reference instance found",
    `Settings → Language carries ${langSelects.length} KSelect(s); using #0 ("Dictation language")`,
  );
  if (!langSelects.length) throw new Error("reference KSelect not found — refusing a vacuous pass");
  reference = await captureStates(page, 0);
  writeFileSync(stage("reference-states.json"), JSON.stringify(reference, null, 2));
  await page.screenshot({ path: stage("ist-reference-language.png") });
  await clickAria(page, "Back to settings");
  await sleep(400);

  // ------------------------------------------- (b) PICKERS, Expert mode OFF
  await scanPickers("expert OFF");

  // --------------------------------------------------- (a) VISIBILITY GATE
  await clickByText(page, "Advanced");
  await sleep(600);
  await clickByText(page, "System");
  await sleep(600);
  await page.screenshot({ path: stage("ist-system-expert-off.png") });

  const offText = await page.evaluate(() => document.body.innerText);
  const offSelects = await listSelects(page);
  const missingOff = ROWS.filter((r) => !offText.includes(r.label)).map((r) => r.label);
  check(
    !rowsPresent(offText) && missingOff.length === ROWS.length && offSelects.length === 0,
    "all four rows absent with Expert mode OFF",
    `absent: ${JSON.stringify(missingOff)}; KSelect count on the System page = ${offSelects.length} (expected 0)`,
  );

  // The Expert-mode control is a bare role="switch" button whose own innerText is
  // empty (the label lives in a sibling span), so it cannot be clicked by text.
  // It is the only switch on the System page.
  const switches = await page.$$('button[role="switch"]');
  check(switches.length === 1, "Expert-mode switch found", `role=switch count = ${switches.length}`);
  if (!switches.length) throw new Error("Expert-mode switch not found — refusing a vacuous pass");
  await switches[0].click();
  await sleep(700);
  await page.screenshot({ path: stage("ist-system-expert-on.png") });

  const onText = await page.evaluate(() => document.body.innerText);
  const onSelects = await listSelects(page);
  check(
    rowsPresent(onText),
    "all four rows present with Expert mode ON",
    `labels found: ${JSON.stringify(ROWS.map((r) => r.label).filter((l) => onText.includes(l)))}`,
  );
  check(
    onSelects.length === ROWS.length,
    "exactly four KSelect rows added",
    JSON.stringify(onSelects),
  );
  if (onSelects.length !== ROWS.length) {
    throw new Error(
      `expected ${ROWS.length} KSelects on Advanced → System, found ${onSelects.length} — refusing to measure the wrong controls`,
    );
  }

  // ------------------------------------- (c) OPTION LISTS, against the fixture
  for (const row of ROWS) {
    const got = await readOptions(page, row.index);
    const want = EXPECTED_OPTIONS[row.label];
    const mismatch =
      got.length !== want.length || got.some((v, i) => v !== want[i]) ? true : false;
    check(
      !mismatch,
      `[${row.label}] options equal the fixture, element-wise`,
      `rendered ${JSON.stringify(got)} vs fixture ${JSON.stringify(want)}`,
    );
  }

  // ------------------------------------------- (d) CONTROL-STATES CONTRACT
  if (INVERT) {
    // ---- inversion 1: the SAME state assertion, wrong reference control.
    const raw = await rawSelectStyle(page);
    const m = await captureStates(page, ROWS[0].index);
    measured.inversionMeasured = m;
    measured.inversionRawSelect = raw;
    // Only `idle` can be pointed at the raw <select> (it has no listbox, no
    // options), so the fake reference carries the raw control's idle style and
    // the real reference elsewhere — the assertion below is byte-identical to
    // the green run's.
    assertStatesEqual(m, { ...reference, idle: raw }, "INVERSION-1 vs raw <select>");
  } else {
    for (const row of ROWS) {
      measured[row.label] = await captureStates(page, row.index);
      assertStatesEqual(measured[row.label], reference, row.label);
    }
  }
  await page.screenshot({ path: stage("ist-debug-rows.png") });

  if (INVERT) {
    // ---- inversion 3: the SAME leak predicate, fed by the SAME option reader,
    // on controls that DO carry `debug`. Restricted to the four KSelects on
    // purpose: the Log Level row's raw <select> also offers a "debug" option
    // (a log verbosity level), and letting that produce the red would prove
    // nothing about the picker scan.
    const labels = [];
    for (const row of ROWS) labels.push(...(await readOptions(page, row.index)));
    const leaked = leakedDebug(labels);
    check(
      leaked.length === 0,
      "INVERSION-3 (EXPECTED RED): no 'debug' option on Advanced → System with Expert mode ON",
      leaked.length
        ? `found ${JSON.stringify(leaked)} — the scan CAN see a debug option, so its green verdict elsewhere means something`
        : "no debug option found — the scan is blind and its green verdicts are void",
    );

    // ---- inversion 2: the SAME visibility predicate, with Expert mode OFF.
    await clickAria(page, "Back to settings");
    await sleep(400);
    await clickByText(page, "Advanced");
    await sleep(600);
    await clickByText(page, "System");
    await sleep(600);
    const text = await page.evaluate(() => document.body.innerText);
    check(
      rowsPresent(text),
      "INVERSION-2 (EXPECTED RED): all four rows present with Expert mode OFF",
      rowsPresent(text)
        ? "the rows rendered without the gate — the visibility check is void"
        : "the rows are absent, as they must be — the visibility predicate discriminates",
    );
  } else {
    // ------------------------------- (b) PICKERS again, Expert mode ON
    // Reached with a throwaway edit to the preview mock (trap #5): the panel's
    // writers are no-ops in preview, so clicking the switch cannot persist the
    // flag for the OTHER settings pages to read.
    setMockExpertMode(true);
    await sleep(1500);
    await page.reload({ waitUntil: "networkidle2", timeout: 30000 });
    await sleep(800);
    pass("boot (after mock edit)", await boot(page));
    await openSettingsCategory(page, "Advanced");
    await clickByText(page, "System");
    await sleep(600);
    const mockText = await page.evaluate(() => document.body.innerText);
    check(
      rowsPresent(mockText),
      "throwaway mock edit took effect (expertMode=true is loaded, not clicked)",
      "the four rows render without touching the switch — so the picker scan below really runs with Expert mode ON",
    );
    await clickAria(page, "Back to settings");
    await sleep(400);
    await scanPickers("expert ON");
  }
} catch (e) {
  fail("harness", String(e.message || e));
} finally {
  await browser.close();
  check(restoreMock(), "throwaway mock edit restored", `${MOCK_FILE} rewritten to its original bytes`);
}

/**
 * (b) The picker scan, run once per Expert-mode state.
 *
 * Recording & Audio carries the ONLY real provider picker. AI & Providers gets a
 * POSITIVE assertion instead of a zero-element scan: the page is proven to have
 * rendered its own content, and every control on it is proven to be a
 * non-provider control.
 */
async function scanPickers(phase) {
  // --- Recording & Audio: the real picker ---
  await clickByText(page, "Recording & Audio");
  await sleep(700);
  const raOpts = await collectOptionLabels(page);
  const raDropdowns = await page.evaluate(
    () => document.querySelectorAll('button[aria-haspopup="listbox"]').length,
  );
  check(
    raOpts.length > 0 && raDropdowns > 0,
    `[${phase}] Recording & Audio picker actually read`,
    `${raDropdowns} dropdown(s), ${raOpts.length} option label(s) — a zero-element scan would prove nothing`,
  );
  const raLeak = leakedDebug(raOpts);
  check(
    raLeak.length === 0,
    `[${phase}] no 'debug' option in the normal provider picker (Recording & Audio)`,
    `options: ${JSON.stringify(raOpts)}`,
  );
  await page.screenshot({ path: stage(`ist-picker-recording-audio-${phase.replace(/\s+/g, "-")}.png`) });
  await clickAria(page, "Back to settings");
  await sleep(400);

  // --- AI & Providers: positively no provider picker ---
  await clickByText(page, "AI & Providers");
  await sleep(700);
  // innerText applies CSS text-transform, and these section headings are
  // `uppercase` — match case-insensitively or this reads as "page not rendered".
  const apRendered = await page.evaluate(() =>
    /cleanup instructions/i.test(document.body.innerText) &&
    /app profiles/i.test(document.body.innerText),
  );
  check(
    apRendered,
    `[${phase}] AI & Providers page rendered`,
    "its own 'Cleanup Instructions' and 'App Profiles' sections are present — the scan below is of a real page, not an empty one",
  );
  // Its ONLY KSelects live inside per-profile rows, and the preview mock has no
  // profiles — so a scan of the page as it loads reads zero controls and would
  // be a vacuous pass. Add a profile first, which materialises every dropdown
  // this surface can ever show, and scan THOSE.
  await clickByText(page, "+ Add Profile");
  await sleep(600);
  const apControls = await page.evaluate(() =>
    [...document.querySelectorAll('button[aria-haspopup="listbox"], select')].map((el) => ({
      tag: el.tagName.toLowerCase(),
      label: (el.closest("div")?.parentElement?.innerText || "").split("\n")[0] || "",
    })),
  );
  const apOpts = await collectOptionLabels(page);
  check(
    apControls.length > 0 && apOpts.length > 0,
    `[${phase}] AI & Providers controls actually read`,
    `${apControls.length} control(s), ${apOpts.length} option label(s) after adding a profile — a zero-element scan would prove nothing`,
  );
  // POSITIVE assertion: not "we found no debug", but "every control this surface
  // can show is a non-provider control". A provider picker would offer at least
  // one of the config allowlist's provider names as an option label.
  const PROVIDER_NAMES = [
    ...EXPECTED_OPTIONS["LLM Provider"],
    ...EXPECTED_OPTIONS["STT Provider"],
  ];
  const providerish = apOpts.filter((o) =>
    PROVIDER_NAMES.some((p) => new RegExp(`(^|\\b)${p}(\\b|$)`, "i").test(o)),
  );
  check(
    providerish.length === 0,
    `[${phase}] AI & Providers carries no provider picker`,
    `${apControls.length} control(s): ${JSON.stringify(apControls)}; ` +
      `option labels: ${JSON.stringify(apOpts)}; provider-shaped labels: ${JSON.stringify(providerish)}`,
  );
  const apLeak = leakedDebug(apOpts);
  check(
    apLeak.length === 0,
    `[${phase}] no 'debug' option in AI & Providers`,
    `options: ${JSON.stringify(apOpts)}`,
  );
  await page.screenshot({ path: stage(`ist-picker-ai-providers-${phase.replace(/\s+/g, "-")}.png`) });
  await clickAria(page, "Back to settings");
  await sleep(400);
}

check(pageErrors.length === 0, "no page errors", pageErrors.join(" || ") || "none");

writeFileSync(stage(INVERT ? "inversion-measured.json" : "measured-states.json"), JSON.stringify(measured, null, 2));

// An inversion assertion is SUPPOSED to be red, so it is scored separately from
// the ordinary checks — otherwise "the run failed" would mean two opposite things.
const GROUPS = ["INVERSION-1", "INVERSION-2", "INVERSION-3"];
const isInversion = (r) => GROUPS.some((g) => r.name.includes(g));
const ordinary = results.filter((r) => !isInversion(r));
const ordinaryFailed = ordinary.filter((r) => !r.ok);
const groupSummary = GROUPS.map((g) => {
  const rows = results.filter((r) => r.name.includes(g));
  const red = rows.filter((r) => !r.ok).length;
  return { g, total: rows.length, red };
});
const inversionOk = !INVERT || groupSummary.every((s) => s.total > 0 && s.red > 0);
const ok = ordinaryFailed.length === 0 && inversionOk;

const lines = [
  `# Story 13-1 — desktop proxy gate${INVERT ? " (INVERSION RUN)" : ""}`,
  "",
  `Run: ${new Date().toISOString()}`,
  `Checks: ${results.length} (derived from the result records, not hand-counted)`,
  `Ordinary checks: ${ordinary.length}, failed: ${ordinaryFailed.length}`,
  `Result: ${ok ? "PASS" : "FAIL"}`,
  ...(INVERT
    ? [
        "",
        "An inversion assertion PASSES BY GOING RED. Each group below must contain",
        "at least one red assertion; that is what licenses the green run.",
        ...groupSummary.map(
          (s) => `- ${s.g}: ${s.red}/${s.total} assertion(s) RED ${s.red > 0 ? "✓" : "✗ (the harness is blind here)"}`,
        ),
      ]
    : []),
  "",
  ...results.map((r) => `- [${r.ok ? "x" : " "}] **${r.name}** — ${r.detail}`),
  "",
];
writeFileSync(stage(INVERT ? "inversion-report.md" : "report.md"), lines.join("\n"));

for (const f of readdirSync(STAGE)) copyFileSync(join(STAGE, f), join(HERE, f));
console.log(lines.join("\n"));
process.exit(ok ? 0 : 1);
