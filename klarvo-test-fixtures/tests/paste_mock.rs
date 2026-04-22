use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::output::PasteBackend;
use klarvo_test_fixtures::MockPasteBackend;

#[tokio::test]
async fn default_happy_path() {
    let mock = MockPasteBackend::new();
    let result = mock.paste().await;
    assert!(result.is_ok());
    assert_eq!(mock.call_count(), 1);
    assert!(mock.was_called());
}

#[tokio::test]
async fn configured_to_fail() {
    let mock = MockPasteBackend::new().with_result(Err(AppError {
        kind: AppErrorKind::Io,
        message: "test io error".into(),
        user_message: None,
        retryable: false,
    }));
    let result = mock.paste().await;
    assert!(matches!(result, Err(ref e) if matches!(e.kind, AppErrorKind::Io)));
    assert_eq!(mock.call_count(), 1);
}

#[tokio::test]
async fn multiple_calls() {
    let mock = MockPasteBackend::new();
    mock.paste().await.unwrap();
    mock.paste().await.unwrap();
    mock.paste().await.unwrap();
    assert_eq!(mock.call_count(), 3);
}
