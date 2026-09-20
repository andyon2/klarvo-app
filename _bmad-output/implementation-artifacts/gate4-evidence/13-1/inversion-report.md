# Story 13-1 — desktop proxy gate (INVERSION RUN)

Run: 2026-09-20T14:25:00.285Z
Checks: 30 (derived from the result records, not hand-counted)
Ordinary checks: 18, failed: 0
Result: PASS

An inversion assertion PASSES BY GOING RED. Each group below must contain
at least one red assertion; that is what licenses the green run.
- INVERSION-1: 1/10 assertion(s) RED ✓
- INVERSION-2: 1/1 assertion(s) RED ✓
- INVERSION-3: 1/1 assertion(s) RED ✓

- [x] **boot** — onboarding skipped
- [x] **reference instance found** — Settings → Language carries 2 KSelect(s); using #0 ("Dictation language")
- [x] **[expert OFF] Recording & Audio picker actually read** — 3 dropdown(s), 8 option label(s) — a zero-element scan would prove nothing
- [x] **[expert OFF] no 'debug' option in the normal provider picker (Recording & Audio)** — options: ["DeepSeek (no key)","OpenAI (no key)","Groq (Llama) (no key)","OpenRouter (no key)","Local (Offline)","System Default","Default Microphone","USB Headset"]
- [x] **[expert OFF] AI & Providers page rendered** — its own 'Cleanup Instructions' and 'App Profiles' sections are present — the scan below is of a real page, not an empty one
- [x] **[expert OFF] AI & Providers controls actually read** — 2 control(s), 6 option label(s) after adding a profile — a zero-element scan would prove nothing
- [x] **[expert OFF] AI & Providers carries no provider picker** — 2 control(s): [{"tag":"button","label":"Polished"},{"tag":"button","label":"Polished"}]; option labels: ["Polished","Verbatim","Chat","Auto","DE","EN"]; provider-shaped labels: []
- [x] **[expert OFF] no 'debug' option in AI & Providers** — options: ["Polished","Verbatim","Chat","Auto","DE","EN"]
- [x] **all four rows absent with Expert mode OFF** — absent: ["LLM Provider","STT Provider","Debug LLM Scenario","Debug STT Scenario"]; KSelect count on the System page = 0 (expected 0)
- [x] **Expert-mode switch found** — role=switch count = 1
- [x] **all four rows present with Expert mode ON** — labels found: ["LLM Provider","STT Provider","Debug LLM Scenario","Debug STT Scenario"]
- [x] **exactly four KSelect rows added** — [{"i":0,"label":"LLM Provider","text":"deepseek"},{"i":1,"label":"STT Provider","text":"groq"},{"i":2,"label":"Debug LLM Scenario","text":"ok"},{"i":3,"label":"Debug STT Scenario","text":"ok"}]
- [x] **[LLM Provider] options equal the fixture, element-wise** — rendered ["deepseek","openai","anthropic","groq","openrouter","debug"] vs fixture ["deepseek","openai","anthropic","groq","openrouter","debug"]
- [x] **[STT Provider] options equal the fixture, element-wise** — rendered ["groq","openai","local","debug"] vs fixture ["groq","openai","local","debug"]
- [x] **[Debug LLM Scenario] options equal the fixture, element-wise** — rendered ["ok","empty","truncated","malformed","http429","http5xx","transport"] vs fixture ["ok","empty","truncated","malformed","http429","http5xx","transport"]
- [x] **[Debug STT Scenario] options equal the fixture, element-wise** — rendered ["ok","empty","malformed","http429","http5xx","transport"] vs fixture ["ok","empty","malformed","http429","http5xx","transport"]
- [ ] **[INVERSION-1 vs raw <select>] state 'idle' equals the reference instance** — backgroundColor: rgb(27, 30, 32) != rgb(15, 17, 18) | borderTopColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderRightColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderBottomColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderLeftColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | paddingTop: 6px != 8px | paddingBottom: 6px != 8px | boxShadow: rgba(41, 199, 172, 0.28) 0px 0px 0px 3px != none
- [x] **[INVERSION-1 vs raw <select>] state 'hover' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] state 'focused' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] state 'open' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] state 'listbox' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] state 'optionSelected' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] keyboard-focused option compared like-for-like** — new aria-selected=false (idx 1), reference aria-selected=false (idx 2)
- [x] **[INVERSION-1 vs raw <select>] state 'optionKeyboardFocused' equals the reference instance** — all 13 properties equal
- [x] **[INVERSION-1 vs raw <select>] open state: aria-expanded=true + chevron rotated** — {"ariaExpanded":"true","chevronRotated":true}
- [x] **[INVERSION-1 vs raw <select>] state 'pressed' (= open) equals the reference instance** — the shipped KSelect defines no distinct pressed styling
- [ ] **INVERSION-3 (EXPECTED RED): no 'debug' option on Advanced → System with Expert mode ON** — found ["debug","debug"] — the scan CAN see a debug option, so its green verdict elsewhere means something
- [ ] **INVERSION-2 (EXPECTED RED): all four rows present with Expert mode OFF** — the rows are absent, as they must be — the visibility predicate discriminates
- [x] **throwaway mock edit restored** — /home/andyon2/workspace/products/klarvo/src/tauri-commands.ts rewritten to its original bytes
- [x] **no page errors** — none
