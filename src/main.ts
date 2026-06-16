import { invoke } from "@tauri-apps/api/core";

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;
let transcriptEl: HTMLElement | null;
let llmResponseEl: HTMLElement | null;
let metricsEl: HTMLElement | null;

// Mirrors the Rust FillerReport. serde keeps the field names as-is (snake_case)
// and Rust tuples (String, usize) serialize as JSON arrays [string, number].
interface FillerReport {
  word_count: number;
  filler_total: number;
  breakdown: [string, number][];
}

async function greet() {
  if (greetMsgEl && greetInputEl) {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
  }
}

async function showMetrics() {
  if (!metricsEl || !transcriptEl) return;
  const text = transcriptEl.textContent ?? "";
  if (!text.trim()) {
    metricsEl.textContent = "Transcribe something first.";
    return;
  }

  const r = await invoke<FillerReport>("analyze_speech", { text });
  const detail = r.breakdown.map(([w, n]) => `${w}: ${n}`).join(", ");
  metricsEl.textContent =
    `${r.word_count} words - ${r.filler_total} fillers` +
    (detail ? ` – ${detail}` : "");
}

async function askLlm() {
  if (!llmResponseEl || !transcriptEl) return;
  const text = transcriptEl.textContent ?? "";
  if (!text.trim()) {
    llmResponseEl.textContent = "Transcribe something first.";
    return;
  }
  llmResponseEl.textContent = "Thinking...";
  try {
    llmResponseEl.textContent = await invoke<string>("analyze_transcript", {
      text,
    });
  } catch (err) {
    llmResponseEl.textContent = `Error: ${err}`;
  }
}

async function record() {
  if (!transcriptEl) return;
  transcriptEl.textContent = "Recording 5s... speak now";
  try {
    transcriptEl.textContent = await invoke<string>("record_and_transcribe");
  } catch (err) {
    transcriptEl.textContent = `Error: ${err}`;
  }
}

async function transcribe() {
  if (!transcriptEl) return;

  // Message to edisplay while whisper loads model, show loading Message
  transcriptEl.textContent = "Transcribing...";

  try {
    // invoke returns a Promise. Ok9text) in Rust -> resolves to the string;
    // Err(msg) in Rust -> rejects, landing us in catch.
    const text = await invoke<string>("transcribe_sample");
    transcriptEl.textContent = text;
  } catch (err) {
    transcriptEl.textContent = `Error: ${err}`;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  document.querySelector("#greet-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  transcriptEl = document.querySelector("#transcript");
  document.querySelector("#transcribe-btn")?.addEventListener("click", () => {
    transcribe();
  });

  document
    .querySelector("#record-btn")
    ?.addEventListener("click", () => record());

  llmResponseEl = document.querySelector("#llm-response");
  document.querySelector("#llm-btn")?.addEventListener("click", () => askLlm());

  metricsEl = document.querySelector("#metrics");
  document
    .querySelector("#metrics-btn")
    ?.addEventListener("click", () => showMetrics());
});
