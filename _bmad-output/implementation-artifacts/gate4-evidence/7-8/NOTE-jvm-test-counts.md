# What the "24 Tests" banner in the smoke logs covers (and what it does not)

Added in story 7-8 fix round 2 (code review round 2, patch finding 9).

Both committed smoke logs in this directory print, under `── JVM-Unit-Tests ──`:

    [ok]    24 Tests, 0 Failures — alle grün

`smoke-r1-00e771d.log:17` and `smoke-r2-27de205.log:17`. The story record carries **168 tests /
0 failures across 22 suites** for the same gate, the same day, the same tree. Both numbers are
correct; they count different things.

**Cause.** The banner is not the run total. `scripts/android-smoke.sh` reads the counts in its
`TEST_XML=$(find app/build/test-results -name "*.xml" -print -quit …)` block — `-print -quit`
plus `head -1` on the `tests="…"` attribute, i.e. the count of **one arbitrary result XML =
one test suite**. The same log states `22 Test-Dateien kopiert` two lines above, so 24 cannot
be the whole run.

**The run totals.** `smoke-r2-27de205.log:44` carries the aggregate line:

    JVM (laptop, 27de205): tests 168 fail 0 err 0 skip 0

That is the `:app:testUniversalDebugUnitTest` variant total — the number the story record uses.
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
