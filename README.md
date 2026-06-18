# Tauri Audio App

## Description

Welcome to Tauri Audio App - the public speaking coach on your local machine.

`tauri-audio-app` is a MacOS desktop application built usint the [Tauri](https://tauri.app/start/) framework. It takes audio input, transcribes the audio to text using whisper.cpp, and calls an LLM via API to analyze the transcript. The initia

## Quickstart

1. Download and install the release
  - Current release is `tauri-audio_0.1.0_aarch64.dmg`
  - Not compatible with Intel Macs (Apple Silicon only)
  - Requires macOS 10.15 (Catalina) or newer
  - Open the .dmg, drag into Applications
  - I currently don't have a notarized Developer-ID, so at first launch run the following command:
  `xattr -dr com.apple.quarantine /Applications/tauri-audio-app` - then open the app normally.
  - Allow microphone access when prompted.
2. Configure an LLM provider 
  - I like [OpenRouter](https://openrouter.ai/) and have it set as default. Replace with any provider.
  - For model I have `openai/gpt-4o-mini` set as default. Replace with any provider supported model.
  - The key is stored in macOS keychain - when prompted this must be enabled

## Known Limitations

- Microphone access for MacOS only works with a full build. If you download the source code and run with `pnpm tauri dev`, the microphone will not work.
- Whisper is bundled with app. If you already have whisper setup on your device you are not able to use it.

## Future Plans

- Currently this build only runs for MacOS. It will be extended in the future to work on Windows as well.
- I plan to add an ability for users who already have whisper to use it instead of shipping the app bundled with the model. This will reduce file size.
