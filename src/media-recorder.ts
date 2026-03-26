/**
 * Browser-based audio recording for Android.
 *
 * On Android, cpal (the Rust audio capture library) is not available. Instead
 * we use the MediaRecorder API that the Android WebView exposes. The recorded
 * audio is returned as a WAV byte array so the Rust backend can feed it
 * directly into the STT pipeline via `transcribe_audio_bytes`.
 *
 * Flow:
 *   startBrowserRecording() -> user speaks -> stopBrowserRecording()
 *   -> WAV Uint8Array -> invoke("transcribe_audio_bytes", ...)
 */

let mediaRecorder: MediaRecorder | null = null;
let audioChunks: Blob[] = [];

/**
 * Requests microphone access and starts recording.
 * Prefers 16 kHz mono with noise suppression -- matches the Whisper model
 * input spec so the backend doesn't need to resample.
 */
export async function startBrowserRecording(): Promise<void> {
  const stream = await navigator.mediaDevices.getUserMedia({
    audio: {
      channelCount: 1,
      sampleRate: 16000,
      echoCancellation: true,
      noiseSuppression: true,
    },
  });

  audioChunks = [];

  // Prefer opus/webm -- good quality at low bitrate, widely supported in
  // Android WebView. Fall back to plain webm if codec string is rejected.
  const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
    ? "audio/webm;codecs=opus"
    : "audio/webm";

  mediaRecorder = new MediaRecorder(stream, { mimeType });

  mediaRecorder.ondataavailable = (event) => {
    if (event.data.size > 0) {
      audioChunks.push(event.data);
    }
  };

  // Collect chunks every 100 ms so we don't lose data on abrupt stops.
  mediaRecorder.start(100);
}

/**
 * Stops recording and returns the captured audio as a 16-bit PCM WAV.
 * Releases the microphone track after decoding.
 */
export async function stopBrowserRecording(): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    if (!mediaRecorder) {
      reject(new Error("No recording in progress"));
      return;
    }

    const recorderRef = mediaRecorder;

    recorderRef.onstop = async () => {
      try {
        const blob = new Blob(audioChunks, { type: recorderRef.mimeType });
        const arrayBuffer = await blob.arrayBuffer();

        // Decode the webm/opus blob into raw PCM samples.
        const audioContext = new AudioContext({ sampleRate: 16000 });
        const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);
        await audioContext.close();

        const wavBytes = audioBufferToWav(audioBuffer);

        // Release the microphone.
        recorderRef.stream.getTracks().forEach((t) => t.stop());
        mediaRecorder = null;
        audioChunks = [];

        resolve(new Uint8Array(wavBytes));
      } catch (err) {
        reject(err);
      }
    };

    recorderRef.stop();
    mediaRecorder = null;
  });
}

/**
 * Returns true while a recording session is active.
 */
export function isRecordingActive(): boolean {
  return mediaRecorder !== null && mediaRecorder.state === "recording";
}

// ---------------------------------------------------------------------------
// WAV encoding
// ---------------------------------------------------------------------------

/**
 * Encodes an AudioBuffer as a 16-bit PCM WAV (mono, preserves source sample rate).
 * Uses only the first channel -- we always request mono from getUserMedia.
 */
function audioBufferToWav(buffer: AudioBuffer): ArrayBuffer {
  const numChannels = 1;
  const sampleRate = buffer.sampleRate;
  const samples = buffer.getChannelData(0);
  const bitsPerSample = 16;
  const bytesPerSample = bitsPerSample / 8;
  const blockAlign = numChannels * bytesPerSample;
  const dataSize = samples.length * blockAlign;
  const headerSize = 44;

  const ab = new ArrayBuffer(headerSize + dataSize);
  const view = new DataView(ab);

  // RIFF header
  writeAscii(view, 0, "RIFF");
  view.setUint32(4, ab.byteLength - 8, true);
  writeAscii(view, 8, "WAVE");

  // fmt chunk
  writeAscii(view, 12, "fmt ");
  view.setUint32(16, 16, true);          // chunk size
  view.setUint16(20, 1, true);           // PCM format
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * blockAlign, true); // byte rate
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);

  // data chunk
  writeAscii(view, 36, "data");
  view.setUint32(40, dataSize, true);

  // Convert float32 samples to int16
  let offset = headerSize;
  for (let i = 0; i < samples.length; i++) {
    const clamped = Math.max(-1, Math.min(1, samples[i]));
    const int16 = clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff;
    view.setInt16(offset, int16, true);
    offset += 2;
  }

  return ab;
}

function writeAscii(view: DataView, offset: number, str: string): void {
  for (let i = 0; i < str.length; i++) {
    view.setUint8(offset + i, str.charCodeAt(i));
  }
}
