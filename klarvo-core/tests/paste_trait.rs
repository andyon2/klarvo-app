use std::sync::Arc;

#[test]
fn paste_backend_is_object_safe_and_arc_compatible() {
    // Verifies Send + Sync bounds and dyn-compatibility at compile time.
    let mock = klarvo_test_fixtures::MockPasteBackend::new();
    let _: Arc<dyn klarvo_core::output::PasteBackend> = Arc::new(mock);
}
