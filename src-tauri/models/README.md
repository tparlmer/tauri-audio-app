# Whisper models

The transcription engine (`whisper-rs`, a binding to whisper.cpp) loads a local
GGML model file from this directory at runtime. The weights are large (~141 MB for
`base.en`) and are intentionally **not** committed to git — download them yourself.

## Download the default model (base.en)

Run from the repository root:

```bash
curl -L -o src-tauri/models/ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```

## Choosing a model size

Smaller models are faster but less accurate. All English-only (`.en`) variants come
from the whisper.cpp model repository on Hugging Face:
<https://huggingface.co/ggerganov/whisper.cpp>

| Model | File                 | Approx size |
|-------|----------------------|-------------|
| tiny  | `ggml-tiny.en.bin`   | ~75 MB      |
| base  | `ggml-base.en.bin`   | ~141 MB     |
| small | `ggml-small.en.bin`  | ~466 MB     |

If the `ggerganov/whisper.cpp` URL ever 404s, the repo may have moved to
`ggml-org/whisper.cpp` — swap that into the path.
