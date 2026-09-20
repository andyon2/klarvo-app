# Story 13-1 — desktop proxy gate

Run: 2026-09-20T12:55:51.401Z
Result: PASS

- [x] **onboarding skipped** — clicked 'Setup überspringen'
- [x] **reference instance found** — Settings → Language carries 2 KSelect(s); using #0 ("Dictation language")
- [x] **no 'debug' option in Recording & Audio** — 3 dropdown(s), 8 option label(s): ["DeepSeek (no key)","OpenAI (no key)","Groq (Llama) (no key)","OpenRouter (no key)","Local (Offline)","System Default","Default Microphone","USB Headset"]
- [x] **no standalone 'debug' text in Recording & Audio** — body scan
- [x] **no 'debug' option in AI & Providers** — 0 dropdown(s), 0 option label(s): [] — this page carries no picker in preview (AI & Providers' KSelects live inside per-profile rows, and the mock has no profiles), so the claim rests on Recording & Audio
- [x] **no standalone 'debug' text in AI & Providers** — body scan
- [x] **provider-picker options were actually read** — 8 option label(s) read across the two picker pages
- [x] **rows absent with Expert mode OFF** — KSelect count on the System page = 0 (expected 0)
- [x] **Expert-mode switch found** — role=switch count = 1
- [x] **rows present with Expert mode ON** — KSelect count on the System page = 2 (expected 2)
- [x] **exactly two KSelect rows added** — [{"i":0,"label":"Debug LLM Scenario","text":"ok"},{"i":1,"label":"Debug STT Scenario","text":"ok"}]
- [x] **LLM scenario options are the seven strings, verbatim** — ["ok","empty","truncated","malformed","http429","http5xx","transport"]
- [x] **STT scenario options are the LLM set minus 'truncated'** — ["ok","empty","malformed","http429","http5xx","transport"]
- [x] **state 'idle' equals the reference instance** — all 13 properties equal
- [x] **state 'hover' equals the reference instance** — all 13 properties equal
- [x] **state 'focused' equals the reference instance** — all 13 properties equal
- [x] **state 'open' equals the reference instance** — all 13 properties equal
- [x] **state 'listbox' equals the reference instance** — all 13 properties equal
- [x] **state 'optionSelected' equals the reference instance** — all 13 properties equal
- [x] **keyboard-focused option compared like-for-like** — new aria-selected=false (idx 1), reference aria-selected=false (idx 2)
- [x] **state 'optionKeyboardFocused' equals the reference instance** — all 13 properties equal
- [x] **open state: aria-expanded=true + chevron rotated** — {"ariaExpanded":"true","chevronRotated":true}
- [x] **state 'pressed' (= open) equals the reference instance** — the shipped KSelect defines no distinct pressed styling
- [x] **no page errors** — none
