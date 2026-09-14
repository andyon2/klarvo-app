/**
 * Story 7-10 — review round 1, directive D1: PROXY smoke for the main-window
 * status line.
 *
 * D1 (Andi): "Hauptfenster zeigt den Cleanup-Fehler in der bestehenden
 * Status-Zeile in Amber statt 'Done' (Zustand done mit Warnung), gleicher
 * Wortlaut wie die Pill. Keine neue Fläche."
 *
 * WHY A PUPPETEER HARNESS AND NOT A UNIT TEST: this repo has no JS test runner
 * (no vitest, no jest — see package.json), and adding one is forbidden
 * (project-context: no `npm install` from a story, the Windows build runs
 * `npm ci`). The documented desktop MACHINE gate for a React surface is
 * puppeteer against `npm run preview` in real Chromium. This is that.
 *
 * THROWAWAY EDIT (documented procedure, project-context "Testing Rules"): the
 * preview mocks cannot produce a hotkey-driven `done + warning` event —
 * `onStateChanged` returns `mockListen()` and never fires. The harness therefore
 * requires src/tauri-commands.ts to be patched so preview mode parks the
 * callback on `window.__klarvoPreviewEmit`. The patch is applied by the runner
 * script, measured, and reverted; `git status` must be clean afterwards.
 *
 * WHAT THIS PROVES: wiring and structure of the real React tree — that a
 * `done` event carrying `warning` renders that exact text in the existing
 * status line, in the amber token, instead of the "Done" label; and the
 * inverse, that a `done` event WITHOUT a warning is unchanged (teal, "Fertig").
 * Computed style is compared against the literal `--color-klarvo-amber` /
 * `--color-klarvo-teal` values from src/styles.css (canon tokens, ADR-0019).
 *
 * WHAT IT DOES NOT EXERCISE (stated explicitly per project-context):
 *   - the Rust backend: no Tauri, no real `klarvo://state-changed` emission,
 *     no `emit_pipeline_state`, no pipeline run. The payload here is synthetic;
 *     that the backend really puts `degrade_msg` on the terminal event is the
 *     Rust suite's claim (`spec_*` in pipeline.rs), not this harness's;
 *   - the native pill (`native_pill.rs`, Windows-only) — Andi's GATE-4;
 *   - Android;
 *   - pixels, fonts, truncation, the Windows text-scale drift.
 *
 * REVIEW ROUND 2 added CASE C: a degraded run followed by a clean run inside the
 * SAME browser boot, which is the sequence a hotkey user produces and the only
 * one that can catch a stale `warningMessage` (`handleRecordToggle` — the button
 * path — is never touched). Round 1 stated this sequence as NOT exercised; it now
 * is. Still synthetic payloads: what a real pipeline emits between two runs is
 * the Rust suite's claim, not this harness's.
 *
 * Harness traps inherited from 7-9's smoke.mjs, both handled below:
 *   #1 preview boots into Onboarding -> click "Setup überspringen" first.
 *   #2 the evidence dir is inside the repo and `npm run preview` is the Vite
 *      DEV server: writing artifacts there mid-run triggers an HMR reload that
 *      remounts the app. All artifacts are staged in a temp dir and copied in
 *      after the browser closes.
 *
 * Run: node .../7-10/d1-status-line-smoke.mjs   (with preview serving on 1422
 *      and the throwaway patch applied — use run-d1-smoke.sh, which does both)
 */
import puppeteer from "puppeteer";
import { writeFileSync, mkdtempSync, copyFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const BASE = "http://localhost:1422";
const STAGE = mkdtempSync(join(tmpdir(), "klarvo-7-10-d1-"));
const stage = (name) => join(STAGE, name);

// Canon tokens, literal values from src/styles.css (ADR-0019). Compared as
// computed rgb() so a renamed/rebound Tailwind class cannot pass silently.
const AMBER = "rgb(233, 162, 76)"; // --color-klarvo-amber #E9A24C
const TEAL_TOKEN = "--color-klarvo-teal";

// The wording the pill shows, from pipeline::degrade_warn_msg_for_model (7-9 D2
// + Story 7-10 Q2). Same literal on purpose: D1 says "gleicher Wortlaut".
const DEGRADE_MSG = "Model 'deepseek-typo' not found — in clipboard";

const results = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const pass = (name, detail) => results.push({ ok: true, name, detail });
const fail = (name, detail) => results.push({ ok: false, name, detail });
const check = (cond, name, detail) => (cond ? pass(name, detail) : fail(name, detail));

async function textOf(page) {
  return page.evaluate(() => document.body.innerText);
}

async function clickByText(page, label) {
  const clicked = await page.evaluate((label) => {
    const nodes = [...document.querySelectorAll("button, a, [role=button]")];
    const matches = nodes.filter((n) => (n.innerText || "").includes(label));
    if (!matches.length) return false;
    matches.sort((a, b) => (a.innerText || "").length - (b.innerText || "").length);
    matches[0].click();
    return true;
  }, label);
  if (!clicked) throw new Error(`cannot find clickable element for "${label}"`);
  await sleep(300);
}

/** Emits one synthetic pipeline event through the parked listener. */
async function emitState(page, payload) {
  const delivered = await page.evaluate((p) => {
    const emit = window.__klarvoPreviewEmit;
    if (typeof emit !== "function") return false;
    emit(p);
    return true;
  }, payload);
  if (!delivered) {
    throw new Error(
      "window.__klarvoPreviewEmit missing — the throwaway patch to " +
        "src/tauri-commands.ts is not applied; run via run-d1-smoke.sh",
    );
  }
  await sleep(250);
}

/**
 * The status line under the record button. Located by its role in the tree
 * rather than a test id (AC4: no new surface, so no new attribute either).
 */
async function readStatusLine(page) {
  return page.evaluate(() => {
    const ps = [...document.querySelectorAll("p.text-xs.font-medium")];
    // The status label is the one inside the centered wrapper next to the
    // record button; it is the only such <p> before any panel is open.
    const el = ps.find((p) => p.closest("div.text-center"));
    if (!el) return null;
    return {
      text: (el.innerText || "").trim(),
      color: getComputedStyle(el).color,
      classes: el.className,
    };
  });
}

const browser = await puppeteer.launch({
  headless: "new",
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 900 });

const pageErrors = [];
const consoleErrors = [];
page.on("pageerror", (e) =>
  pageErrors.push(String(e.stack || e).split("\n").slice(0, 6).join(" | ")),
);
page.on("console", (m) => {
  if (m.type() === "error") consoleErrors.push(m.text());
});

let teal = null;

try {
  await page.goto(BASE, { waitUntil: "networkidle2", timeout: 30000 });

  // Trap #1: preview boots into Onboarding.
  const boot = await textOf(page);
  if (boot.includes("Setup überspringen")) {
    await clickByText(page, "Setup überspringen");
    await sleep(1200);
    const after = await textOf(page);
    if (after.includes("Setup überspringen")) {
      throw new Error("onboarding did not dismiss after clicking 'Setup überspringen'");
    }
    pass("onboarding skipped", "clicked 'Setup überspringen'");
  } else {
    pass("onboarding not shown", "app booted past onboarding");
  }
  await sleep(400);

  // Resolve the teal token from the live stylesheet, so the "unchanged" half is
  // checked against the canon value and not against a hardcoded guess.
  teal = await page.evaluate((name) => {
    const raw = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    const probe = document.createElement("span");
    probe.style.color = raw;
    document.body.appendChild(probe);
    const rgb = getComputedStyle(probe).color;
    probe.remove();
    return { raw, rgb };
  }, TEAL_TOKEN);

  // Guard against a vacuous pass: if the status line cannot be found at all,
  // every text assertion below would compare null to null.
  const idle = await readStatusLine(page);
  check(
    idle !== null,
    "status line found",
    idle ? `idle text=${JSON.stringify(idle.text)}` : "NOT FOUND — all checks below are void",
  );
  if (idle === null) throw new Error("status line not found; refusing to report a vacuous pass");

  // ------------------------------------------------------------------ CASE A
  // done + warning -> the cause, in amber, instead of the "Fertig" label.
  await emitState(page, {
    state: "done",
    text: "so like the thing by friday",
    rawText: "so like the thing by friday",
    clipboardOnly: true,
    warning: DEGRADE_MSG,
  });
  const degraded = await readStatusLine(page);
  writeFileSync(stage("status-degraded.json"), JSON.stringify(degraded, null, 2));
  await page.screenshot({ path: stage("ist-status-degraded.png") });

  check(
    degraded.text === DEGRADE_MSG,
    "D1: status line shows the degrade cause verbatim",
    `text=${JSON.stringify(degraded.text)} expected=${JSON.stringify(DEGRADE_MSG)}`,
  );
  check(
    degraded.text.includes("deepseek-typo"),
    "D1: the model ID survives into the main window (7-9 D2)",
    `text=${JSON.stringify(degraded.text)}`,
  );
  check(
    degraded.color === AMBER,
    "D1: rendered in the canon amber token",
    `computed=${degraded.color} expected=${AMBER} (--color-klarvo-amber)`,
  );
  check(
    !/\bDone\b/.test(degraded.text),
    "D1: the 'Done' label is replaced, not appended",
    `text=${JSON.stringify(degraded.text)}`,
  );

  // ------------------------------------------------------------------ CASE C
  // Review round 2: the SAME boot continues with a clean run. `warningMessage`
  // must be reset by the listener, otherwise the next successful run wears the
  // previous run's amber degrade text instead of "Done". No click on the record
  // button here on purpose — `handleRecordToggle` holds the only other clearing
  // path, and a hotkey-driven run never goes through it.
  await emitState(page, { state: "transcribing" });
  await emitState(page, { state: "cleaning" });
  await emitState(page, {
    state: "done",
    text: "Send me the report by Friday.",
    rawText: "so like the report by friday",
  });
  const afterDegrade = await readStatusLine(page);
  writeFileSync(stage("status-clean-after-degrade.json"), JSON.stringify(afterDegrade, null, 2));
  await page.screenshot({ path: stage("ist-status-clean-after-degrade.png") });

  check(
    afterDegrade.text === "Done",
    "round 2: a clean run after a degraded run shows the plain done label",
    `text=${JSON.stringify(afterDegrade.text)} expected="Done"`,
  );
  check(
    !afterDegrade.text.includes("deepseek-typo"),
    "round 2: the previous run's degrade cause is gone, not stale",
    `text=${JSON.stringify(afterDegrade.text)}`,
  );
  check(
    afterDegrade.color === teal.rgb,
    "round 2: a clean run after a degraded run is teal, not amber",
    `computed=${afterDegrade.color} expected=${teal.rgb} (${TEAL_TOKEN} = ${teal.raw})`,
  );

  // ------------------------------------------------------------------ CASE B
  // The regression half, in a fresh boot so no warning is in state: an ordinary
  // successful run must look exactly as it did before D1.
  await page.reload({ waitUntil: "networkidle2", timeout: 30000 });
  const boot2 = await textOf(page);
  if (boot2.includes("Setup überspringen")) {
    await clickByText(page, "Setup überspringen");
    await sleep(1200);
  }
  await sleep(400);

  await emitState(page, {
    state: "done",
    text: "Send me the report by Friday.",
    rawText: "so like the report by friday",
  });
  const ok = await readStatusLine(page);
  writeFileSync(stage("status-ok.json"), JSON.stringify(ok, null, 2));
  await page.screenshot({ path: stage("ist-status-ok.png") });

  // STATUS_LABELS (src/types.ts) is English — "Done", not "Fertig". D1's
  // German phrasing names the state, not the literal.
  check(
    ok.text === "Done",
    "regression: a clean run still shows the plain done label",
    `text=${JSON.stringify(ok.text)}`,
  );
  check(
    ok.color === teal.rgb,
    "regression: a clean run stays teal",
    `computed=${ok.color} expected=${teal.rgb} (${TEAL_TOKEN} = ${teal.raw})`,
  );
  check(
    ok.color !== AMBER,
    "regression: a clean run is NOT amber",
    `computed=${ok.color}`,
  );
} catch (e) {
  fail("harness", String(e.stack || e));
} finally {
  await browser.close();
}

check(pageErrors.length === 0, "no uncaught page errors", pageErrors.join(" ;; ") || "none");

const report = [
  `Story 7-10 — D1 status-line proxy smoke`,
  `run: ${new Date().toISOString()}`,
  `base: ${BASE}`,
  `teal token: ${teal ? `${teal.raw} -> ${teal.rgb}` : "not resolved"}`,
  ``,
  ...results.map((r) => `${r.ok ? "PASS" : "FAIL"}  ${r.name}\n        ${r.detail}`),
  ``,
  `console.error (handled, not crashes): ${consoleErrors.length}`,
  ...consoleErrors.map((c) => `        ${c}`),
  ``,
  `RESULT: ${results.filter((r) => r.ok).length}/${results.length} checks passed`,
].join("\n");

writeFileSync(stage("report.txt"), report);
console.log(report);

// Trap #2: copy artifacts in only after the browser is gone.
for (const f of readdirSync(STAGE)) copyFileSync(join(STAGE, f), join(HERE, f));

process.exit(results.every((r) => r.ok) ? 0 : 1);
