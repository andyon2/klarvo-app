# Phase-1 Developer Notes

## Security: Plain-SQLite API-Key Storage

`PlainSqliteKeyStore` stores API-keys in plaintext within a local SQLite file. This is
**Security-Theater** (NFR4): a Windows-ACL-restriction on current-user read/write mitigates
casual-access by other OS-users, but does **not** protect against privileged-process-read,
disk-backup-extraction, or malware running as the same user. This implementation exists
**only** behind the `dev-plain-keystore` Cargo-feature and is **never** compiled into
release-builds. Real API-key-protection comes via the OS-Keystore-Impl (Phase-4
release-default per FR46).

Phase-1-Builds are dogfooding-prototype only — do not treat local API-keys as production-secure.
Rotate keys frequently if testing in shared environments.
