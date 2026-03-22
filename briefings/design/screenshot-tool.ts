/**
 * Automated screenshot tool for Klarvo theme preview.
 *
 * Takes screenshots of 3 views (main, settings, stats) for a given theme.
 * Requires: npm run preview running on localhost:1422
 *
 * Usage:
 *   npx tsx briefings/design/screenshot-tool.ts <theme-id>
 *   npx tsx briefings/design/screenshot-tool.ts notion-v2
 *   npx tsx briefings/design/screenshot-tool.ts all
 */

import puppeteer from "puppeteer";
import path from "path";
import fs from "fs";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PREVIEW_URL = "http://localhost:1422";
const SCREENSHOT_DIR = path.resolve(__dirname, "screenshots");
const VIEWPORT = { width: 1280, height: 800 };

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function expandSwitcherAndClickTheme(page: puppeteer.Page, themeId: string): Promise<boolean> {
  // Step 1: Expand the switcher (buttons are conditionally rendered)
  await page.evaluate(() => {
    const switcher = document.querySelector('[aria-label="Theme switcher (preview only)"]');
    if (!switcher) return;
    const toggleBtn = switcher.querySelector("button");
    if (toggleBtn) toggleBtn.click();
  });
  await sleep(300); // Wait for React re-render

  // Step 2: Find and click the theme button
  // Match by: exact name, partial name, or theme-id substring
  return page.evaluate((id) => {
    const needle = id.replace(/-/g, " ").toLowerCase();
    // Also try just the last part: "notion-v2a" → "v2a"
    const shortNeedle = needle.split(" ").pop() ?? needle;
    const allButtons = document.querySelectorAll("button[aria-pressed]");
    for (const btn of allButtons) {
      const text = btn.textContent?.trim().toLowerCase() ?? "";
      if (text === needle || text.includes(needle) || text.includes(shortNeedle)) {
        (btn as HTMLElement).click();
        return true;
      }
    }
    return false;
  }, themeId);
}

async function captureTheme(themeId: string) {
  console.log(`\n=== Capturing theme: ${themeId} ===`);

  const browser = await puppeteer.launch({
    headless: true,
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });

  const page = await browser.newPage();
  await page.setViewport(VIEWPORT);

  const views = [
    { id: "main", ariaLabel: null },
    { id: "settings", ariaLabel: "Toggle settings" },
    { id: "stats", ariaLabel: "Toggle stats" },
  ];

  for (const view of views) {
    // Fresh load for each view to avoid panel state leaking
    await page.goto(PREVIEW_URL, { waitUntil: "networkidle0", timeout: 10000 });
    await sleep(800);

    // Apply theme
    const applied = await expandSwitcherAndClickTheme(page, themeId);
    if (!applied) {
      console.error(`  Theme "${themeId}" not found!`);
      break;
    }
    await sleep(400);

    // Open the panel if needed
    if (view.ariaLabel) {
      const btn = await page.$(`button[aria-label="${view.ariaLabel}"]`);
      if (btn) {
        await btn.click();
        await sleep(500);
      }
    }

    // Hide the theme switcher for a clean screenshot
    await page.evaluate(() => {
      const switcher = document.querySelector('[aria-label="Theme switcher (preview only)"]');
      if (switcher) (switcher as HTMLElement).style.display = "none";
      // Also hide the "Preview Mode" badge
      const badges = document.querySelectorAll('[class*="preview"], [class*="Preview"]');
      badges.forEach((b) => ((b as HTMLElement).style.display = "none"));
    });
    await sleep(100);

    // Take screenshot
    const filename = `${themeId}_${view.id}.png`;
    const filepath = path.join(SCREENSHOT_DIR, filename);
    await page.screenshot({ path: filepath, fullPage: false });
    console.log(`  Saved: ${filename}`);
  }

  await browser.close();
  console.log(`  Done: ${themeId}`);
}

async function main() {
  const themeId = process.argv[2];

  if (!themeId) {
    console.log("Usage: npx tsx briefings/design/screenshot-tool.ts <theme-id>");
    console.log("       npx tsx briefings/design/screenshot-tool.ts notion-v2");
    console.log("       npx tsx briefings/design/screenshot-tool.ts all");
    process.exit(1);
  }

  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });

  // Check preview is running
  try {
    const res = await fetch(PREVIEW_URL);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
  } catch {
    console.error("Preview server not running! Start with: npm run preview");
    process.exit(1);
  }

  if (themeId === "all") {
    for (const t of ["current", "obsidian", "carbon", "notion-warm", "notion-v2"]) {
      await captureTheme(t);
    }
  } else {
    await captureTheme(themeId);
  }
}

main().catch(console.error);
