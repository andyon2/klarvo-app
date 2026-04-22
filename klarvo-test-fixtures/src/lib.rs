// Shared test fixtures: typed WAV accessors + trait mocks + v1 AppData snapshot.

pub mod audio_source;
pub mod clock;
pub mod env;
pub mod event_bus_harness;
pub mod network;
pub mod output;
pub mod cleanup;
pub mod groq_mock;
pub mod keystore;
pub mod manifest;
pub mod paste;
pub mod pipeline_stage;
pub mod stage_type;
pub mod stt;
pub mod stt_provider;
pub mod v1_appdata;
pub mod vad_provider;

pub use audio_source::MockAudioSource;
pub use clock::FakeClock;
pub use env::HeadlessTestEnv;
pub use event_bus_harness::{MockErrorEmitter, assert_contains_variant, collect_emitted};
pub use network::NoNetworkGuard;
pub use output::InMemoryOutputTarget;
pub use cleanup::{MockCleanupMode, MockCleanupStyle, assert_cleanup_input};
pub use groq_mock::GroqMockServer;
pub use keystore::InMemoryKeyStore;
pub use paste::MockPasteBackend;
pub use manifest::{
    manifest_with_unknown_stage_toml, manifest_with_wrong_schema_version_toml,
    valid_minimal_manifest_toml, valid_passthrough_verbatim_manifest_toml,
};
pub use pipeline_stage::{MockPipelineStage, harness_run_stage};
pub use stage_type::*;
pub use stt::{QueuedMockSttProvider, assert_stt_call_count};
pub use stt_provider::MockSttProvider;
pub use vad_provider::MockVadProvider;
