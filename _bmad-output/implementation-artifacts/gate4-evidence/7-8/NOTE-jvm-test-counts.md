# What the "24 Tests" banner in the smoke logs covers (and what it does not)

Added in story 7-8 fix round 2 (code review round 2, patch finding 9); corrected and completed in
fix round 3 (round-3 decision D1 + patch finding 8).

**Convention (declared here because it was previously undeclared):** `.gitignore:15` ignores
`*.log`, so every smoke log in this evidence directory is committed with an explicit
`git add -f` — that is how `smoke-r1-00e771d.log` got here, and `smoke-r2` / `smoke-r3` now
follow the same route.

All three committed smoke logs in this directory print, under `── JVM-Unit-Tests ──`:

    [ok]    24 Tests, 0 Failures — alle grün

`smoke-r1-00e771d.log:17`, `smoke-r2-27de205.log:17` and `smoke-r3-85aa0ee.log:17`. The story
record carries **168 tests / 0 failures across 22 suites** for the same gate, on the same day, across the three
commits named above (`00e771d`, `27de205`, `85aa0ee`). Both numbers are correct; they count different things.

**Cause.** The banner is not the run total. `scripts/android-smoke.sh` reads the counts in its
`TEST_XML=$(find app/build/test-results -name "*.xml" -print -quit …)` block — `-print -quit`
plus `head -1` on the `tests="…"` attribute, i.e. the count of **one arbitrary result XML =
one test suite**. (The same log states `22 Test-Dateien kopiert` three lines above the banner,
at `:14`. That line does *not* prove the banner is partial — 22 source files can hold 24 test
methods — it only tells you how many test source files were synced. The load-bearing argument is the
aggregate below.)

**The run totals.** Both later smoke logs carry an aggregate line at `:44`, quoted verbatim here so
the 168 stands even without the logs:

    JVM (laptop, 27de205): tests 168 fail 0 err 0 skip 0
    JVM (laptop, 85aa0ee): tests 168 fail 0 err 0 skip 0

(`smoke-r2-27de205.log:44` and `smoke-r3-85aa0ee.log:44`; `smoke-r1-00e771d.log` predates the
aggregate line and carries only the `24` banner.) That is the
`:app:testUniversalDebugUnitTest` variant total — the number the story record uses.
`structure-install-r1.txt` carries **no** test counts at all; it is the AC6b install probe
(`primaryCpuAbi=arm64-v8a`, `lib/arm64` nativeloader path, 0 `UnsatisfiedLinkError`, live
`klarvo_lib::*` log lines). Do not read a test total out of it.

**Scope of these logs.** They are AC6b **install** evidence: the arm64-v8a split lands on a
locally attached AVD (`emulator-5554`, on the laptop) and JNI is reached. They prove no
dictation, no gesture, no overlay, no HyperOS behaviour and no network call. The `24` in the
banner exercises one suite; the `168` exercises the 22 suites of one variant (gradle also runs
other ABI/buildtype variants that duplicate the same suite) — both are Linux/JVM logic results.

**The script fix is deferred**, not applied: correcting the banner to aggregate all result XMLs
is recorded as a deferred finding in the 7-8 story record (round 2, deferred list) and sits
outside AC6's two named traps. `scripts/android-smoke.sh` is deliberately unchanged by this
note.

**Suite count anchor (review round 4, D1 → a).** The "22 suites" figure is anchored in
`jvm-suites-5128432.md` — the per-suite table read from the result XMLs of the final smoke on
`5128432` (`smoke-r4-5128432.log`). The aggregate lines in the r2/r3/r4 logs are written by the
conductor's probe after the script's closing box, not by `android-smoke.sh` itself. The r2 and r3
logs carry `[warn] APK-Timestamp nicht aktualisiert` — no production Kotlin changed after `00e771d`,
so Gradle reused the APK; the arm64 install and the Rust-side JNI proof were re-run on each.
