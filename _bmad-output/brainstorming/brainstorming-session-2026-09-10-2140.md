---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - docs/backlog.md (DECIDED M12 + BRAINSTORM-CANDIDATE, 2026-09-10)
  - src-tauri/src/stt/mod.rs (build_stt_prompt_with_hint)
  - src-tauri/src/dictionary/mod.rs
  - android/kotlin-src/com/klarvo/voice/KlarvoApi.kt (appendPromptExtensions)
  - src/components/settings/DictionaryContent.tsx
session_topic: 'Dictionary rework (reliability, sync, mechanism) + an always-available "smart" add-on layer on top of the three cleanup styles + a learning dictionary fed from History'
session_goals: 'Understand the current dictionary mechanism critically; generate many candidate designs for (a) a more reliable and synced dictionary, (b) a structure/intent-aware add-on (lists, mid-speech retractions, consolidation of loose thoughts), (c) a low-threshold learning loop from History corrections. Think and decide only — no building in this session.'
selected_approach: 'ai-recommended'
techniques_used: ['Five Whys', 'Assumption Reversal', 'Analogical Thinking']
ideas_generated: ['45 ideas in 7 themes: T1 Enforcement, T2 Learning loop, T3 Profile model, T4 Sync, T5 Structure & retraction, T6 Context, T7 Business']
context_file: ''
status: complete
decisions: ['D1 rule switches on the 3 styles', 'D2 user-owned storage, several sync options, no Klarvo server', 'D3 both capture paths, diagnosis on demand', 'D4 order T1>T3>T2>T4, T5 separate epic']
---

# Brainstorming Session Results

**Facilitator:** Claude (BMAD brainstorming skill)
**Participant:** Andi
**Date:** 2026-09-10 (session started 21:40, continued past midnight into 2026-09-11)

## Session Overview

**Topic:** Dictionary rework + a "smart" add-on layer for the cleanup styles + a learning dictionary.
**Goals:** Critical, thorough exploration of how the dictionary should really work; candidate designs for an add-on that adds structure and intent understanding on top of Polished / Verbatim / Chat; a learning loop that Andi can feed with low effort. Decide only — building is a separate, explicitly released act.

### Context Guidance

**Current state (verified in code, 2026-09-10):**
- Input: flat list of words/phrases in Settings ("Add word or phrase..."); free tier capped at 20 terms, licensed unlimited.
- Storage: `dictionary.json` in the app-data dir; Desktop and Android each keep their own file; no sync.
- Use site 1 — STT (Whisper via Groq, shared Rust core on both platforms): a language hint sentence + all terms comma-joined are sent as the Whisper `prompt` (hard cap ~224 tokens). It is a bias hint, not a rule.
- Use site 2 — Cleanup (DeepSeek primary): the system prompt gets one sentence, "The user's custom dictionary terms (preserve these exactly): {terms}". Desktop Chat style omits it today; M12 decided 2026-09-10 that Chat includes it (Story 7-6, not started).
- Not available today: replacement rules (spoken X -> written Y), per-term meaning/context, casing rules, snippets, categories, priority order, cross-device sync.

### Session Setup

**Andi's raw input, structured (Part 1 — what is wrong with the dictionary today):**
- **I1 — Unreliable.** Terms are in the dictionary and still come out wrong. Flagship example: "Claude" (the AI model) is transcribed as English "Cloud" although "Claude" is a dictionary entry. Same failure reported for many other terms.
- **I2 — Local-only.** Dictionary is stored per device (Android and Windows separately). Andi does not want that. Wanted: one synced dictionary, available everywhere.
- **I3 — Mechanism is opaque.** Andi does not know exactly where and how the dictionary is applied. The two use sites (Whisper prompt, cleanup prompt) must be walked through again, and Andi doubts the current approach is the smartest one. Andi sees this as a professional field of its own ("so etwas professionell aufzuziehen und zu optimieren").

**Andi's raw input, structured (Part 2 — the new function):**
- **W1 — An always-addable function** (an add-on that can be switched on with any of the three styles, or possibly a fourth style). Two aspects:
  - **W1a — Structure from speech.** Spoken "erstens, zweitens, drittens" should become a real formatted list, as Google's and Apple's new dictation does. Today Klarvo writes the words out and does nothing with them: there is no meta-analysis of what was said with respect to format or structure.
  - **W1b — Mid-speech retraction and consolidation.** As in a new Samsung/Google feature: say mid-way "everything so far was irrelevant, I actually mean ..." and the AI drops the earlier part on its own. Klarvo today sticks very closely to what was said; for think-aloud dictation (loose, unconnected thoughts) that is not the best output form.
- **W2 — A learning dictionary with low-threshold feeding.** From History: open the entry, mark a word or a whole sentence, right-click "Korrektur/Anpassung", state "this should have been X". The dictionary is then maintained from that. The system should check whether the term already exists, and diagnose the actual cause (dictionary gap vs. something else). Target: a self-optimising, learning system that understands Andi's language and terms better over time, with ever fewer misunderstandings.

**Facilitator's first reading (to be challenged in the session):** I1 and W2 are one loop (reliability -> correction -> learning). I2 is an infrastructure question (where does the truth live?). W1a/W1b are not dictionary at all; they are cleanup intelligence — a candidate for the add-on layer. I3 asks for a design walk-through before any of this is built.

## Technique Selection

**Approach:** AI-Recommended Techniques
**Analysis Context:** Dictionary rework + smart add-on layer + learning loop, with the explicit goal of a critical, thorough exploration before any building.

**Recommended Techniques:**

- **Five Whys (deep):** Drill from the flagship symptom ("Claude" -> "Cloud" despite a dictionary entry) to the root cause, walking through every point in the pipeline where a term can be lost. Covers I1 and I3 (mechanism walk-through).
- **Assumption Reversal (deep):** List the silent assumptions of today's dictionary ("a dictionary is a word list", "the user maintains it by hand", "it belongs to one device", "a style is a mode") and flip each one. Covers I2, W2 and the fourth-style-vs-add-on question.
- **Analogical Thinking (creative):** Andi cited Google, Apple and Samsung dictation. Examine how they and other dictation tools solve structure, retraction and learning; decide what transfers to Klarvo. Covers W1a, W1b and the learning loop.

**AI Rationale:** Andi's language is analytical and critical, the topic is a complex mechanism the participant wants to understand first, and the session has three distinct fields. A deep-category opener (root cause) gives the shared understanding the reversal step needs; the analogy step then feeds the add-on field with concrete outside patterns. Theatrical and wild techniques were left out on purpose: the participant asked for rigour, not play.

## Phase 1 — Five Whys: "Claude" -> "Cloud"

**Chain (V = verified in code, S = synthesized):**
1. Why does "Cloud" reach the output? Whisper emits "Cloud"; cleanup leaves it. (V: the dictionary touches only these two sites.)
2. Why does Whisper emit "Cloud" despite the hint? The Whisper `prompt` is treated as preceding text, not as a rule; a comma list without sentence context shifts probability only slightly; "Cloud" is a very frequent word in German tech speech and nearly homophonous; the default DE hint even primes "occasional English technical terms". (V: hint construction. S: the weighting.)
3. Why does cleanup not repair it? The dictionary sentence is "preserve these exactly" — a protect rule, not a repair rule; with "Cloud" in the text there is nothing to preserve. Verbatim also says both "Fix obvious transcription errors" and "NEVER substitute words" — the model picks the cautious rule. (V.)
4. Why does the dictionary carry only the word, not sound or context? Data model is a flat `Vec<String>`; no field for "sounds like", "never write as", meaning. (V.)
5. Why a flat list? The module was built as a feeder for the Whisper prompt (its own doc comment says so). Target was "inject terms", never "term reliably appears in output". (V.)

**Root:** No pipeline stage owns "dictionary terms win". No return signal whether a term arrived correctly.

**Side findings:** all three styles already resolve small mid-speech corrections ("morgen, nein Freitag") — W1b is the large-scale variant. Verbatim explicitly forbids lists/line breaks; Polished allows lists when the speaker clearly enumerates — W1a fails in Verbatim by style rule, not by model capability.

**Andi's observations (2026-09-11):**
- The failure is intermittent. It depends on articulation precision, speaking speed and position in the sentence. -> The Whisper bias is soft; a deterministic fix must sit at a stage that sees the sentence context.
- Andi has no example list because collecting examples is too cumbersome today (notice -> open settings -> type). The moment of noticing is lost. -> The feature must capture at the moment of noticing, with one gesture.
- New insight from Andi: a correction should not only add a word. In some or all cases it needs an AI-assisted discussion of the cause: dictionary gap, Whisper settings, speech itself. "More words in the dictionary" is not always the right answer.

### Ideas — Phase 1

**[Reliability #1]: Repair rule instead of protect rule**
_Concept_: The cleanup dictionary sentence adds: "If a word in the text sounds like one of these terms, write the term." One sentence, no rebuild.
_Novelty_: Today the dictionary only protects, never corrects.

**[Reliability #2]: Hint as a sentence, not a list**
_Concept_: Whisper receives a natural sentence containing the terms ("Ich arbeite mit Claude, Kubernetes und Klarvo.") instead of a comma list.
_Novelty_: Exploits that Whisper reads the prompt as preceding text.

**[Reliability #3]: Sound twins per entry**
_Concept_: An entry gets a field "not to be confused with": Claude / Cloud. Cleanup receives these pairs as replacement rules.
_Novelty_: First departure from the flat list.

**[Reliability #4]: Post-check as its own stage**
_Concept_: After cleanup a small fast step runs with only the dictionary: "Here is the text, here are the terms, replace confusions." Independent of style.
_Novelty_: One stage whose sole job is enforcement.

**[Reliability #5]: Hit counter**
_Concept_: Klarvo counts per term how often it appeared in raw STT and in final text. Settings show "Claude: recognised 12x, repaired 7x".
_Novelty_: The first return signal at all.

**[Capture #6]: One-tap "wrong word" report**
_Concept_: In History (or the preview) tap a word -> "falsch erkannt". Klarvo stores a case, nothing else happens yet. Noticing = capturing.
_Novelty_: Separates capturing from deciding; removes the settings detour that kills examples today.

**[Capture #7]: Case file per error**
_Concept_: A case keeps the audio segment, raw STT text, cleanup output, style, dictionary state, language, provider, temperature, hint text. Epic 12's audio-retry history already retains audio for a window — the hook exists.
_Novelty_: Diagnosis material instead of a bare word.

**[Diagnosis #8]: AI cause classification**
_Concept_: On a case, an assistant classifies: (a) STT misheard, (b) cleanup changed it, (c) speech unclear, (d) language/setting, (e) hint truncated. It proposes exactly one action.
_Novelty_: Andi's insight made mechanical — the dictionary is one of several answers.

**[Diagnosis #9]: Re-transcription as experiment**
_Concept_: The case's audio is re-run with variants (with/without the term, sentence hint, other temperature). The variant that fixes it becomes the evidence for the action.
_Novelty_: Evidence-based instead of guessed; uses retained audio.

**[Diagnosis #10]: Correction yields different artifact types**
_Concept_: A correction can produce a dictionary term, a sound-twin pair, a custom hint sentence, a style rule, or "ignore". The user confirms the type.
_Novelty_: Dictionary stops being the only output of learning.

**[Diagnosis #11]: Raw-vs-final diff in History**
_Concept_: History shows raw STT and final text side by side with changed words highlighted. Shows before any AI whether the error entered at STT or at cleanup.
_Novelty_: Zero-cost first diagnosis step.

**[Learning #12]: Collection basket with proposals**
_Concept_: Corrections accumulate. Periodically Klarvo proposes: "These 3 words failed twice or more. Add?" Batch instead of per-case.
_Novelty_: Learning without interrupting dictation.

**[Risk #13]: Over-correction hazard**
_Concept_: A rule "Cloud -> Claude" breaks real "Cloud" sentences. Repair rules need context or confidence; the hit counter must also count false repairs.
_Novelty_: Names the failure mode of every repair idea above.

**[Risk #14]: Position effect is measurable**
_Concept_: Andi's observation (position in sentence, speed) becomes a hypothesis the case file can test: do terms fail more at sentence start? At high speed?
_Novelty_: Turns an anecdote into a measurement.

## Phase 2 — Assumption Reversal

| # | Silent assumption today | Reversal | Idea |
|---|---|---|---|
| A1 | A dictionary is a word list. | It is a personal language profile. | #15 |
| A2 | The user maintains it by hand. | Nobody types into it; it grows only from corrections. | #16 |
| A3 | The dictionary belongs to the device. | It belongs to the person. | #17 #18 #19 |
| A4 | The Whisper hint is the main lever. | The hint is a nudge; enforcement lives after cleanup. | #4, #20 |
| A5 | All terms are equally important, forever. | Terms have weight, recency and decay. | #21 |
| A6 | A style is a mode; you pick one. | A style is a base; rules stack on it. | #22 #23 #24 |
| A7 | Cleanup sees only this one text. | Cleanup sees what the user usually talks about. | #25 |
| A8 | The free cap of 20 terms is the sales lever. | The learning features are the lever; the list is free. | #26 |
| A9 | The dictionary is language-neutral. | Terms carry a language. | #27 |
| A10 | The user finds the errors. | Klarvo flags its own uncertain words first. | #28 |
| A11 | The dictionary acts the same in every recording. | Context (target app, time, topic) selects what applies. | #29 |

### Ideas — Phase 2

**[Model #15]: Personal language profile**
_Concept_: One structured object per person: terms, sound-twin pairs, phrases/snippets, preferred spellings, a hint sentence per language, style rules. The "dictionary" is one section of it.
_Novelty_: Replaces the flat list with the thing the flat list was standing in for.

**[Model #16]: No input field**
_Concept_: The settings list becomes read-only. Entries arrive only through corrections (#6-#12) or import. Manual add stays as a hidden fallback.
_Novelty_: Forces every entry to carry its evidence (the case it came from).

**[Sync #17]: Profile file in a folder the user owns**
_Concept_: The profile is one JSON/Markdown file. The user points Klarvo at a folder that already syncs (Dropbox, Syncthing, OneDrive). Both devices read and merge it. No Klarvo server. Fits BYOK and the no-telemetry stance.
_Novelty_: Sync without an account.

**[Sync #18]: Device hand-off by QR / share sheet**
_Concept_: Desktop shows a QR with the profile; Android scans it, and vice versa via share sheet. Manual, but works with zero infrastructure and no folder setup.
_Novelty_: Explicit, visible, user-triggered sync — nothing silent.

**[Sync #19]: Klarvo account with server-side profile**
_Concept_: Real cloud sync through a Klarvo backend.
_Novelty_: Conflicts with the product stance (BYOK, no remote data, no server). Listed to make the rejection explicit, not as a candidate.

**[Hint #20]: Relevance-selected hint**
_Concept_: Whisper's 224-token cap means "send all terms" breaks past ~100 terms. Send only terms weighted by recency, frequency, and the target app; render them as a sentence (#2).
_Novelty_: The hint becomes a per-recording selection, not a dump.

**[Model #21]: Terms age**
_Concept_: Each term has last-used and hit counts (#5). Unused terms sink out of the hint but stay in the profile. Failed terms rise.
_Novelty_: Priority without the user ranking anything.

**[Style #22]: Rule switches instead of a fourth style**
_Concept_: Keep Polished / Verbatim / Chat as bases. Add switchable rules on top: "Structure spoken enumerations into lists", "Apply large-scale retractions", "Condense think-aloud into a coherent text". Each switch is one prompt block appended to any base.
_Novelty_: Andi's "add-on" made concrete as composable prompt blocks; Verbatim's "NEVER lists" is then overridden by an explicit switch, not by a new mode.

**[Style #23]: Fourth style "Gedanken" (think-aloud)**
_Concept_: A dedicated base for loose thinking: keeps content, drops false starts, groups related thoughts, structures enumerations, never invents. Chosen when Andi dictates thoughts, not messages.
_Novelty_: A mode whose contract is "consolidate", where the others say "preserve order".

**[Style #24]: Spoken meta-commands**
_Concept_: Words the cleanup treats as commands, not content: "neue Liste", "Punkt eins", "streich alles davor", "neuer Absatz". Defined in the profile, so the user chooses the trigger words.
_Novelty_: Structure becomes explicit and deterministic instead of guessed by the model. Google/Apple mix both.

**[Context #25]: Cleanup sees recent History**
_Concept_: The last N final texts (or a topic summary) ride along as context. "Claude" vs "Cloud" resolves itself when the last ten texts were about AI models.
_Novelty_: Disambiguation from the user's own corpus, no dictionary entry needed.

**[Business #26]: Cap the learning, not the list**
_Concept_: Free: unlimited terms, no case files, no diagnosis. Paid: the learning loop (#6-#12), sync (#17). The list itself stops being a paywall.
_Novelty_: The upsell is the intelligence, not a counter.

**[Model #27]: Terms carry a language**
_Concept_: "Claude" is language-neutral; "Rechnungsläufe" is DE only. The hint per language includes only matching terms; cleanup gets the right "never translate" set.
_Novelty_: Cuts hint noise and translation accidents.

**[Signal #28]: Klarvo flags its own uncertain words**
_Concept_: Whisper already returns per-segment confidence (`avg_logprob`, used today only by the hallucination filter). Low-confidence words get a subtle mark in History/preview; one tap opens the case (#6). The user reviews, not hunts.
_Novelty_: The machine proposes where to look. Verified: the signal is already parsed in code.

**[Context #29]: Profiles per target app or situation**
_Concept_: Different term sets and rules when pasting into a code editor vs. a chat vs. e-mail. Android knows the foreground app (banking blocklist already uses this); Desktop knows the active window.
_Novelty_: The dictionary stops being global.

## Phase 3 — Analogical Thinking

**Sources examined (facilitator knowledge, cutoff mid-2026; confidence noted):**
- **Dragon NaturallySpeaking** (high confidence): vocabulary entries have a *spoken form* and a *written form*; "Scratch that" deletes the last utterance; "new paragraph", "numbered list" are spoken commands; per-user profiles; adaptation from corrections ("Correct that" dialog).
- **OpenAI Whisper guidance** (high): the prompt should look like natural preceding text, not a list; for names/terms the documented recipe is a second LLM pass with the term list to fix spelling.
- **Wispr Flow** (medium-high): personal dictionary that learns from the user's edits after dictation and proposes entries; snippets; per-app tone; account-based sync; command mode for editing selected text by voice.
- **Apple Dictation / iOS keyboard** (medium-high): contacts and app content feed the recognizer's vocabulary; keyboard dictionary and text replacements sync via iCloud; on-device personalization.
- **Google Gboard / Pixel voice typing** (medium): spoken commands ("new line", "delete", "send"); personal dictionary synced via the Google account; contacts as vocabulary.
- **Samsung / Google "drop what I said before"** (low — Andi's observation, not verified by me): implicit retraction detection at large scale.
- **Aqua Voice / SuperWhisper** (medium): screen or window context for disambiguation; modes = custom prompt per situation; local models.
- **Spell checkers / autocorrect** (high): "Add to dictionary" vs "Ignore" as the two learning outcomes; undo of an autocorrect counts as a negative signal.
- **Obsidian / git** (high): plain text files in a user-owned folder, synced by whatever the user already uses; full history and revert.

### Ideas — Phase 3

**[Retraction #30]: "Scratch that" with a defined scope**
_Concept_: An explicit spoken command deletes a defined span: last sentence / everything since the last pause / everything so far. Deterministic, works in every style, no model judgment needed.
_Novelty_: Dragon solved W1b explicitly in 1997; Klarvo needs no AI for the explicit case.

**[Retraction #31]: Implicit retraction only where the style allows it**
_Concept_: Model-judged "the speaker changed their mind" runs only in Polished and in the think-aloud style (#23). Verbatim keeps the explicit command (#30) only.
_Novelty_: Separates the deterministic command from the risky inference by style.

**[Model #32]: Spoken form + written form per entry**
_Concept_: An entry is a pair: how it sounds ("klohd", "cloud") and how it is written ("Claude"). The Whisper hint gets the written form in a sentence; the post-check gets the pairs as rules. Same as Dragon's vocabulary model and the same as #3, now with a 25-year precedent.
_Novelty_: The pair replaces the guess.

**[Pipeline #33]: Vendor-documented two-pass**
_Concept_: OpenAI's own recipe for names is "transcribe, then a second pass with the list". That is #4. Adopt it as the primary enforcement stage; the Whisper hint stays as a nudge.
_Novelty_: Turns #4 from an idea into a documented best practice.

**[Learning #34]: Learn from edits in History**
_Concept_: Wispr Flow watches the user's post-dictation edits. Klarvo cannot see edits inside target apps, but History is editable: an edit there yields a diff, and each changed word becomes a correction candidate (#6) with no extra gesture.
_Novelty_: Editing is the correction; no "report" step.

**[Sources #35]: Vocabulary sources instead of typing**
_Concept_: Apple and Google feed the recognizer from contacts. Klarvo lets the user attach sources: Android contacts, a folder of project names, a text file of product terms, identifiers from a git repo. Terms are derived, not typed.
_Novelty_: The dictionary is a view over sources the user already maintains.

**[Sync #36]: Account sync is what everyone else does — and why Klarvo does not**
_Concept_: Apple, Google, Wispr all sync via an account. Klarvo's equivalent with the same user outcome is the user-owned file (#17) plus the QR hand-off (#18). Explicitly state this trade-off in the product.
_Novelty_: Makes the difference a feature ("your words never leave your storage"), not a gap.

**[Commands #37]: Small fixed command set plus user-defined triggers**
_Concept_: Gboard proves a short fixed set works ("new line", "delete that"). Klarvo ships five commands in DE and EN and lets the profile add its own trigger words (#24).
_Novelty_: Fixed core for reliability, profile for personal habit.

**[Learning #38]: "Ignore" is a first-class outcome**
_Concept_: Spell checkers have "Add" and "Ignore". A case can end with "that was my pronunciation, not the app". No artifact, but the case still counts in the statistics (#5, #14).
_Novelty_: Stops the loop from producing a term for every hiccup.

**[Signal #39]: Undo of a repair is a negative vote**
_Concept_: When the user reverts a repaired word (in preview or History), the pair loses confidence; after N reverts the rule pauses. Same as autocorrect learning from undo.
_Novelty_: Makes #13 (over-correction) self-limiting.

**[Context #40]: Window/screen context, opt-in**
_Concept_: Aqua Voice reads screen context. Desktop could pass the active window title and selected text; Android the foreground app. Strictly opt-in, local, never stored.
_Novelty_: Disambiguation from where the text is going, not only from what was said.

**[Hint #41]: Speaker profile sentence**
_Concept_: Dragon adapts acoustically; cloud Whisper cannot. The closest lever is a fixed, personal hint sentence: "Fast German speaker, frequent English tech terms, product names: Claude, Klarvo …". Written once, refined by the diagnosis loop (#8).
_Novelty_: Personalization at the one place cloud Whisper allows it.

**[Storage #42]: Profile as a versioned text file**
_Concept_: The profile lives as Markdown/JSON in the user's folder (#17) with a change log. Every learned entry is a diff with its case reference; revert is a text operation.
_Novelty_: Explainable learning — the user can read why a word is there.

**[Offline #43]: Deterministic post-check without the LLM**
_Concept_: Sound-twin pairs (#32) run as a local, rule-based replacement when the cleanup LLM is unavailable or when the style is Verbatim with cleanup off. The dictionary still works offline.
_Novelty_: The enforcement stage has a no-network floor.

**[Import #44]: Import from other tools**
_Concept_: Accept Wispr Flow / Dragon / plain-text word lists as import into the profile.
_Novelty_: Lowers the switching cost for exactly Klarvo's target group (power users who already own a word list).

**[Edge #45]: Term collision detection**
_Concept_: When a new pair would collide with an existing real word (Cloud is also a real word Andi may say), the diagnosis step warns and asks for a context rule instead of a blanket replacement.
_Novelty_: The loop refuses rules that #13 says are dangerous.

## Idea Organization

**Session output:** 45 ideas across 3 techniques (Five Whys, Assumption Reversal, Analogical Thinking).

### Themes

| Theme | What it answers | Ideas |
|---|---|---|
| **T1 Enforcement** — make terms win | I1 reliability | #1 #2 #4 #20 #32 #33 #41 #43 #45 (#13 as the risk) |
| **T2 Learning loop** — capture, diagnose, learn | W2, I1 | #5 #6 #7 #8 #9 #10 #11 #12 #14 #21 #28 #34 #38 #39 |
| **T3 Profile model** — what the "dictionary" is | I3, W2 | #15 #16 #27 #35 #42 #44 |
| **T4 Sync** — where the truth lives | I2 | #17 #18 #36 (#19 rejected) |
| **T5 Structure & retraction** — the add-on | W1a, W1b | #22 #23 #24 #30 #31 #37 |
| **T6 Context** — what else the cleanup knows | I1 (disambiguation) | #25 #29 #40 |
| **T7 Business** | — | #26 |

### Dependencies

- **T1 before T2.** A learning loop that produces entries into a dictionary that cannot enforce them (today's state) would learn into the void. Enforcement is the precondition for learning to show results.
- **T3 under T1, T2, T4.** Spoken/written pairs (#32), case references (#42) and a syncable file (#17) all need the profile format first. T3 is the shared foundation, but it can start minimal: today's list + optional pair per entry.
- **T4 after T3.** File sync needs a file format that merges; decide the format once.
- **T5 is independent of the dictionary.** Structure and retraction touch the cleanup prompts and the styles only. It can be its own epic and can run before or after T1–T4.
- **T6 later.** Context features add privacy questions (screen/window) and need the profile to store per-context rules.
- **T7 waits on the open licensing/publishing decision** (docs/backlog.md OPEN-DECISION). Not decided here.

### Smallest coherent core per theme (facilitator proposal, Reduktion vor Konstruktion)

- **T1 core:** #32 spoken/written pair as an optional field on today's entry + #33 second pass ("here is the text, here are the pairs, replace confusions") after cleanup, style-independent, with #45 collision warning at entry time. #2 (hint as sentence) is a one-line change on the way.
- **T2 core:** #34 History edit → diff → case, plus #11 raw-vs-final view. Diagnosis (#8) on demand per case, proposing exactly one artifact type (#10) or "ignore" (#38). Counters (#5, #39) come free once cases exist.
- **T3 core:** keep `dictionary.json`, extend entries from string to `{written, spoken?, language?, source?}`. No new object yet.
- **T4 core:** #17 file in a user-chosen folder, last-writer-wins merge by entry, #18 QR as a fallback for the first pairing.
- **T5 core:** #30 explicit "streich das" command + #22 one switch "Struktur" (enumerations → lists) appended to any style. Implicit retraction (#31) and a fourth style (#23) only if the switch fails in writing.

### Decision questions for Andi

- **D1 Style question:** rule switches on the three styles (#22) / fourth style "Gedanken" (#23) / both. Facilitator recommendation: switches first; a fourth style only after the switch approach is shown to fail in writing.
- **D2 Sync path:** user-owned file in a syncing folder (#17) + QR hand-off (#18); Klarvo account (#19) rejected. Recommendation: #17.
- **D3 Learning entry point:** History edit diff (#34) / explicit tap (#6) / both; AI diagnosis always or on demand. Recommendation: both capture paths, diagnosis on demand with one proposal.
- **D4 Order:** recommendation T1 → T3 → T2 → T4, with T5 as a separate epic scheduled independently.

## Decisions (Andi, 2026-09-11)

| # | Decision | Notes |
|---|---|---|
| **D1** | **Rule switches on the three styles** (#22). No fourth style now. | A fourth style "Gedanken" (#23) only if the switch approach fails in writing. Explicit "streich das" (#30) is part of T5; implicit retraction (#31) only behind a switch or in Polished. |
| **D2** | **User-owned storage, several sync options, no Klarvo server.** | Andi: Dropbox-style folder sync does not transfer to Android; if Klarvo goes public again, offer users several ways to sync for themselves. **Leading candidate for the first backend (facilitator, post-session): the existing Turso history sync** — user's own DB URL + token, already on both platforms (`src-tauri/src/sync/mod.rs`, Kotlin `pushToTurso`); the backlog STORY-CANDIDATE "Dictionary shared across devices" already sketches a second Turso table. File-in-folder (#17, Syncthing / SAF document) and QR hand-off (#18) stay as later alternatives. Which backend ships first is **not** decided. #19 rejected. |
| **D3** | **Both capture paths** — History edit diff (#34) and one-tap (#6) — **diagnosis on demand** per case with exactly one proposal (#8, #10) or "ignore" (#38). | Diagnosis is never automatic. Cases carry the material of #7; raw-vs-final view (#11) is the zero-cost first step. |
| **D4** | **Order: T1 Enforcement → T3 Profile format → T2 Learning loop → T4 Sync. T5 Structure & retraction is a separate epic, scheduled independently.** | T6 Context and T7 Business not scheduled. T7 waits on the licensing/publishing OPEN-DECISION. |

**What is NOT decided:** the concrete prompt wording of the second pass; the exact entry schema beyond "written + optional spoken/language/source"; which sync backend ships first; whether the free cap moves (#26); any UI. Nothing is released to build. The next act is a cut (epic/story shaping), not code.

**Relation to Story 7-6 (M12):** unchanged — the one-line Chat `{dict_section}` change is still the minimal coherent state and does not pre-empt T1.

## Session Summary

- **Ideas:** 45, in 7 themes. Root cause found (no enforcement stage, no return signal). Four product decisions taken.
- **Techniques:** Five Whys, Assumption Reversal, Analogical Thinking, then organization.
- **Verified in code during the session:** two dictionary use sites; flat `Vec<String>` model; Whisper prompt = hint sentence + comma list, 224-token cap; cleanup sentence is protect-only; Verbatim forbids lists and holds two conflicting rules; all styles resolve small mid-speech corrections; `avg_logprob` per segment is parsed but unused for the user; Turso sync exists on both platforms with user credentials.
- **Next steps:** (1) backlog entry "EPIC-CANDIDATE Dictionary rework" with the T1 core as first cut; (2) T5 as its own EPIC-CANDIDATE; (3) reconcile the existing sync STORY-CANDIDATE into T4; (4) routing updated. No build until Andi releases a cut.
