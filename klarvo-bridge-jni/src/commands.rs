//! Control-plane: uniffi-exported `Session` with `start_meter`/`stop_meter`.
//!
//! Spawns a producer task (fake sine-wave RMS at 20 Hz) and a bridge task
//! that forwards broadcast events into the raw-jni callback.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tokio::runtime::{Handle, Runtime};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::audio_level::AudioLevel;
use crate::streams;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
const BROADCAST_CAPACITY: usize = 32;
const TICK_MS: u64 = 50;

fn runtime_handle() -> Handle {
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .thread_name("klarvo-bridge")
                .build()
                .expect("tokio runtime init")
        })
        .handle()
        .clone()
}

#[derive(uniffi::Object)]
pub struct Session {
    state: Mutex<SessionState>,
}

#[derive(Default)]
struct SessionState {
    producer: Option<JoinHandle<()>>,
    bridge: Option<JoinHandle<()>>,
    tx: Option<broadcast::Sender<AudioLevel>>,
}

#[uniffi::export]
impl Session {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(SessionState::default()),
        })
    }

    pub fn start_meter(&self) {
        let mut state = self.state.lock().unwrap();
        if state.producer.is_some() {
            return;
        }
        let handle = runtime_handle();
        let (tx, mut rx) = broadcast::channel::<AudioLevel>(BROADCAST_CAPACITY);
        let tx_for_producer = tx.clone();

        let producer = handle.spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(TICK_MS));
            let start = Instant::now();
            let mut i: u32 = 0;
            loop {
                tick.tick().await;
                let ts_ms = start.elapsed().as_millis() as u64;
                let phase = (i as f32) * 0.1;
                let rms = ((phase.sin() * 0.5) + 0.5).abs();
                if tx_for_producer.send(AudioLevel { rms, ts_ms }).is_err() {
                    break;
                }
                i = i.wrapping_add(1);
            }
        });

        let bridge = handle.spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(level) => {
                        streams::emit_audio_level(&level);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        });

        state.producer = Some(producer);
        state.bridge = Some(bridge);
        state.tx = Some(tx);
    }

    pub fn stop_meter(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(h) = state.producer.take() {
            h.abort();
        }
        if let Some(h) = state.bridge.take() {
            h.abort();
        }
        state.tx = None;
    }
}
