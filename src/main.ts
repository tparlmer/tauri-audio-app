import { invoke } from "@tauri-apps/api/core";

type ToastKind = "info" | "success" | "error";

function toast(message: string, kind: ToastKind = "info") {
  // Create the container once, reuse it after that.
  let container = document.querySelector("#toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "toast-container";
    document.body.appendChild(container);
  }

  const el = document.createElement("div");
  el.className = `toast toast-${kind}`;
  el.textContent = message;
  container.appendChild(el);

  // Fade out after 3.5s, then remove from the DOM.
  setTimeout(() => {
    el.classList.add("toast-hide");
    setTimeout(() => el.remove(), 300); // wait for the fade transition
  }, 3500);
}

let transcriptEl: HTMLElement | null;
let llmResponseEl: HTMLElement | null;
let metricsEl: HTMLElement | null;
let apiKeyInputEl: HTMLInputElement | null;
let keyStatusEl: HTMLElement | null;
let baseUrlInputEl: HTMLInputElement | null;
let modelInputEl: HTMLInputElement | null;

// Mirrors the Rust FillerReport. serde keeps the field names as-is (snake_case)
// and Rust tuples (String, usize) serialize as JSON arrays [string, number].
interface FillerReport {
  word_count: number;
  filler_total: number;
  breakdown: [string, number][];
}

const DEFAULT_BASE_URL = "https://openrouter.ai/api/v1";
const DEFAULT_MODEL = "openai/gpt-4o-mini";

function getConfig() {
  return {
    base_url: localStorage.getItem("base_url") || DEFAULT_BASE_URL,
    model: localStorage.getItem("model") || DEFAULT_MODEL,
  };
}

function saveConfig() {
  if (baseUrlInputEl)
    localStorage.setItem(
      "base_url",
      baseUrlInputEl.value.trim() || DEFAULT_BASE_URL,
    );
  if (modelInputEl)
    localStorage.setItem("model", modelInputEl.value.trim() || DEFAULT_MODEL);
}

// Disable a button while its async action runs, re-enable when done (even on error)
async function withButton(buttonId: string, action: () => Promise<void>) {
  const btn = document.querySelector<HTMLButtonElement>(`#${buttonId}`);
  if (btn) btn.disabled = true;
  try {
    await action();
  } finally {
    if (btn) btn.disabled = false;
  }
}

async function refreshKeyStatus() {
  if (!keyStatusEl) return;
  const has = await invoke<boolean>("has_api_key");
  keyStatusEl.textContent = has ? "key saved" : "no key saved";
}

async function saveKey() {
  if (!apiKeyInputEl || !keyStatusEl) return;
  const key = apiKeyInputEl.value.trim();
  if (!key) return;
  try {
    await invoke("set_api_key", { key });
    apiKeyInputEl.value = ""; // clear the field once stored
    keyStatusEl.textContent = "key saved";
    toast("API key saved", "success");
  } catch (err) {
    toast(`Couldn't save key: ${err}`, "error");
  }
}

async function showMetrics() {
  if (!metricsEl || !transcriptEl) return;
  const text = transcriptEl.textContent ?? "";
  if (!text.trim()) {
    toast("Transcribe something first", "info");
    return;
  }
  try {
    const r = await invoke<FillerReport>("analyze_speech", { text });
    const detail = r.breakdown.map(([w, n]) => `${w}: ${n}`).join(", ");
    metricsEl.textContent =
      `${r.word_count} words - ${r.filler_total} fillers` +
      (detail ? ` – ${detail}` : "");
  } catch (err) {
    toast(`Metrics failed: ${err}`, "error");
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
  const { base_url, model } = getConfig();
  try {
    llmResponseEl.textContent = await invoke<string>("analyze_transcript", {
      text,
      baseUrl: base_url,
      model,
    });
  } catch (err) {
    llmResponseEl.textContent = "";
    toast(`Coaching failed: ${err}`, "error");
  }
}

async function record() {
  if (!transcriptEl) return;
  transcriptEl.textContent = "Recording 5s... speak now";
  try {
    transcriptEl.textContent = await invoke<string>("record_and_transcribe");
  } catch (err) {
    transcriptEl.textContent = "";
    toast(`Recording failed: ${err}`, "error");
  }
}

async function transcribe() {
  if (!transcriptEl) return;

  // Message to edisplay while whisper loads model, show loading Message
  transcriptEl.textContent = "Transcribing...";

  try {
    const text = await invoke<string>("transcribe_sample");
    transcriptEl.textContent = text;
  } catch (err) {
    transcriptEl.textContent = "";
    toast(`Transcription failed: ${err}`, "error");
  }
}

async function testConnection() {
  const { base_url, model } = getConfig();
  toast("Testing connection...", "info");
  try {
    await invoke<string>("test_llm", { baseUrl: base_url, model }); // camelCase to snake_case translation for baseUrl
    toast("Connection OK", "success");
  } catch (err) {
    toast(`Connection failed: ${err}`, "error");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  apiKeyInputEl = document.querySelector("#api-key-input");
  keyStatusEl = document.querySelector("#key-status");
  document.querySelector("#save-key-btn")?.addEventListener("click", () => {
    saveConfig(); // base_url + model -> localStorage
    saveKey(); // key -> keychain (only if a new one was typed)
  });
  refreshKeyStatus();

  baseUrlInputEl = document.querySelector("#base-url-input");
  modelInputEl = document.querySelector("#model-input");
  const cfg = getConfig();
  if (baseUrlInputEl) baseUrlInputEl.value = cfg.base_url;
  if (modelInputEl) modelInputEl.value = cfg.model;

  transcriptEl = document.querySelector("#transcript");
  document
    .querySelector("#transcribe-btn")
    ?.addEventListener("click", () => withButton("transcribe-btn", transcribe));

  document
    .querySelector("#record-btn")
    ?.addEventListener("click", () => withButton("record-btn", record));

  llmResponseEl = document.querySelector("#llm-response");
  document
    .querySelector("#llm-btn")
    ?.addEventListener("click", () => withButton("llm-btn", askLlm));

  metricsEl = document.querySelector("#metrics");
  document
    .querySelector("#metrics-btn")
    ?.addEventListener("click", () => withButton("metrics-btn", showMetrics));

  document
    .querySelector("#test-llm-btn")
    ?.addEventListener("click", () =>
      withButton("test-llm-btn", testConnection),
    );
});
