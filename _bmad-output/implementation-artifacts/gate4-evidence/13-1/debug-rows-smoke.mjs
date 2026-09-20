/**
 * Story 13-1 — desktop PROXY gate for the two debug-scenario rows.
 *
 * WHY A PUPPETEER HARNESS: this repo has no JS test runner (no vitest, no jest)
 * and adding one is forbidden (project-context: no `npm install` from a story;
 * the Windows build runs `npm ci`). The documented desktop MACHINE gate for a
 * React surface is puppeteer against `npm run preview` (port 1422) in real
 * Chromium. This is that.
 *
 * WHAT IT ASSERTS, in one run:
 *   (a) VISIBILITY — the two rows are ABSENT from Settings → Advanced → System
 *       with Expert mode off, and PRESENT with it on.
 *   (b) PICKER — `debug` is not an option in the normal provider pickers
 *       (Settings → Recording & Audio, Settings → AI & Providers).
 *   (c) CONTROL-STATES CONTRACT — for every state the harness can drive
 *       (idle, hover, focus, press, open, the open listbox, its selected option
 *       and its keyboard-focused option), `getComputedStyle` of the NEW control
 *       equals that of the NAMED REFERENCE INSTANCE — the "Dictation language"
 *       `KSelect` at src/components/settings/LanguageContent.tsx:59 — on
 *       background-color, border-color, border-radius, padding, font-size,
 *       color and box-shadow. A mismatch on any property fails the gate.
 *
 * INVERSION (`--invert`): the same comparison is run against a DELIBERATELY
 * DIFFERENT control — the raw `<select>` of the Log Level row
 * (AdvancedSettingsPanel.tsx, the pre-existing outlier recorded under
 * frontmatter `deferred`) — and MUST report a mismatch. A comparison harness
 * that has never been shown red proves nothing.
 *
 * WHAT IT DOES NOT EXERCISE (stated explicitly per project-context):
 *   - persistence: preview-mode writers are no-ops, so "Save" is not clicked
 *     and nothing reaches config.json. The allowlist round-trip is the Rust
 *     suite's claim (config::tests::spec_debug_provider_survives_normalization);
 *   - anything Rust: no Tauri, no provider resolution, no canned wire;
 *   - the `disabled` state of KSelect — these two rows never pass `disabled`,
 *     the Expert-mode gate removes them instead;
 *   - pixels, font rasterisation, the Windows text-scale drift — Andi's
 *     real-screen gate;
 *   - Android.
 *
 * Trap #1 (documented): preview boots into Onboarding → click
 * "Setup überspringen" first, or every selector times out.
 * Trap #2 (documented): the evidence dir is inside the repo and `npm run
 * preview` is the Vite DEV server, so writing artifacts mid-run triggers an HMR
 * reload. Everything is staged in a temp dir and copied in after the browser
 * closes.
 *
 * Run: node _bmad-output/implementation-artifacts/gate4-evidence/13-1/debug-rows-smoke.mjs [--invert]
 */
import puppeteer from "puppeteer";
import { writeFileSync, mkdtempSync, copyFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const BASE = "http://localhost:1422";
const INVERT = process.argv.includes("--invert");
const STAGE = mkdtempSync(join(tmpdir(), "klarvo-13-1-"));
const stage = (n) => join(STAGE, n);

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

const results = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const pass = (name, detail) => results.push({ ok: true, name, detail });
const fail = (name, detail) => results.push({ ok: false, name, detail });
const check = (c, name, detail) => (c ? pass(name, detail) : fail(name, detail));

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

/**
 * Opens every dropdown on the current page (one at a time, with real awaits --
 * a synchronous click inside page.evaluate never lets React render the portal)
 * and returns every option label found, plus every native <select> option.
 */
async function collectOptionLabels(page) {
  const out = [];
  const n = await page.evaluate(
    () => document.querySelectorAll('button[aria-haspopup="listbox"]').length,
  );
  for (let i = 0; i < n; i++) {
    await page.evaluate((i) => {
      document.querySelectorAll('button[aria-haspopup="listbox"]')[i].click();
    }, i);
    await sleep(250);
    const labels = await page.evaluate(() =>
      [...document.querySelectorAll('ul[role="listbox"] li[role="option"]')].map((l) =>
        (l.innerText || "").trim(),
      ),
    );
    out.push(...labels);
    await page.evaluate((i) => {
      document.querySelectorAll('button[aria-haspopup="listbox"]')[i].click();
    }, i);
    await sleep(200);
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
    return Object.fromEntries(PROPS.map((p) => [p, cs[p]]));
  }, PROPS);

  // keyboard-focused option: ArrowDown moves the highlight off the selected one
  await page.keyboard.press("ArrowDown");
  await sleep(180);
  // The keyboard highlight is the EXACT class token `bg-klarvo-surface-2`.
  // Matching it as a substring also hits `hover:bg-klarvo-surface-2`, which
  // every enabled option carries — that read option[0] and silently compared
  // the selected option on one control against a plain option on the other
  // (harness trap found in run 4).
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

  // Close by clicking the trigger again, NOT with Escape: KSelect's Escape
  // handler does not stop propagation, so the key reaches SettingsPanel's own
  // document listener and closes the whole panel (harness trap found in run 2).
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

const browser = await puppeteer.launch({
  headless: "new",
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 950 });

const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(String(e.stack || e).split("\n").slice(0, 4).join(" | ")));

let reference = null;
let measured = null;

try {
  await page.goto(BASE, { waitUntil: "networkidle2", timeout: 30000 });
  const boot = await page.evaluate(() => document.body.innerText);
  if (boot.includes("Setup überspringen")) {
    await clickByText(page, "Setup überspringen");
    await sleep(1200);
    pass("onboarding skipped", "clicked 'Setup überspringen'");
  } else {
    pass("onboarding not shown", "app booted past onboarding");
  }

  // ------------------------------------------------------------- REFERENCE
  await clickAria(page, "Toggle settings");
  await sleep(500);
  await clickByText(page, "Language");
  await sleep(500);
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

  // --------------------------------------------------------------- PICKERS
  await clickAria(page, "Back to settings");
  await sleep(400);
  let totalPickerLabels = 0;
  for (const [cat, label] of [
    ["Recording & Audio", "recording-audio"],
    ["AI & Providers", "ai-providers"],
  ]) {
    await clickByText(page, cat);
    await sleep(600);
    const opts = await collectOptionLabels(page);
    const dropdowns = await page.evaluate(
      () => document.querySelectorAll('button[aria-haspopup="listbox"]').length,
    );
    totalPickerLabels += opts.length;
    const leaked = opts.filter((o) => /(^|\b)debug(\b|$)/i.test(o));
    check(
      leaked.length === 0,
      `no 'debug' option in ${cat}`,
      `${dropdowns} dropdown(s), ${opts.length} option label(s): ${JSON.stringify(opts.slice(0, 24))}` +
        (opts.length === 0
          ? " — this page carries no picker in preview (AI & Providers' KSelects live inside per-profile rows, and the mock has no profiles), so the claim rests on Recording & Audio"
          : ""),
    );
    // A standalone `debug` token must not appear in the page text either — a
    // picker rendered as something other than a dropdown would slip past the
    // option scan above.
    const bodyHasDebug = await page.evaluate(() =>
      /(^|\s)debug($|\s)/i.test(document.body.innerText),
    );
    check(!bodyHasDebug, `no standalone 'debug' text in ${cat}`, `body scan`);
    await page.screenshot({ path: stage(`ist-picker-${label}.png`) });
    await clickAria(page, "Back to settings");
    await sleep(400);
  }

  // Guard against a vacuous picker claim across BOTH pages: if neither page
  // yielded a single option label, the two checks above proved nothing.
  check(
    totalPickerLabels > 0,
    "provider-picker options were actually read",
    `${totalPickerLabels} option label(s) read across the two picker pages`,
  );

  // ------------------------------------------------------- VISIBILITY GATE
  await clickByText(page, "Advanced");
  await sleep(600);
  await clickByText(page, "System");
  await sleep(600);
  await page.screenshot({ path: stage("ist-system-expert-off.png") });

  const offText = await page.evaluate(() => document.body.innerText);
  const offSelects = await listSelects(page);
  check(
    !offText.includes("Debug LLM Scenario") && !offText.includes("Debug STT Scenario"),
    "rows absent with Expert mode OFF",
    `KSelect count on the System page = ${offSelects.length} (expected 0)`,
  );

  // The Expert-mode control is a bare role="switch" button whose own innerText
  // is empty (the label lives in a sibling span), so it cannot be clicked by
  // text. It is the only switch on the System page.
  const switches = await page.$$('button[role="switch"]');
  check(switches.length === 1, "Expert-mode switch found", `role=switch count = ${switches.length}`);
  if (!switches.length) throw new Error("Expert-mode switch not found — refusing a vacuous pass");
  await switches[0].click();
  await sleep(700);
  await page.screenshot({ path: stage("ist-system-expert-on.png") });

  const onText = await page.evaluate(() => document.body.innerText);
  const onSelects = await listSelects(page);
  check(
    onText.includes("Debug LLM Scenario") && onText.includes("Debug STT Scenario"),
    "rows present with Expert mode ON",
    `KSelect count on the System page = ${onSelects.length} (expected 2)`,
  );
  check(onSelects.length === 2, "exactly two KSelect rows added", JSON.stringify(onSelects));

  // Option labels are the scenario strings, verbatim.
  const optLabels = {};
  for (let i = 0; i < 2; i++) {
    await page.evaluate((i) => {
      document.querySelectorAll('button[aria-haspopup="listbox"]')[i].click();
    }, i);
    await sleep(250);
    optLabels[i] = await page.evaluate(() =>
      [...document.querySelectorAll('ul[role="listbox"] li[role="option"]')].map((l) =>
        (l.innerText || "").trim(),
      ),
    );
    await page.evaluate((i) => {
      document.querySelectorAll('button[aria-haspopup="listbox"]')[i].click();
    }, i);
    await sleep(200);
  }
  check(
    JSON.stringify(optLabels[0]) ===
      JSON.stringify(["ok", "empty", "truncated", "malformed", "http429", "http5xx", "transport"]),
    "LLM scenario options are the seven strings, verbatim",
    JSON.stringify(optLabels[0]),
  );
  check(
    JSON.stringify(optLabels[1]) ===
      JSON.stringify(["ok", "empty", "malformed", "http429", "http5xx", "transport"]),
    "STT scenario options are the LLM set minus 'truncated'",
    JSON.stringify(optLabels[1]),
  );

  // ------------------------------------------- CONTROL-STATES CONTRACT (c)
  if (INVERT) {
    // Compare the NEW trigger's idle state against the raw <select> of the Log
    // Level row. They must NOT match — this is the harness's own red run.
    const raw = await rawSelectStyle(page);
    const mine = await triggerStyle(page, 0);
    const d = diff(mine, raw);
    // Deliberately the SAME equality assertion the green run makes, pointed at
    // the wrong control. This run is EXPECTED TO FAIL: its red verdict is what
    // licenses the green one. A comparison that has never been shown red proves
    // nothing (project-context: "a guard that is red on a clean tree is
    // worthless" — and its mirror, a guard that can never go red).
    check(
      d.length === 0,
      "INVERSION (EXPECTED RED): idle state equals the raw <select> of the Log Level row",
      d.length
        ? `differing properties (${d.length}/${PROPS.length}): ${d.join(" | ")}`
        : "no difference — the harness cannot discriminate and its green run is void",
    );
    measured = { inversionRawSelect: raw, inversionNewTrigger: mine, differences: d };
  } else {
    measured = await captureStates(page, 0);
    for (const state of [
      "idle",
      "hover",
      "focused",
      "open",
      "listbox",
      "optionSelected",
      "optionKeyboardFocused",
    ]) {
      const d = diff(measured[state], reference[state]);
      if (state === "optionKeyboardFocused") {
        check(
          measured[state]?.__ariaSelected === reference[state]?.__ariaSelected,
          "keyboard-focused option compared like-for-like",
          `new aria-selected=${measured[state]?.__ariaSelected} (idx ${measured[state]?.__index}), ` +
            `reference aria-selected=${reference[state]?.__ariaSelected} (idx ${reference[state]?.__index})`,
        );
      }
      check(
        d.length === 0,
        `state '${state}' equals the reference instance`,
        d.length ? d.join(" | ") : `all ${PROPS.length} properties equal`,
      );
    }
    check(
      measured.openMeta?.ariaExpanded === "true" && measured.openMeta?.chevronRotated === true,
      "open state: aria-expanded=true + chevron rotated",
      JSON.stringify(measured.openMeta),
    );
    // "pressed" has no distinct styling in the shipped component: a press opens
    // the listbox, so pressed IS the open state. Assert that explicitly rather
    // than silently skipping it.
    check(
      JSON.stringify(measured.open) === JSON.stringify(reference.open),
      "state 'pressed' (= open) equals the reference instance",
      "the shipped KSelect defines no distinct pressed styling",
    );
  }

  writeFileSync(stage("measured-states.json"), JSON.stringify(measured, null, 2));
  await page.screenshot({ path: stage("ist-debug-rows.png") });
} catch (e) {
  fail("harness", String(e.message || e));
} finally {
  await browser.close();
}

check(pageErrors.length === 0, "no page errors", pageErrors.join(" || ") || "none");

const failed = results.filter((r) => !r.ok);
const lines = [
  `# Story 13-1 — desktop proxy gate${INVERT ? " (INVERSION RUN)" : ""}`,
  "",
  `Run: ${new Date().toISOString()}`,
  `Result: ${failed.length === 0 ? "PASS" : `FAIL (${failed.length}/${results.length})`}`,
  "",
  ...results.map((r) => `- [${r.ok ? "x" : " "}] **${r.name}** — ${r.detail}`),
  "",
];
writeFileSync(stage(INVERT ? "inversion-report.md" : "report.md"), lines.join("\n"));

for (const f of readdirSync(STAGE)) copyFileSync(join(STAGE, f), join(HERE, f));
console.log(lines.join("\n"));
process.exit(failed.length === 0 ? 0 : 1);
