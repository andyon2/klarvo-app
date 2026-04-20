// Shared test fixtures: typed WAV accessors + trait mocks + v1 AppData snapshot.

pub mod audio_source;
pub mod pipeline_stage;
pub mod v1_appdata;

pub use audio_source::MockAudioSource;
pub use pipeline_stage::{MockPipelineStage, harness_run_stage};
