//! Integration-test: spawn a real JVM on Linux (via `jni` invocation feature),
//! register a listener, run `Session::start_meter` for N seconds, verify events
//! reached the listener with expected rate + no exceptions.

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use jni::objects::{JObject, JValueOwned};
use jni::{Env, InitArgsBuilder, JNIVersion, JavaVM, jni_sig, jni_str};

use klarvo_bridge_jni::Session;
use klarvo_bridge_jni::streams::{register_listener, unregister_listener};

const LISTENER_SRC: &str = r#"
public class TestListener {
    public volatile int count = 0;
    public volatile float lastRms = 0.0f;
    public volatile long lastTsMs = 0L;

    public void onLevel(float rms, long tsMs) {
        count++;
        lastRms = rms;
        lastTsMs = tsMs;
    }

    public int getCount() { return count; }
    public float getLastRms() { return lastRms; }
    public long getLastTsMs() { return lastTsMs; }
}
"#;

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn shared_jvm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().to_path_buf();
        compile_listener(&path);
        // Leak tempdir so .class files survive JVM lifetime.
        let _ = tmp.keep();

        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V1_8)
            .option(format!("-Djava.class.path={}", path.display()))
            .option("-Xcheck:jni")
            .build()
            .expect("InitArgsBuilder");

        JavaVM::new(jvm_args).expect("spawn JVM")
    })
}

fn compile_listener(out_dir: &Path) {
    let src_path = out_dir.join("TestListener.java");
    std::fs::write(&src_path, LISTENER_SRC).expect("write TestListener.java");
    let status = Command::new("javac")
        .arg("-d")
        .arg(out_dir)
        .arg(&src_path)
        .status()
        .expect("javac not available on PATH");
    assert!(status.success(), "javac failed");
}

fn create_listener_and_register<'a>(env: &mut Env<'a>) -> JObject<'a> {
    let class = env
        .find_class(jni_str!("TestListener"))
        .expect("find TestListener");
    let listener = env
        .new_object(&class, jni_sig!("()V"), &[])
        .expect("new TestListener");
    register_listener(env, &listener).expect("register_listener");
    listener
}

fn read_count(env: &mut Env, listener: &JObject) -> i32 {
    let result = env
        .call_method(listener, jni_str!("getCount"), jni_sig!("()I"), &[])
        .expect("getCount call");
    match result {
        JValueOwned::Int(v) => v,
        other => panic!("expected Int, got {other:?}"),
    }
}

#[test]
fn listener_receives_events_smoke() {
    let jvm = shared_jvm();
    jvm.attach_current_thread(|env| -> Result<(), jni::errors::Error> {
        let listener = create_listener_and_register(env);

        let session = Session::new();
        session.start_meter();

        // Wait ~500ms — expect ≥5 events (20 Hz × 0.5 s = 10 nominal; allow startup jitter).
        std::thread::sleep(Duration::from_millis(500));

        session.stop_meter();
        std::thread::sleep(Duration::from_millis(50));

        let count = read_count(env, &listener);
        assert!(count >= 5, "expected ≥5 events in 500ms, got {count}");

        unregister_listener();
        Ok(())
    })
    .expect("smoke test body");
}

#[test]
fn twenty_hz_over_ten_seconds_no_drops() {
    let jvm = shared_jvm();
    jvm.attach_current_thread(|env| -> Result<(), jni::errors::Error> {
        let listener = create_listener_and_register(env);

        let session = Session::new();
        session.start_meter();

        std::thread::sleep(Duration::from_secs(10));

        session.stop_meter();
        std::thread::sleep(Duration::from_millis(100));

        let count = read_count(env, &listener);
        eprintln!("[twenty_hz_over_ten_seconds_no_drops] final count = {count}");

        // Nominal: 20 Hz × 10 s = 200. Tolerate ±5% (190..=210).
        assert!(
            (190..=210).contains(&count),
            "expected ~200 events (±5%) in 10s, got {count}"
        );

        unregister_listener();
        Ok(())
    })
    .expect("20Hz test body");
}
