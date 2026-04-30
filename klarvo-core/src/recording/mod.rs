use std::fmt;
use std::str::FromStr;

use crate::error::{AppError, AppErrorKind};

/// Recording mode for the push-to-talk hotkey slot.
///
/// Controls the start/stop semantics of a recording session (ADR-0012 Amendment 1).
/// Serialised as lowercase strings in user settings (`hotkey.slot1.mode`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordingMode {
    /// Hold key to record; release to stop and transcribe (Phase-1 default).
    Hold,
    /// Press once to start; press again to stop and transcribe.
    Toggle,
    /// Press to start; VAD silence-end auto-stops and transcribes.
    AutoStop,
    /// Like Hold but skips auto-paste; emits `RecordingDelivered` instead.
    WaitAndType,
}

impl FromStr for RecordingMode {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hold" => Ok(Self::Hold),
            "toggle" => Ok(Self::Toggle),
            "autostop" => Ok(Self::AutoStop),
            "wait_and_type" => Ok(Self::WaitAndType),
            other => Err(AppError {
                kind: AppErrorKind::Validation,
                message: format!("unknown recording mode: {other:?}"),
                user_message: Some("error.settings.validation".into()),
                retryable: false,
            }),
        }
    }
}

impl fmt::Display for RecordingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hold => write!(f, "hold"),
            Self::Toggle => write!(f, "toggle"),
            Self::AutoStop => write!(f, "autostop"),
            Self::WaitAndType => write!(f, "wait_and_type"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_variants() {
        for mode in [
            RecordingMode::Hold,
            RecordingMode::Toggle,
            RecordingMode::AutoStop,
            RecordingMode::WaitAndType,
        ] {
            let s = mode.to_string();
            let parsed = RecordingMode::from_str(&s).expect("roundtrip must succeed");
            assert_eq!(parsed, mode, "roundtrip failed for {:?}", mode);
        }
    }

    #[test]
    fn from_str_known_values() {
        assert_eq!(RecordingMode::from_str("hold").unwrap(), RecordingMode::Hold);
        assert_eq!(RecordingMode::from_str("toggle").unwrap(), RecordingMode::Toggle);
        assert_eq!(RecordingMode::from_str("autostop").unwrap(), RecordingMode::AutoStop);
        assert_eq!(RecordingMode::from_str("wait_and_type").unwrap(), RecordingMode::WaitAndType);
    }

    #[test]
    fn wait_and_type_uses_underscore_not_hyphen() {
        assert!(RecordingMode::from_str("wait_and_type").is_ok());
        assert!(RecordingMode::from_str("wait-and-type").is_err());
    }

    #[test]
    fn unknown_string_returns_validation_error() {
        let err = RecordingMode::from_str("unknown_mode").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Validation));
    }

    #[test]
    fn display_produces_expected_strings() {
        assert_eq!(RecordingMode::Hold.to_string(), "hold");
        assert_eq!(RecordingMode::Toggle.to_string(), "toggle");
        assert_eq!(RecordingMode::AutoStop.to_string(), "autostop");
        assert_eq!(RecordingMode::WaitAndType.to_string(), "wait_and_type");
    }
}
