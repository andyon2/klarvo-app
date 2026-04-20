// Shared test fixtures: typed WAV accessors + trait mocks + v1 AppData snapshot.

pub mod audio_source;
pub mod cleanup;
pub mod pipeline_stage;
pub mod stt;
pub mod v1_appdata;

pub use audio_source::MockAudioSource;
pub use cleanup::{MockCleanupMode, MockCleanupStyle, assert_cleanup_input};
pub use pipeline_stage::{MockPipelineStage, harness_run_stage};
pub use stt::{MockSttProvider, assert_stt_call_count};
