import { invoke } from "@tauri-apps/api/core";

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;
let transcriptEl: HTMLElement | null;
let llmResponseEl: HTMLElement | null;

async function greet() {
  if (greetMsgEl && greetInputEl) {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
  }
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
});
