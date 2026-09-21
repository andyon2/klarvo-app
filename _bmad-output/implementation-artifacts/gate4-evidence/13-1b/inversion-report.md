# Story 13-1b — desktop proxy gate (INVERSION RUN)

Run: 2026-09-21T11:10:57.601Z
Checks: 35 (derived from the result records, not hand-counted)
Ordinary checks: 20, failed: 0
Result: PASS

An inversion assertion PASSES BY GOING RED. Each group below must contain
at least one red assertion; that is what licenses the green run.
- INVERSION-1: 1/10 assertion(s) RED ✓
- INVERSION-2: 1/1 assertion(s) RED ✓
- INVERSION-3: 1/1 assertion(s) RED ✓
- INVERSION-4: 1/3 assertion(s) RED ✓

- [x] **boot** — onboarding skipped
- [x] **reference instance found** — Settings → Language carries 2 KSelect(s); using #0 ("Dictation language")
- [x] **[expert OFF] Recording & Audio picker actually read** — 3 dropdown(s), 8 option label(s) — a zero-element scan would prove nothing
- [x] **[expert OFF] no test-provider value in the normal provider picker (Recording & Audio)** — options: ["DeepSeek (no key)","OpenAI (no key)","Groq (Llama) (no key)","OpenRouter (no key)","Local (Offline)","System Default","Default Microphone","USB Headset"]
- [x] **[expert OFF] AI & Providers page rendered** — its own 'Cleanup Instructions' and 'App Profiles' sections are present — the scan below is of a real page, not an empty one
- [x] **[expert OFF] AI & Providers controls actually read** — 2 control(s), 6 option label(s) after adding a profile — a zero-element scan would prove nothing
- [x] **[expert OFF] AI & Providers carries no provider picker** — 2 control(s): [{"tag":"button","label":"Polished"},{"tag":"button","label":"Polished"}]; option labels: ["Polished","Verbatim","Chat","Auto","DE","EN"]; provider-shaped labels: []
- [x] **[expert OFF] no test-provider value in AI & Providers** — options: ["Polished","Verbatim","Chat","Auto","DE","EN"]
- [x] **both rows absent with Expert mode OFF** — absent: ["Test provider (LLM)","Test provider (STT)"]; KSelect count on the System page = 0 (expected 0)
- [x] **none of the four story-13-1 row labels render with Expert mode OFF** — found: []
- [x] **Expert-mode switch found** — role=switch count = 1
- [x] **both rows present with Expert mode ON** — labels found: ["Test provider (LLM)","Test provider (STT)"]
- [x] **exactly TWO KSelect rows added** — [{"i":0,"label":"Test provider (LLM)","text":"off"},{"i":1,"label":"Test provider (STT)","text":"off"}]
- [x] **none of the four story-13-1 row labels render with Expert mode ON** — found: [] — the provider/scenario split must be gone, not hidden
- [x] **[Test provider (LLM)] options equal the fixture, element-wise** — rendered ["off","ok","empty","truncated","malformed","http429","http5xx","transport"] vs fixture ["off","ok","empty","truncated","malformed","http429","http5xx","transport"]
- [x] **[Test provider (STT)] options equal the fixture, element-wise** — rendered ["off","ok","empty","malformed","http429","http5xx","transport"] vs fixture ["off","ok","empty","malformed","http429","http5xx","transport"]
- [x] **[desktop] no feedback FAB and no feedback tooltip anywhere** — aria-label="Send feedback": 0, title="Send Feedback": 0, tooltip text: false
- [x] **[INVERSION-1 vs raw <select>] hover VARIANT matches the reference instance (structural; computed hover not exercisable — trap #7)** — measured "hover:border-klarvo-border-2" vs reference "hover:border-klarvo-border-2"
- [ ] **[INVERSION-1 vs raw <select>] state 'idle' equals the reference instance** — backgroundColor: rgb(27, 30, 32) != rgb(15, 17, 18) | borderTopColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderRightColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderBottomColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderLeftColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | paddingTop: 6px != 8px | paddingBottom: 6px != 8px | boxShadow: rgba(41, 199, 172, 0.28) 0px 0px 0px 3px != none
- [x] **[INVERSION-1 vs raw <select>] state 'focused' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] state 'open' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] state 'listbox' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] state 'optionSelected' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] keyboard-focused option compared like-for-like** — new aria-selected=false (idx 1), reference aria-selected=false (idx 2)
- [x] **[INVERSION-1 vs raw <select>] state 'optionKeyboardFocused' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] open state: aria-expanded=true + chevron rotated** — {"ariaExpanded":"true","chevronRotated":true}
- [x] **[INVERSION-1 vs raw <select>] state 'pressed' (= open) equals the reference instance** — the shipped KSelect defines no distinct pressed styling
- [x] **[desktop] Advanced panel made dirty through the Test provider (LLM) row** — option picked: true; Save footer rendered: true
- [x] **INVERSION-4 (EXPECTED RED) [desktop, sticky stripped] the scroll container really overflows** — scrollHeight 412 - clientHeight 350 = 62px of scroll
- [ ] **INVERSION-4 (EXPECTED RED) [desktop, sticky stripped] Save button fully inside the scroll container at scrollTop=0** — button 453..495 vs container 111..461 (scrollTop=0, footer position=static)
- [x] **INVERSION-4 (EXPECTED RED) [desktop, sticky stripped] Save button fully inside the scroll container at scrollTop=max** — button 391..433 vs container 111..461 (scrollTop=62, footer position=static)
- [ ] **INVERSION-3 (EXPECTED RED): no test-provider value on Advanced → System with Expert mode ON** — found ["off","ok","empty","truncated","malformed","http429","http5xx","transport","off","ok","empty","malformed","http429","http5xx","transport"] — the scan CAN see a test-provider value, so its green verdict elsewhere means something
- [ ] **INVERSION-2 (EXPECTED RED): both rows present with Expert mode OFF** — the rows are absent, as they must be — the visibility predicate discriminates
- [x] **throwaway mock edit restored** — /home/andyon2/workspace/products/klarvo/src/tauri-commands.ts rewritten to its original bytes
- [x] **no page errors** — none

## NOT EXERCISED (neither passed nor failed — stated, per project-context
"a number states what it covers")

- HOVER NOT EXERCISED — `(hover: hover)` is false in headless Chromium (no pointing device), and Tailwind v4 wraps every `hover:` utility in that media query, so `hover:border-klarvo-border-2` cannot apply. Emulation.setEmulatedMedia, --blink-settings=…HoverType… and CSS.forcePseudoState were all measured and none changes it. The `hover` state is therefore DROPPED from the computed-style comparison rather than compared as two idle samples; a structural check that both controls carry the same hover variant class runs instead. The rendered hover colour remains Andi's real-screen gate.
