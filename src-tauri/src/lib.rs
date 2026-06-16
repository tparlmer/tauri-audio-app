use serde::Deserialize;
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

// Shared application state: the Whisper model, loaded once and reused by all commands.
// It's safe to share because WhisperContext is Send + Sync (Arc around a thread-safe inner)
struct AppState {
    whisper: Arc<WhisperContext>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: Message,
}
#[derive(Deserialize)]
struct Message {
    content: String,
}

// Classic HTTP request/response API call to LLM provider
fn ask_llm(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();

    // Build the JSON request body. `json!{...}` writes JSON inline.
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt }],
    });

    // POST (blocking - waits for the response)
    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(api_key) // sets header: Authorization: Bearer <key>
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;

    // Non-2xx -> return the status + body as an error
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("LLM returned {status}: {text}"));
    }

    // Parse JSON into our structs, then pull out the first choice's content.
    let parsed: ChatResponse = resp.json().map_err(|e| e.to_string())?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "no choices in response".to_string())
}

// Run Whisper on 16kHz mono f32 samples that are already in memory
fn transcribe_samples(ctx: &WhisperContext, audio: &[f32]) -> Result<String, String> {
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 0 });
    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state.full(params, audio).map_err(|e| e.to_string())?;

    let mut text = String::new();
    for segment in state.as_iter() {
        text.push_str(&segment.to_string());
    }
    Ok(text)
}

fn transcribe_audio(ctx: &WhisperContext, wav_path: &str) -> Result<String, String> {
    let samples: Vec<i16> = hound::WavReader::open(wav_path)
        .map_err(|e| e.to_string())?
        .into_samples::<i16>()
        .map(|x| x.unwrap())
        .collect();

    let mut audio = vec![0.0f32; samples.len()];
    whisper_rs::convert_integer_to_float_audio(&samples, &mut audio).map_err(|e| e.to_string())?;

    transcribe_samples(ctx, &audio)
}

// Crude 48 kHz -> 16 kHz: average each group of 3 samples into one.
// The averaginv doubles as a simple low-pass, which limits aliasing for the exact 3:1 ratio
// Good enough for speech into Whisper; swapping in `rubato` for sharper quality is a later pass
fn downsample_48k_to_16k(input: &[f32]) -> Vec<f32> {
    input
        .chunks_exact(3)
        .map(|c| (c[0] + c[1] + c[2]) / 3.0)
        .collect()
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Send a transcript to the LLM and return its reply.
// Runs on its own thread for two reasons: ask_llm uses a BLOCKING HTTP client, which
// must not run inside Tauri's async runtime, and it keeps the network wait off the UI.
#[tauri::command]
fn analyze_transcript(text: String) -> Result<String, String> {
    std::thread::spawn(move || -> Result<String, String> {
        let key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| "OPENROUTER_API_KEY not set".to_string())?;
        let prompt = format!("Respond helpfully to what the speaker said:\n\n{text}");
        ask_llm(&key, "openai/gpt-4o-mini", &prompt)
    })
    .join()
    .map_err(|_| "llm thread panicked".to_string())?
}

// capture ~5s of mic audio on a dedicated thread (cpal's Stream is !Send, so it
// must be created, used and dropped all on one thread), then downsample + transcribe
#[tauri::command]
fn record_and_transcribe(state: tauri::State<'_, AppState>) -> Result<String, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let whisper = Arc::clone(&state.whisper); // cheap Arc clone, moves into the thread

    let handle = std::thread::spawn(move || -> Result<String, String> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("no input device")?;
        let config = device.default_input_config().map_err(|e| e.to_string())?;

        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let buffer_for_cb = Arc::clone(&buffer);
        let stream = device
            .build_input_stream(
                config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    buffer_for_cb.lock().unwrap().extend_from_slice(data);
                },
                |err| eprintln!("stream error: {err}"),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        std::thread::sleep(Duration::from_secs(5));
        drop(stream);

        let captured = buffer.lock().unwrap().clone();
        let audio_16k = downsample_48k_to_16k(&captured);
        transcribe_samples(&whisper, &audio_16k)
    });

    // Wait for the worker thread and unwrap its Result<String, String>.
    handle
        .join()
        .map_err(|_| "audio thread panicked".to_string())?
}

#[tauri::command]
fn transcribe_sample(state: tauri::State<'_, AppState>) -> Result<String, String> {
    transcribe_audio(&state.whisper, "samples/jfk.wav")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // load model once at startup. If the file is missing the app won't work, causing a panic
    let whisper = Arc::new(
        WhisperContext::new_with_params(
            "models/ggml-base.en.bin",
            WhisperContextParameters::default(),
        )
        .expect("failed to load whisper model"),
    );

    tauri::Builder::default()
        .manage(AppState { whisper }) // <-- register the shared state
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            transcribe_sample,
            record_and_transcribe,
            analyze_transcript
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[test]
fn transcribes_jfk() {
    let ctx = WhisperContext::new_with_params(
        "models/ggml-base.en.bin",
        WhisperContextParameters::default(),
    )
    .unwrap();
    let text = transcribe_audio(&ctx, "samples/jfk.wav").unwrap();
    println!("TRANSCRIPT: {text}");
    assert!(text.to_lowercase().contains("country"));
}

#[test]
fn print_input_device() {
    use cpal::traits::{DeviceTrait, HostTrait};

    // the "host" is the OS audio system (CoreAudio on macOS)
    let host = cpal::default_host();
    // The default input device is the users current default microphone
    let device = host
        .default_input_device()
        .expect("no input device available");
    println!("Input device: {}", device);

    // The device's default capture format. This is what we'l be resampling from
    let config = device
        .default_input_config()
        .expect("no default input config");
    println!(" sample rate: {} Hz", config.sample_rate());
    println!(" channels: {}", config.channels());
    println!(" sample format: {:?}", config.sample_format());
}

#[test]
fn captures_audio() {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input device");
    let config = device.default_input_config().expect("no default config");

    // Shared buffer: the audio thread writes to it, the main thread reads it afterward.
    // Arc = shared ownership across threads; Mutex = safe shared mutation.
    // TODO: Review Rust concurrency patterns below
    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buffer_for_cb = Arc::clone(&buffer); // a second handle to the same buffer

    let stream = device
        .build_input_stream(
            config.into(),
            // This closure runs on cpal's real-time AUDIO thread, repeatedly,
            // handed a chunk of samples each time. `move` gives it ownership of
            // buffer_for_cb. We only do the cehap thing here: append the samples
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                buffer_for_cb.lock().unwrap().extend_from_slice(data);
            },
            |err| eprintln!("stream error: {err}"),
            None, // timeout
        )
        .expect("failed to build input stream");

    stream.play().expect("failed to start stream"); // start capturing
    println!("recording 3 seconds... say something");
    std::thread::sleep(Duration::from_secs(3));
    drop(stream); // stopping = dropping the stream (RAII))

    let n = buffer.lock().unwrap().len();
    println!("captured {n} samples (~{} seconds at 48kHz)", n / 48000);
    assert!(n > 0, "no audio captured");
}

#[test]
// TODO: Figure out how to get test to prompt your voice for recording
fn transcribes_my_voice() {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input device");
    let config = device.default_input_config().expect("no default config");

    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buffer_for_cb = Arc::clone(&buffer);
    let stream = device
        .build_input_stream(
            config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                buffer_for_cb.lock().unwrap().extend_from_slice(data);
            },
            |err| eprintln!("stream error: {err}"),
            None,
        )
        .expect("failed to build stream");

    stream.play().expect("failed to start");
    println!(">>> recording 5 seconds - SPEAK NOW <<<");
    std::thread::sleep(Duration::from_secs(5));
    drop(stream);

    let captured = buffer.lock().unwrap().clone();

    let peak = captured.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    let nonzero = captured.iter().filter(|&&s| s != 0.0).count();
    println!(
        "peak amplitude: {peak:.4} | nonzero: {nonzero}/{}",
        captured.len()
    );

    let audio_16k = downsample_48k_to_16k(&captured);
    println!(
        "captured {} samples @48k -> {} @16k",
        captured.len(),
        audio_16k.len()
    );

    let ctx = WhisperContext::new_with_params(
        "models/ggml-base.en.bin",
        WhisperContextParameters::default(),
    )
    .unwrap();
    let text = transcribe_samples(&ctx, &audio_16k).unwrap();
    print!(">>> YOU SAID: {text}");
}

// LLM API call connectivity test
// API key setup for single terminal session
#[test]
fn asks_llm() {
    let key = std::env::var("OPENROUTER_API_KEY").expect("set OPENROUTER_API_KEY");
    let reply = ask_llm(
        &key,
        "openai/gpt-4o-mini",
        "Say hello in exactly five words.",
    )
    .unwrap();
    println!(">>> LLM: {reply}");
    assert!(!reply.is_empty());
}
