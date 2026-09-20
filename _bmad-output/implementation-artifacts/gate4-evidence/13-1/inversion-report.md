# Story 13-1 — desktop proxy gate (INVERSION RUN)

Run: 2026-09-20T12:56:06.635Z
Result: FAIL (1/15)

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
- [ ] **INVERSION (EXPECTED RED): idle state equals the raw <select> of the Log Level row** — differing properties (8/13): backgroundColor: rgb(27, 30, 32) != rgb(15, 17, 18) | borderTopColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderRightColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderBottomColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | borderLeftColor: rgb(40, 44, 47) != oklab(0.290349 -0.00391997 -0.00683959 / 0.6) | paddingTop: 6px != 8px | paddingBottom: 6px != 8px | boxShadow: rgba(41, 199, 172, 0.28) 0px 0px 0px 3px != none
- [x] **no page errors** — none
