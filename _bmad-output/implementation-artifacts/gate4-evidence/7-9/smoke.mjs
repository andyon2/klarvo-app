/**
 * Story 7-9 — desktop PROXY smoke (AC7).
 *
 * Drives real Chromium against `npm run preview` (port 1422) and asserts that
 * the Advanced panel and Settings -> Shortcuts -> "Paste & Behavior" render
 * ONLY live keys after the 13 dead keys were removed.
 *
 * WHAT THIS PROVES: wiring and structure — which rows/inputs the React tree
 * actually renders, in both expert-mode states, across all four Advanced
 * sections. A proxy render cannot prove design (pixels, font rasterisation,
 * Windows text-scale drift) and it does not touch Tauri: `isPreviewMode` in
 * src/tauri-commands.ts serves MOCK data, so nothing here exercises the Rust
 * config, the save chain's backend half, or the model-ID runtime path.
 *
 * WHAT IT DOES NOT EXERCISE (stated explicitly per project-context):
 *   - the Rust backend (no Tauri): no save_advanced_settings, no hot-reload,
 *     no `Klarvo.log` line, no real config.json read/write;
 *   - Android (no Kotlin path here at all);
 *   - the Windows release build (Andi's GATE-4);
 *   - a real cleanup request carrying an overridden model ID;
 *   - pixel/visual fidelity of any kind.
 *
 * Trap #1 (preview boots into Onboarding) is handled: "Setup überspringen" is
 * clicked first, otherwise every selector times out.
 *
 * HARNESS TRAP found while writing this (2026-09-12): a `fullPage: true`
 * screenshot resizes the emulated viewport, which REMOUNTS the app and resets
 * the Settings panel to its home view — measured as 31 divs -> 1 -> 12, with
 * `[aria-label="Back to settings"]` gone and every row query returning []. It
 * looks exactly like "the rows were removed". All screenshots here are
 * therefore VIEWPORT-only (1280x900). If a check ever reports zero matching
 * nodes, suspect this before believing the product changed: the zero-count
 * guards below exist so such a run FAILS instead of passing vacuously.
 *
 * Run:  node _bmad-output/implementation-artifacts/gate4-evidence/7-9/smoke.mjs
 *       (with `npm run preview` already serving on 1422)
 */
import puppeteer from "puppeteer";
import { writeFileSync, mkdtempSync, copyFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const BASE = "http://localhost:1422";

// HARNESS TRAP #2 (2026-09-12): the evidence dir lives INSIDE the repo, and
// `npm run preview` is the Vite DEV server — its file watcher sees every .png
// and .txt this harness writes and triggers an HMR full reload, which remounts
// the app back to its initial view mid-run. Measured as 31 divs -> 1 right
// after the first artifact write, with the Settings panel reset to home.
// So: stage all artifacts in a temp dir outside the watched tree and copy them
// into the evidence dir only after the browser is closed.
const STAGE = mkdtempSync(join(tmpdir(), "klarvo-7-9-smoke-"));
const stage = (name) => join(STAGE, name);

// The 13 keys story 7-9 removed, as they would surface in the UI: the visible
// label text of the row each key drove.
const DEAD_ROW_LABELS = [
  "STT Temperature",       // #1  sttTemperature
  "LLM Temperature",       // #2  llmTemperature
  "Max Tokens",            // #3  llmMaxTokens
  "Chunk Threshold",       // #4  chunkThreshold
  "Chunk Target Size",     // #5  chunkTargetSize
  "Auto-Paste",            // #6  autoPaste
  "Auto-Capitalize",       // #7  autoCapitalize
  "System Prompt: Polished",  // #10 llmSystemPromptPolished
  "System Prompt: Verbatim",  // #11 llmSystemPromptVerbatim
  "System Prompt: Chat",      // #12 llmSystemPromptChat
  "Command Mode Prompt",      // #13 llmCommandModePrompt
];
// #8/#9 (bubbleTapAutoSend / bubbleLongPressAutoSend) had NO toggle anywhere
// before this story either, so the UI cannot show their removal. They are
// covered by the TS/Rust/Kotlin greps and the type system, not here.

// Copy that must be GONE (Q2/Q3).
const DEAD_COPY = [
  "Custom prompts & temperature",
  "Models, parameters & instructions",
  "Model & Parameters",
  "Custom Cleanup Instructions",
  "Reveals raw audio thresholds, chunking and STT temperature",
];

// Copy that must be PRESENT (Q2/Q3/Q4).
const LIVE_COPY = [
  "Custom prompts",
  "Model IDs",
  "Reveals raw audio thresholds.",
];

const results = [];
const fail = (name, detail) => results.push({ ok: false, name, detail });
const pass = (name, detail = "") => results.push({ ok: true, name, detail });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function textOf(page) {
  return page.evaluate(() => document.body.innerText);
}

/**
 * Click the element whose text matches `label`, preferring real clickables and
 * the TIGHTEST match.
 *
 * Naive "first element whose innerText contains X" picks the outer app
 * container — its innerText contains everything — and clicking a div does
 * nothing. So: consider only button/[role=button]/a, and among the matches take
 * the one with the shortest innerText (the leaf, not an ancestor).
 */
async function clickByText(page, label, { exact = false } = {}) {
  const clicked = await page.evaluate(
    (label, exact) => {
      const els = [...document.querySelectorAll("button, [role=button], a")];
      const matches = els.filter((e) => {
        const t = (e.innerText || "").trim();
        return exact ? t === label : t.includes(label);
      });
      if (!matches.length) return false;
      matches.sort((a, b) => (a.innerText || "").length - (b.innerText || "").length);
      matches[0].click();
      return true;
    },
    label,
    exact,
  );
  if (!clicked) throw new Error(`cannot find clickable element for "${label}"`);
  await sleep(300);
}

const browser = await puppeteer.launch({
  headless: "new",
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 900 });

// Keep the two sources apart: an uncaught page error is a crash, a
// console.error is code reporting a handled failure. Lumping them together
// makes a benign preview-mode log look like a crash.
const pageErrors = [];
const consoleErrors = [];
page.on("pageerror", (e) =>
  pageErrors.push(String(e.stack || e).split("\n").slice(0, 6).join(" | ")),
);
page.on("console", (m) => {
  if (m.type() === "error") consoleErrors.push(m.text());
});

try {
  await page.goto(BASE, { waitUntil: "networkidle2", timeout: 30000 });

  // Trap #1: preview boots into Onboarding.
  const boot = await textOf(page);
  if (boot.includes("Setup überspringen")) {
    await clickByText(page, "Setup überspringen");
    await sleep(1200); // handleSkip is async: setOnboardingState -> getSettings
    const after = await textOf(page);
    if (after.includes("Setup überspringen")) {
      throw new Error("onboarding did not dismiss after clicking 'Setup überspringen'");
    }
    pass("onboarding skipped", "clicked 'Setup überspringen'");
  } else {
    pass("onboarding not shown", "app booted past onboarding");
  }
  await sleep(400);

  // ---------------------------------------------------------------- Shortcuts
  await page.click('[aria-label="Toggle settings"]');
  await sleep(400);
  await clickByText(page, "Shortcuts");
  await sleep(400);

  const shortcuts = await textOf(page);
  writeFileSync(stage("text-shortcuts.txt"), shortcuts);
  await page.screenshot({ path: stage("01-shortcuts-paste-behavior.png") });

  // The heading has Tailwind `uppercase`, so innerText yields "PASTE & BEHAVIOR".
  // Compare case-insensitively — otherwise the harness reports a product defect
  // that does not exist.
  const shortcutsCI = shortcuts.toLowerCase();
  if (!shortcutsCI.includes("paste & behavior")) {
    fail("Shortcuts: section heading kept (Q4)", "'Paste & Behavior' not found");
  } else {
    pass("Shortcuts: section heading kept (Q4)", "rendered as 'PASTE & BEHAVIOR' (CSS uppercase)");
  }
  for (const label of ["Auto-Paste", "Auto-Capitalize"]) {
    if (shortcuts.includes(label)) fail(`Shortcuts: dead row '${label}' removed`, "still rendered");
    else pass(`Shortcuts: dead row '${label}' removed`);
  }
  for (const label of ["Auto-Send", "Paste Delay (ms)"]) {
    if (!shortcuts.includes(label)) fail(`Shortcuts: live row '${label}' kept (Q4)`, "missing");
    else pass(`Shortcuts: live row '${label}' kept (Q4)`);
  }

  // Q4: Auto-Send and Paste Delay must no longer be dimmed/disabled.
  const dimming = await page.evaluate(() => {
    const rows = [...document.querySelectorAll("div")].filter((d) => {
      const t = (d.innerText || "").trim();
      return t.startsWith("Auto-Send") || t.startsWith("Paste Delay (ms)");
    });
    const out = [];
    for (const r of rows) {
      const row = r.closest("div.flex.items-center.justify-between") || r;
      out.push({
        label: (r.innerText || "").split("\n")[0],
        className: row.className,
        disabledDescendants: [...row.querySelectorAll("[disabled]")].length,
      });
    }
    return out;
  });
  writeFileSync(stage("paste-behavior-rows.json"), JSON.stringify(dimming, null, 2));
  const dimmed = dimming.filter(
    (d) => /opacity-40|pointer-events-none/.test(d.className) || d.disabledDescendants > 0,
  );
  // NON-VACUOUS: if the query matched nothing there is nothing to judge, and
  // "no dimmed rows found" would be a pass for the wrong reason.
  const sawAutoSend = dimming.some((d) => d.label === "Auto-Send");
  const sawPasteDelay = dimming.some((d) => d.label === "Paste Delay (ms)");
  if (!sawAutoSend || !sawPasteDelay) {
    fail(
      "Q4: Auto-Send / Paste Delay no longer dimmed or disabled",
      `VACUOUS — rows not found (matched ${dimming.length}); autoSend=${sawAutoSend} pasteDelay=${sawPasteDelay}`,
    );
  } else if (dimmed.length) {
    fail("Q4: Auto-Send / Paste Delay no longer dimmed or disabled", JSON.stringify(dimmed));
  } else {
    pass(
      "Q4: Auto-Send / Paste Delay no longer dimmed or disabled",
      `${dimming.length} matching node(s), 0 dimmed, 0 disabled`,
    );
  }

  // ----------------------------------------------------------------- Advanced
  // Settings detail -> settings home, then into the Advanced category. The
  // category buttons read "Advanced\nPrompts, audio, webhooks, sync".
  const backToSettingsHome = async () => {
    const clicked = await page.evaluate(() => {
      const b = document.querySelector('[aria-label="Back to settings"]');
      if (!b) return false;
      b.click();
      return true;
    });
    if (clicked) await sleep(350);
  };
  const backToAdvancedHome = async () => {
    // The back button only exists in a DETAIL view; on the home view there is
    // nothing to click and that is fine.
    const clicked = await page.evaluate(() => {
      const b = document.querySelector('[aria-label="Back to advanced settings"]');
      if (!b) return false;
      b.click();
      return true;
    });
    if (clicked) await sleep(350);
  };
  await backToSettingsHome();
  await clickByText(page, "Advanced");
  await sleep(500);

  for (const expert of [false, true]) {
    // Each pass starts from the Advanced HOME view — the previous pass ends
    // inside a detail view, where the "System" home row does not exist.
    await backToAdvancedHome();
    // The Expert mode toggle lives in the Advanced "System" section.
    await clickByText(page, "System");
    await sleep(400);
    const sysText = await textOf(page);
    // The label is "Expert mode" (lowercase m) and the control is a
    // role=switch button carrying aria-checked — read that, don't infer state
    // from CSS classes.
    if (!sysText.includes("Expert mode")) {
      fail(`Advanced/System reachable (expert=${expert})`, "'Expert mode' toggle not found");
      break;
    }
    const readSwitch = () =>
      page.evaluate(() => {
        const sw = [...document.querySelectorAll('button[role="switch"]')].find((b) => {
          const row = b.closest("div.flex.items-center.justify-between");
          return (row?.innerText || "").includes("Expert mode");
        });
        return sw ? sw.getAttribute("aria-checked") === "true" : null;
      });
    let isOn = await readSwitch();
    if (isOn === null) {
      fail(`Expert mode switch found (expert=${expert})`, "no role=switch in the Expert mode row");
      break;
    }
    if (isOn !== expert) {
      await page.evaluate(() => {
        const sw = [...document.querySelectorAll('button[role="switch"]')].find((b) => {
          const row = b.closest("div.flex.items-center.justify-between");
          return (row?.innerText || "").includes("Expert mode");
        });
        sw?.click();
      });
      await sleep(400);
      isOn = await readSwitch();
    }
    if (isOn !== expert) {
      fail(`Expert mode set to ${expert}`, `aria-checked stayed ${isOn}`);
      break;
    }
    pass(`Expert mode set to ${expert}`, "read from aria-checked on role=switch");

    // Q2: with the STT-temperature/chunking rows gone, the hint must no longer
    // promise them.
    if (sysText.includes("chunking and STT temperature")) {
      fail(`Advanced/System (expert=${expert}): expert-mode hint corrected (Q2)`, "stale hint still rendered");
    } else if (!sysText.includes("Reveals raw audio thresholds.")) {
      fail(`Advanced/System (expert=${expert}): expert-mode hint corrected (Q2)`, "new hint not found");
    } else {
      pass(`Advanced/System (expert=${expert}): expert-mode hint corrected (Q2)`);
    }

    // Walk back to the Advanced home, then through every section.
    const sections = ["Speech-to-Text", "Text Cleanup", "Audio", "System"];
    const seen = {};
    for (const section of sections) {
      await backToAdvancedHome();
      const home = await textOf(page);
      if (section === "Audio" && !home.includes("Audio")) {
        // Expected when expert mode is off: the Audio home row is hidden.
        seen[section] = "(hidden — expert mode off)";
        pass(`Advanced/Audio hidden with expert=${expert}`, "home row absent as designed");
        continue;
      }
      try {
        await clickByText(page, section);
      } catch {
        seen[section] = "(unreachable)";
        fail(`Advanced/${section} reachable (expert=${expert})`, "home row not clickable");
        continue;
      }
      await sleep(400);
      const t = await textOf(page);
      seen[section] = t;
      for (const label of DEAD_ROW_LABELS) {
        if (t.includes(label)) {
          fail(`Advanced/${section} (expert=${expert}): dead row '${label}' removed`, "still rendered");
        }
      }
      await page.screenshot({
        path: stage(`02-advanced-${section.toLowerCase().replace(/ /g, "-")}-expert-${expert}.png`),
      });
    }
    writeFileSync(
      stage(`text-advanced-expert-${expert}.json`),
      JSON.stringify(seen, null, 2),
    );

    const entered = Object.entries(seen).filter(
      ([, v]) => typeof v === "string" && !v.startsWith("("),
    );
    if (entered.length < 3) {
      fail(
        `Advanced (expert=${expert}): section walk is non-vacuous`,
        `only entered ${entered.length} section(s): ${entered.map(([k]) => k).join(", ") || "none"}`,
      );
    } else {
      pass(
        `Advanced (expert=${expert}): section walk is non-vacuous`,
        `entered ${entered.length}: ${entered.map(([k]) => k).join(", ")}`,
      );
    }
    const all = Object.values(seen).join("\n");
    const leaked = DEAD_ROW_LABELS.filter((l) => all.includes(l));
    if (leaked.length === 0) {
      pass(
        `Advanced (expert=${expert}): none of the 11 UI-visible dead rows render`,
        `sections walked: ${Object.keys(seen).join(", ")}`,
      );
    }
    const allCI = all.toLowerCase();
    const deadCopy = DEAD_COPY.filter((c) => allCI.includes(c.toLowerCase()));
    if (deadCopy.length) fail(`Advanced (expert=${expert}): stale copy removed`, deadCopy.join(" | "));
    else pass(`Advanced (expert=${expert}): stale copy removed`, `${DEAD_COPY.length} phrases checked`);

    // Model-ID inputs must still be there (Q3: flat, under "Model IDs").
    await backToAdvancedHome();
    await clickByText(page, "Text Cleanup");
    await sleep(400);
    const models = await page.evaluate(() => {
      const inputs = [...document.querySelectorAll('input[type="text"]')];
      return inputs.map((i) => ({ placeholder: i.placeholder, value: i.value }));
    });
    writeFileSync(stage(`model-inputs-expert-${expert}.json`), JSON.stringify(models, null, 2));
    const wanted = [
      "deepseek-chat",
      "gpt-4o-mini",
      "claude-haiku-4-5-20251001",
      "llama-3.3-70b-versatile",
    ];
    const missing = wanted.filter((w) => !models.some((m) => m.placeholder === w));
    if (missing.length) {
      fail(`Advanced/Text Cleanup (expert=${expert}): 4 model-ID inputs kept`, `missing: ${missing.join(", ")}`);
    } else {
      pass(`Advanced/Text Cleanup (expert=${expert}): 4 model-ID inputs kept`, wanted.join(", "));
    }
    const cleanupText = await textOf(page);
    // `uppercase` again: the title renders as "MODEL IDS".
    if (!cleanupText.toLowerCase().includes("model ids")) {
      fail(`Advanced/Text Cleanup (expert=${expert}): 'Model IDs' title (Q2/Q3)`, "title missing");
    } else {
      pass(`Advanced/Text Cleanup (expert=${expert}): 'Model IDs' title (Q2/Q3)`);
    }
    // Q3: the inputs must sit FLAT — no collapsible accordion button remains.
    if (cleanupText.toLowerCase().includes("model & parameters") ||
        cleanupText.toLowerCase().includes("custom cleanup instructions")) {
      fail(`Advanced/Text Cleanup (expert=${expert}): accordions gone (Q3)`, "an accordion header is still rendered");
    } else {
      pass(`Advanced/Text Cleanup (expert=${expert}): accordions gone (Q3)`);
    }
  }

  // Live copy present somewhere in the Advanced panel.
  await backToAdvancedHome();
  const advHome = await textOf(page);
  writeFileSync(stage("text-advanced-home.txt"), advHome);
  for (const c of LIVE_COPY) {
    // "Reveals raw audio thresholds." lives in System, not on home.
    if (c.startsWith("Reveals")) continue;
    if (!advHome.toLowerCase().includes(c.toLowerCase()))
      fail(`Advanced home: live copy '${c}' present (Q2)`, "missing");
    else pass(`Advanced home: live copy '${c}' present (Q2)`);
  }
} catch (e) {
  fail("harness", String(e && e.stack ? e.stack : e));
} finally {
  await browser.close();
}

const failed = results.filter((r) => !r.ok);
const report = {
  story: "7-9",
  kind: "desktop proxy smoke (puppeteer vs npm run preview :1422, real Chromium)",
  proves: "wiring + structure of the React tree (which rows render, in both expert-mode states)",
  does_not_prove:
    "design/pixels; the Rust backend (preview serves MOCK data — no Tauri); Android; the Windows release build; any real cleanup request or Klarvo.log line",
  not_exercised: [
    "save_advanced_settings + cleanup-provider hot-reload (backend)",
    "real config.json load of an old file (Rust test covers it)",
    "the Kotlin/Android path entirely",
    "a network cleanup call carrying an overridden model ID",
    "pixel fidelity, fonts, Windows text-scale",
  ],
  total: results.length,
  passed: results.length - failed.length,
  failed: failed.length,
  page_errors: pageErrors,
  console_errors: consoleErrors,
  console_errors_note:
    "Expected in preview mode and PRE-EXISTING (not introduced by story 7-9): " +
    "SettingsPanel.tsx's voice-command-state-changed effect calls " +
    "listen() from @tauri-apps/api/event WITHOUT the isPreviewMode guard that " +
    "tauri-commands.ts's own listen() wrapper applies, so it hits " +
    "window.__TAURI_INTERNALS__ (absent in a plain browser). Its own " +
    ".catch(console.error) handles it; React StrictMode mounts the panel twice, " +
    "hence two lines. Cannot occur in the real app. Left alone here: not mapped " +
    "to any task in this story.",
  results,
};
writeFileSync(stage("smoke-report.json"), JSON.stringify(report, null, 2));

// Only now — with the browser closed and no further page interaction — copy
// the staged artifacts into the in-repo evidence dir (see HARNESS TRAP #2).
for (const f of readdirSync(STAGE)) copyFileSync(join(STAGE, f), join(HERE, f));
console.log(`artifacts: ${readdirSync(STAGE).length} file(s) -> ${HERE}`);

for (const r of results) console.log(`${r.ok ? "ok  " : "FAIL"}  ${r.name}${r.detail ? " — " + r.detail : ""}`);
console.log(`\n${report.passed}/${report.total} checks passed, ${report.failed} failed`);
if (pageErrors.length) console.log(`UNCAUGHT page errors: ${pageErrors.length}\n${pageErrors.join("\n")}`);
else console.log("uncaught page errors: 0");
if (consoleErrors.length)
  console.log(`console.error lines (handled, pre-existing — see report): ${consoleErrors.length}\n  ${consoleErrors.join("\n  ")}`);
process.exit(failed.length ? 1 : 0);
