import { invoke } from "@tauri-apps/api/core";

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;
let transcriptEl: HTMLElement | null;

async function greet() {
  if (greetMsgEl && greetInputEl) {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
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
});
