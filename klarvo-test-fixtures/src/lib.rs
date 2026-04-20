// Shared test fixtures: typed WAV accessors + trait mocks + v1 AppData snapshot.

pub mod audio_source;
pub mod cleanup;
pub mod groq_mock;
pub mod keystore_mock;
pub mod manifest;
pub mod pipeline_stage;
pub mod stage_type;
pub mod stt;
pub mod stt_provider;
pub mod v1_appdata;
pub mod vad_provider;

pub use audio_source::MockAudioSource;
pub use cleanup::{MockCleanupMode, MockCleanupStyle, assert_cleanup_input};
pub use groq_mock::GroqMockServer;
pub use keystore_mock::MockKeyStore;
pub use manifest::{
    manifest_with_unknown_stage_toml, manifest_with_wrong_schema_version_toml,
    valid_minimal_manifest_toml, valid_passthrough_verbatim_manifest_toml,
};
pub use pipeline_stage::{MockPipelineStage, harness_run_stage};
pub use stage_type::*;
pub use stt::{QueuedMockSttProvider, assert_stt_call_count};
pub use stt_provider::MockSttProvider;
pub use vad_provider::MockVadProvider;
