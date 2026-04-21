# test-assets

Binary test fixtures for integration and unit tests.

## Git LFS

All audio files (`.wav`, `.mp3`, `.ogg`) are tracked via Git Large File Storage.
Root-level `*.wav / *.mp3 / *.ogg` patterns in `.gitattributes` cover any
subdirectory. Run `git lfs install` once per dev machine before cloning or pulling.

## Directory layout

```
test-assets/
├── audio/          # WAV/MP3/OGG fixtures for AudioSource / resampler tests
└── v1-appdata/     # Snapshot of v1 AppData directory for migration tests (ADR-0004)
```

## Adding fixtures

Place new audio files under `test-assets/audio/`. Git will automatically pointer-
track them via LFS. Commit both the LFS pointer and run `git lfs push` before
opening a PR so CI can fetch the actual binary.

Non-audio binaries that exceed ~1 MB should also be committed via LFS with an
explicit `.gitattributes` entry.
