#![cfg(target_os = "android")]

use klarvo_core::error::AppErrorKind;
use klarvo_core::keystore::os::AndroidKeystore;
use klarvo_core::keystore::{keys, KeyStore};
use secrecy::SecretString;

#[test]
fn new_is_infallible() {
    let _store = AndroidKeystore::new("klarvo");
}

#[tokio::test]
async fn all_methods_return_phase3_scaffold_error() {
    let store = AndroidKeystore::new("klarvo");

    for result in [
        store.get("k").await.err(),
        store.set("k", SecretString::new("v".into())).await.err(),
        store.delete("k").await.err(),
    ] {
        let err = result.expect("each method must return Err");
        assert!(matches!(err.kind, AppErrorKind::KeyMissing));
        assert_eq!(err.user_message.as_deref(), Some(keys::BACKEND_UNAVAILABLE));
        // Divergenz 2: substrings in .message (AppError has no source-chain).
        assert!(
            err.message.contains("Phase-3 scope"),
            "message must contain 'Phase-3 scope': {}",
            err.message
        );
        assert!(
            err.message.contains("AccessibilityService-Policy-Audit blocker"),
            "message must contain 'AccessibilityService-Policy-Audit blocker': {}",
            err.message
        );
    }
}
