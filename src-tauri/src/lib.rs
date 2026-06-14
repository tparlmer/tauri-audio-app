use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

// Shared application state: the Whisper model, loaded once and reused by all commands.
// It's safe to share because WhisperContext is Send + Sync (Arc around a thread-safe inner)
struct AppState {
    whisper: WhisperContext,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Transcribe a 16 kHz mono WAV using an already-loaded model context.
fn transcribe_audio(ctx: &WhisperContext, wav_path: &str) -> Result<String, String> {
    let samples: Vec<i16> = hound::WavReader::open(wav_path)
        .map_err(|e| e.to_string())?
        .into_samples::<i16>()
        .map(|x| x.unwrap())
        .collect();

    let mut audio = vec![0.0f32; samples.len()];
    whisper_rs::convert_integer_to_float_audio(&samples, &mut audio).map_err(|e| e.to_string())?;

    // The model is already in memory; create_state() is the cheap per-request part.
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 0 });
    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state.full(params, &audio[..]).map_err(|e| e.to_string())?;

    let mut text = String::new();
    for segment in state.as_iter() {
        text.push_str(&segment.to_string());
    }
    Ok(text)
}

#[tauri::command]
fn transcribe_sample(state: tauri::State<'_, AppState>) -> Result<String, String> {
    transcribe_audio(&state.whisper, "samples/jfk.wav")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // load model once at startup. If the file is missing the app won't work, causing a panic
    let whisper = WhisperContext::new_with_params(
        "models/ggml-base.en.bin",
        WhisperContextParameters::default(),
    )
    .expect("failed to load whisper model");

    tauri::Builder::default()
        .manage(AppState { whisper }) // <-- register the shared state
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, transcribe_sample])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Reads a 16 kHz mono WAV file, runs Whisper on it, and returns the transcript.
// Errors are returned as `String` so they can later cross into the JS frontend
// fn transcribe_file(model_path: &str, wav_path: &str) -> Result<String, String> {
//     use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

//     // Read the WAV into 16-bit integer samples (hound)
//     // hound fails with a `hound::Error`; `.map_err(|e| e.to_string())` flattens it
//     // to a String so `?` can return it from this function. Every fallible call below
//     // needs the same bridge, because each one fails with its own distinct error type.
//     let samples: Vec<i16> = hound::WavReader::open(wav_path)
//         .map_err(|e| e.to_string())?
//         .into_samples::<i16>()
//         .map(|x| x.unwrap())
//         .collect();

//     // Convert those i16s to the f32 format Whisper wants
//     let mut audio = vec![0.0f32; samples.len()];
//     whisper_rs::convert_integer_to_float_audio(&samples, &mut audio).map_err(|e| e.to_string())?;
//     // jfk.wav is already MONO + 16 kHz, we will convert_stereo_to_mono later

//     // Load the model into a context, then make a "state" to run it
//     let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
//         .map_err(|e| e.to_string())?;
//     let mut state = ctx.create_state().map_err(|e| e.to_string())?;

//     // Configure the run
//     let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 0 });
//     params.set_language(Some("en"));
//     params.set_print_special(false);
//     params.set_print_progress(false);
//     params.set_print_realtime(false);
//     params.set_print_timestamps(false);

//     // Run inference
//     state.full(params, &audio[..]).map_err(|e| e.to_string())?;

//     // Collect the text out of the segments
//     let mut text = String::new();
//     for segment in state.as_iter() {
//         text.push_str(&segment.to_string());
//     }

//     Ok(text)
// }

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
