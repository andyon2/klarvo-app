# Story 13-1b — desktop proxy gate

Run: 2026-09-21T10:27:44.729Z
Checks: 59 (derived from the result records, not hand-counted)
Ordinary checks: 59, failed: 0
Result: PASS

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
- [x] **[Test provider (LLM)] state 'idle' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (LLM)] state 'hover' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (LLM)] state 'focused' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (LLM)] state 'open' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (LLM)] state 'listbox' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (LLM)] state 'optionSelected' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (LLM)] keyboard-focused option compared like-for-like** — new aria-selected=false (idx 1), reference aria-selected=false (idx 2)
- [x] **[Test provider (LLM)] state 'optionKeyboardFocused' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (LLM)] open state: aria-expanded=true + chevron rotated** — {"ariaExpanded":"true","chevronRotated":true}
- [x] **[Test provider (LLM)] state 'pressed' (= open) equals the reference instance** — the shipped KSelect defines no distinct pressed styling
- [x] **[Test provider (STT)] state 'idle' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (STT)] state 'hover' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (STT)] state 'focused' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (STT)] state 'open' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (STT)] state 'listbox' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (STT)] state 'optionSelected' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (STT)] keyboard-focused option compared like-for-like** — new aria-selected=false (idx 1), reference aria-selected=false (idx 2)
- [x] **[Test provider (STT)] state 'optionKeyboardFocused' equals the reference instance** — all 13 properties equal
- [x] **[Test provider (STT)] open state: aria-expanded=true + chevron rotated** — {"ariaExpanded":"true","chevronRotated":true}
- [x] **[Test provider (STT)] state 'pressed' (= open) equals the reference instance** — the shipped KSelect defines no distinct pressed styling
- [x] **[desktop] Advanced panel made dirty through the Test provider (LLM) row** — option picked: true; Save footer rendered: true
- [x] **[desktop] the scroll container really overflows** — scrollHeight 412 - clientHeight 350 = 62px of scroll
- [x] **[desktop] Save button fully inside the scroll container at scrollTop=0** — button 391..433 vs container 111..461 (scrollTop=0, footer position=sticky)
- [x] **[desktop] Save button fully inside the scroll container at scrollTop=max** — button 391..433 vs container 111..461 (scrollTop=62, footer position=sticky)
- [x] **boot (after mock edit)** — onboarding skipped
- [x] **throwaway mock edit took effect (expertMode=true is loaded, not clicked)** — the four rows render without touching the switch — so the picker scan below really runs with Expert mode ON
- [x] **[expert ON] Recording & Audio picker actually read** — 3 dropdown(s), 8 option label(s) — a zero-element scan would prove nothing
- [x] **[expert ON] no test-provider value in the normal provider picker (Recording & Audio)** — options: ["DeepSeek (no key)","OpenAI (no key)","Groq (Llama) (no key)","OpenRouter (no key)","Local (Offline)","System Default","Default Microphone","USB Headset"]
- [x] **[expert ON] AI & Providers page rendered** — its own 'Cleanup Instructions' and 'App Profiles' sections are present — the scan below is of a real page, not an empty one
- [x] **[expert ON] AI & Providers controls actually read** — 2 control(s), 6 option label(s) after adding a profile — a zero-element scan would prove nothing
- [x] **[expert ON] AI & Providers carries no provider picker** — 2 control(s): [{"tag":"button","label":"Polished"},{"tag":"button","label":"Polished"}]; option labels: ["Polished","Verbatim","Chat","Auto","DE","EN"]; provider-shaped labels: []
- [x] **[expert ON] no test-provider value in AI & Providers** — options: ["Polished","Verbatim","Chat","Auto","DE","EN"]
- [x] **boot (phone viewport)** — onboarding skipped
- [x] **[phone] the mobile layout branch is the one under test** — navigator.userAgent reports Android = true; src/platform.ts keys isMobile on exactly this
- [x] **[phone] no feedback FAB and no feedback tooltip anywhere** — aria-label="Send feedback": 0, title="Send Feedback": 0, tooltip text: false
- [x] **[phone] both rows render on the mobile layout branch** — labels found: ["Test provider (LLM)","Test provider (STT)"]
- [x] **[phone] Advanced panel made dirty through the Test provider (LLM) row** — option picked: true; Save footer rendered: true
- [x] **[phone] the scroll container really overflows** — scrollHeight 565 - clientHeight 442 = 123px of scroll
- [x] **[phone] Save button fully inside the scroll container at scrollTop=0** — button 443..485 vs container 115..557 (scrollTop=0, footer position=sticky)
- [x] **[phone] Save button fully inside the scroll container at scrollTop=max** — button 443..485 vs container 115..557 (scrollTop=123, footer position=sticky)
- [x] **throwaway mock edit restored** — /home/andyon2/workspace/products/klarvo/src/tauri-commands.ts rewritten to its original bytes
- [x] **no page errors** — none
