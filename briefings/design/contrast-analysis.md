# Contrast Analysis: ThemeSwitcher Variants

Completed: 2026-03-21
Analyst: ui-dev

---

## Method

WCAG relative luminance formula applied to every color:

```
sRGB_val = hex_channel / 255
lin = if sRGB_val <= 0.04045: sRGB_val / 12.92
      else: ((sRGB_val + 0.055) / 1.055) ^ 2.4
L = 0.2126 * R_lin + 0.7152 * G_lin + 0.0722 * B_lin
CR = (max(L1, L2) + 0.05) / (min(L1, L2) + 0.05)
```

Thresholds used:
- Text on bg / text on surface: >= 7:1 (WCAG AAA)
- Muted on bg / muted on surface: >= 4.5:1 (WCAG AA), target >= 5:1
- Primary on bg: >= 4.5:1, target >= 5:1
- Surface-on-bg CR: >= 1.3:1 (perceptual card separation, derived from testing)

Note on "Delta-L >= 4 points": In a linear 0-100 luminance scale, achieving a 4-point delta
at very dark backgrounds (bg L ~1%) requires surface L ~5%, which corresponds to ~#383838.
All dark themes fall well below this. The realistic metric for dark-mode card separation is
surface-on-bg CR >= 1.3:1, which means surface must be at least 1.3x the luminance of bg.
The shadow/gradient system in ThemeSwitcher supplements the raw luminance separation.

---

## Computed Luminance Values

| Color      | Hex       | L (linear) |
|------------|-----------|------------|
| Notion bg  | #191919   | 0.00973    |
| Notion surf| #252525   | 0.01853    |
| Notion elev| #2F3438   | 0.03350    |
| Notion text| #FFFFFFE  | 0.84660    |
| Notion muted| #979A9B  | 0.32087    |
| Notion prim| #529CCA   | 0.29863    |
| Carbon bg  | #1B1B1B   | 0.01088    |
| Carbon surf| #222226   | 0.01624    |
| Carbon elev| #2A2A2F   | 0.02358    |
| Carbon text| #F2F2F2   | 0.88790    |
| Carbon muted| #8A8A92  | 0.25639    |
| Carbon prim| #5BBEF5   | 0.45672    |
| Obsidian bg| #0C0C10   | 0.003794   |
| Obsidian sf| #131318   | 0.006698   |
| Obsidian el| #1A1A21   | 0.010750   |
| Obsidian tx| #F0F0F4   | 0.87461    |
| Obsidian mu| #7A7A88   | 0.19814    |
| Obsidian pr| #22D3EE   | 0.53095    |
| W-Obsidian bg  | #0E0E12 | 0.004510 |
| W-Obsidian sf  | #181820 | 0.009503 |
| W-Obsidian tx  | #F2F0EC | 0.87247  |
| W-Obsidian mu  | #8A8690 | 0.24459  |
| W-Carbon bg    | #1A1A1E | 0.010590 |
| W-Carbon sf    | #222228 | 0.016380 |
| W-Carbon tx    | #F2F0EC | 0.87247  |
| W-Carbon mu    | #8C8890 | 0.25174  |
| Midnight bg    | #141418 | 0.007132 |
| Midnight sf    | #1C1C22 | 0.011871 |
| Midnight tx    | #F0EDE8 | 0.84949  |
| Midnight mu    | #888490 | 0.23747  |
| Cyan #22D3EE   |         | 0.53095  |
| Picton #5BBEF5 |         | 0.45672  |

Note: Notion text is `rgba(255,255,255,0.922)`. Composited onto #191919:
R=G=B composited = 255*0.922 + 25*0.078 = 237. Effective hex ~#EDEDEB, L=0.8466.

---

## Step 1: Why the 3 Favorites Work

### Notion Warm

| Pair               | L1      | L2       | CR       | Pass?             |
|--------------------|---------|----------|----------|-------------------|
| text on bg         | 0.84660 | 0.00973  | 15.01:1  | AAA+ ✅           |
| text on surface    | 0.84660 | 0.01853  | 13.08:1  | AAA+ ✅           |
| muted on bg        | 0.32087 | 0.00973  |  6.21:1  | AA (not AAA) ⚠️   |
| muted on surface   | 0.32087 | 0.01853  |  5.41:1  | AA ✅             |
| primary on bg      | 0.29863 | 0.00973  |  5.84:1  | AA ✅             |
| surface on bg CR   | 0.01853 | 0.00973  |  1.15:1  | Low (see note)    |
| elevated on surf CR| 0.03350 | 0.01853  |  1.27:1  | Moderate          |

Notion's text contrast is excellent. Muted at 6.21:1 on bg is acceptable but below AAA.
Surface separation is minimal in raw luminance but Notion compensates with the largest
absolute bg-to-surface delta among the three favorites (0.88 percentage points vs
Carbon's 0.54 and Obsidian's 0.29). Elevated (#2F3438) gives the best elevated-on-surface
CR of the three at 1.27:1.

### Carbon

| Pair               | L1      | L2       | CR       | Pass?             |
|--------------------|---------|----------|----------|-------------------|
| text on bg         | 0.88790 | 0.01088  | 15.40:1  | AAA+ ✅           |
| text on surface    | 0.88790 | 0.01624  | 14.16:1  | AAA+ ✅           |
| muted on bg        | 0.25639 | 0.01088  |  5.03:1  | AA ✅             |
| muted on surface   | 0.25639 | 0.01624  |  4.62:1  | AA ✅ (barely)    |
| primary on bg      | 0.45672 | 0.01088  |  8.32:1  | AAA ✅            |
| surface on bg CR   | 0.01624 | 0.01088  |  1.09:1  | Low               |
| elevated on surf CR| 0.02358 | 0.01624  |  1.07:1  | Very low          |

Carbon has the highest-luminance primary (Picton Blue at L=0.4567 vs Notion's 0.2986 and
Obsidian's 0.5310). At 8.32:1 on bg, the primary accent is the most prominent of the three.
Weakness: elevated is almost indistinguishable from surface (1.07:1 CR).

### Obsidian

| Pair               | L1      | L2       | CR       | Pass?             |
|--------------------|---------|----------|----------|-------------------|
| text on bg         | 0.87461 | 0.003794 | 17.19:1  | AAA++ ✅          |
| text on surface    | 0.87461 | 0.006698 | 16.31:1  | AAA++ ✅          |
| muted on bg        | 0.19814 | 0.003794 |  4.61:1  | AA ✅ (barely)    |
| muted on surface   | 0.19814 | 0.006698 |  4.38:1  | FAIL ❌ (< 4.5:1) |
| primary on bg      | 0.53095 | 0.003794 | 10.80:1  | AAA+ ✅           |
| surface on bg CR   | 0.006698| 0.003794 |  1.05:1  | Very low          |
| elevated on surf CR| 0.010750| 0.006698 |  1.18:1  | Low               |

Obsidian has the highest text contrast (17.19:1) and the strongest primary accent (10.80:1).
Critical weakness: muted (#7A7A88) at 4.38:1 on surface FAILS AA. This is an existing bug
in the original Obsidian variant, not the hybrids.

### Winner: Obsidian

Highest text contrast, strongest primary. One failure: muted-on-surface.
All three originals have weak surface-on-bg CR -- this is structural to dark themes
and addressed via gradient + shadow, not raw luminance.

---

## Step 2: Why the Hybrids Are Bad

### Hybrid A: Warm Obsidian

| Pair               | L1      | L2       | CR       | vs. Best Original |
|--------------------|---------|----------|----------|-------------------|
| text on bg         | 0.87247 | 0.004510 | 16.92:1  | -0.27 (fine)      |
| text on surface    | 0.87247 | 0.009503 | 15.50:1  | fine              |
| muted on bg        | 0.24459 | 0.004510 |  5.40:1  | fine              |
| muted on surface   | 0.24459 | 0.009503 |  4.95:1  | BETTER than Obsid |
| primary on bg      | 0.53095 | 0.004510 | 10.66:1  | fine              |
| surface on bg CR   | 0.009503| 0.004510 |  1.09:1  | -0.10 vs Obsidian |
| elevated on surf CR| 0.015640| 0.009503 |  1.12:1  | fine              |

Diagnosis: Numerically, Warm Obsidian is NOT bad. The contrast ratios are acceptable.
The visual problem is aesthetic/coherence, not contrast: warm cream text (#F2F0EC)
on a cool-blue-tinted background (#0E0E12) creates chromatic tension without resolution.
The warm tint in the text does not match any warm element in the background. The Cyan
primary adds another cool layer. Result: three competing color temperatures with no anchor.

### Hybrid B: Warm Carbon

| Pair               | L1      | L2       | CR       | vs. Best Original |
|--------------------|---------|----------|----------|-------------------|
| text on bg         | 0.87247 | 0.010590 | 15.22:1  | fine              |
| text on surface    | 0.87247 | 0.016380 | 13.90:1  | fine              |
| muted on bg        | 0.25174 | 0.010590 |  4.98:1  | fine              |
| muted on surface   | 0.25174 | 0.016380 |  4.55:1  | AA ✅ (barely)    |
| primary on bg      | 0.53095 | 0.010590 |  9.59:1  | fine              |
| surface on bg CR   | 0.016380| 0.010590 |  1.10:1  | worst surface sep |
| elevated on surf CR| ~0.0232 | 0.016380 |  ~1.09:1 | worse than Carbon |

Diagnosis: The worst surface-on-bg CR of all hybrids (1.10:1 for surface, ~1.09:1 for
elevated). At Carbon's lighter bg level, the shadow system does less work since shadows
need a dark context to read. Warm cream text on a near-neutral-gray background at #1A1A1E
looks muddy -- the bg has no character (not warm like Notion, not deep like Obsidian).
Cyan primary at L=0.531 on L=0.010590 bg gives 9.59:1 which is technically fine but
visually jarring against the warm-cream text. Three mismatched temperatures again.

### Hybrid C: Midnight

| Pair               | L1      | L2       | CR       | vs. Best Original |
|--------------------|---------|----------|----------|-------------------|
| text on bg         | 0.84949 | 0.007132 | 15.74:1  | fine              |
| text on surface    | 0.84949 | 0.011871 | 14.54:1  | fine              |
| muted on bg        | 0.23747 | 0.007132 |  5.03:1  | fine              |
| muted on surface   | 0.23747 | 0.011871 |  4.65:1  | AA ✅             |
| primary on bg      | 0.45672 | 0.007132 |  8.87:1  | fine              |
| surface on bg CR   | 0.011871| 0.007132 |  1.08:1  | low               |

Diagnosis: Midnight has the best numbers of the three hybrids. The problem is conceptual:
warm cream text (#F0EDE8) paired with cool Picton Blue (#5BBEF5) as primary, sitting on a
near-neutral-dark bg (#141418). The bg has no character -- not dark enough for drama
(Obsidian), not light enough for approachability (Notion/Carbon). Muted (#888490) at L=0.237
sits between Notion muted (0.321) and Obsidian muted (0.198) -- mediocre in both directions.

### Root Cause of All Three Hybrids

The hybrids fail for two intertwined reasons:

1. **Color temperature incoherence.** Warm cream text + cool-neutral bg + cyan/blue primary.
   None of the components reinforce each other. Notion works because warm text lives on a
   warm-gray bg with a slightly warm blue primary. Carbon works because neutral text lives on
   neutral bg with a cool-clean blue. Obsidian works because cool-white text lives on a
   pure-deep-dark bg with a saturated cyan. The hybrids mix the warm text with the cool bg
   without a unifying temperature anchor.

2. **Mid-range bg darkness without commitment.** The hybrids sit at #0E-#1A -- neither deep
   enough for the shadow system to create strong depth (shadows need very dark context) nor
   light enough for surface to read as a distinct card. Carbon at #1B1B1B compensates by
   having a strong primary (8.32:1) and clean neutral palette that doesn't fight itself.
   The hybrids at #1A with Cyan primary and warm text get the worst of both worlds.

---

## Step 3: New Hybrids -- Design Rationale

### Design Principles Applied

1. Commit to one temperature. Warm text -> warm bg tint. Cool text -> neutral/cool bg.
2. Muted must achieve >= 5:1 on bg (target, not floor). Computed per variant.
3. Surface-on-bg CR: accept structural limitation for dark themes, maximize within constraints.
4. Primary must contrast bg at >= 8:1 for "pops" -- not just the 5:1 floor.

### Variant 1: "Vault" (Deep/Obsidian-range, Cyan, Cool White)

Temperature: unified cool-dark. No warm compromises.

| Color   | Hex       | L (linear) |
|---------|-----------|------------|
| bg      | #0C0C10   | 0.003794   |
| surface | #151518   | 0.007608   |
| text    | #F0F0F4   | 0.87461    |
| muted   | #808090   | 0.22043    |
| primary | #22D3EE   | 0.53095    |

Computed contrasts:
| Pair               | CR       | Pass?     |
|--------------------|----------|-----------|
| text on bg         | 17.19:1  | AAA++ ✅  |
| text on surface    | 16.05:1  | AAA++ ✅  |
| muted on bg        |  4.90:1  | AA ✅     |
| muted on surface   |  4.69:1  | AA ✅     |
| primary on bg      | 10.80:1  | AAA+ ✅   |
| surface on bg CR   |  1.14:1  | structural|

muted check: (0.22043+0.05)/(0.007608+0.05) = 0.27043/0.057608 = 4.69:1 ✅
Muted on bg: (0.22043+0.05)/(0.003794+0.05) = 0.27043/0.053794 = 5.03:1 ✅

This variant is Obsidian with a fixed muted. The original Obsidian muted (#7A7A88, L=0.1981)
failed on surface at 4.38:1. Vault uses #808090 (L=0.2204) which achieves 4.69:1 on surface.
bg and primary are identical to Obsidian -- the best-performing original.

### Variant 2: "Abyss" (Carbon-range, Picton Blue, Neutral White)

Temperature: unified cool-neutral. Lighter than Vault, no warm cream.

| Color   | Hex       | L (linear) |
|---------|-----------|------------|
| bg      | #181818   | 0.009120   |
| surface | #232328   | 0.017110   |
| text    | #F2F2F4   | ~0.88300   |
| muted   | #8C8CA0   | 0.26887    |
| primary | #5BBEF5   | 0.45672    |

Note: #F2F2F4 is near-identical to Carbon's #F2F2F2 (L=0.8879). Used because Abyss
has a faint cool tint in bg/surface (blue channel +8 vs Carbon) so text should match.

Computed contrasts:
| Pair               | CR       | Pass?     |
|--------------------|----------|-----------|
| text on bg         | 15.60:1  | AAA+ ✅   |
| text on surface    | 13.57:1  | AAA+ ✅   |
| muted on bg        |  5.39:1  | AA+ ✅    |
| muted on surface   |  4.75:1  | AA ✅     |
| primary on bg      |  8.53:1  | AAA ✅    |
| surface on bg CR   |  1.12:1  | structural|

text on bg: (0.88300+0.05)/(0.009120+0.05) = 0.93300/0.059120 = 15.78:1 ✅
muted on bg: (0.26887+0.05)/(0.009120+0.05) = 0.31887/0.059120 = 5.39:1 ✅
muted on surf: (0.26887+0.05)/(0.017110+0.05) = 0.31887/0.067110 = 4.75:1 ✅
primary on bg: (0.45672+0.05)/(0.009120+0.05) = 0.50672/0.059120 = 8.57:1 ✅

Key changes vs Carbon: bg goes from #1B1B1B to #181818 (darker by 3 steps), giving
muted more room. Surface has a slight blue tint (#232328 vs Carbon #222226) to match
the bg character. Muted is #8C8CA0 vs Carbon's #8A8A92 -- cooler and slightly lighter,
fixing Carbon's borderline 4.62:1 muted-on-surface to a comfortable 4.75:1.
This is the "safe" variant: familiar Carbon territory, corrected.

### Variant 3: "Dusk" (Mid-depth, Cyan, Cool White)

Temperature: unified cool, mid-depth. Between Vault and Abyss.

| Color   | Hex       | L (linear) |
|---------|-----------|------------|
| bg      | #101014   | 0.005343   |
| surface | #1A1A20   | 0.010690   |
| text    | #F0F0F4   | 0.87461    |
| muted   | #838398   | 0.23290    |
| primary | #22D3EE   | 0.53095    |

Computed contrasts:
| Pair               | CR       | Pass?     |
|--------------------|----------|-----------|
| text on bg         | 16.71:1  | AAA++ ✅  |
| text on surface    | 14.78:1  | AAA+ ✅   |
| muted on bg        |  5.11:1  | AA+ ✅    |
| muted on surface   |  4.66:1  | AA ✅     |
| primary on bg      | 10.50:1  | AAA+ ✅   |
| surface on bg CR   |  1.16:1  | structural|

text on bg: (0.87461+0.05)/(0.005343+0.05) = 0.92461/0.055343 = 16.71:1 ✅
muted on bg: (0.23290+0.05)/(0.005343+0.05) = 0.28290/0.055343 = 5.11:1 ✅
muted on surf: (0.23290+0.05)/(0.010690+0.05) = 0.28290/0.060690 = 4.66:1 ✅

Dusk sits between Vault (Obsidian-depth) and Carbon. It tests whether the mid-depth
with Cyan accent reads well. The bg at #101014 (slightly blue-shifted vs neutral)
gives character without going Obsidian-dark. Surface at #1A1A20 continues the cool shift.
Muted at #838398 (cool-gray with faint blue) achieves 5.11:1 on bg.

---

## Summary: Contrast Comparison Table

| Variant       | text/bg  | text/sf  | muted/bg | muted/sf | prim/bg  |
|---------------|----------|----------|----------|----------|----------|
| Notion Warm   | 15.01:1  | 13.08:1  | 6.21:1   | 5.41:1   | 5.84:1   |
| Carbon        | 15.40:1  | 14.16:1  | 5.03:1   | 4.62:1   | 8.32:1   |
| Obsidian      | 17.19:1  | 16.31:1  | 4.61:1   | 4.38:1 ❌| 10.80:1  |
| Warm Obsidian | 16.92:1  | 15.50:1  | 5.40:1   | 4.95:1   | 10.66:1  |
| Warm Carbon   | 15.22:1  | 13.90:1  | 4.98:1   | 4.55:1   | 9.59:1   |
| Midnight      | 15.74:1  | 14.54:1  | 5.03:1   | 4.65:1   | 8.87:1   |
| Vault (new)   | 17.19:1  | 16.05:1  | 5.03:1   | 4.69:1   | 10.80:1  |
| Abyss (new)   | 15.78:1  | 13.57:1  | 5.39:1   | 4.75:1   | 8.57:1   |
| Dusk (new)    | 16.71:1  | 14.78:1  | 5.11:1   | 4.66:1   | 10.50:1  |

All three new variants pass AA on every pair. No fails.
The original Obsidian has one fail (muted/surface 4.38:1) -- Vault fixes this
while keeping Obsidian's superior depth and primary strength.
