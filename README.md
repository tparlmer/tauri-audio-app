# Tauri Audio App

## Description

`tauri-audio-app` is a MacOS desktop application built usint the [Tauri](https://tauri.app/start/) framework. It takes audio input, transcribes the audio to text using whisper.cpp, and calls an LLM via API to analyze the transcript.

TODO: Add gif of frontend workflow

## Quickstart

1. Download and install whisper
  - [Instructions in repo](https://github.com/ggml-org/whisper.cpp)
  - TODO: Adjust this section for production build

## Known Limitations

Microphone access for MacOS only works with a full build. If you download the source code and run with `pnpm tauri dev`, the microphone will not work.

## Future Plans

Currently this build only runs for MacOS. It will be extended in the future to work on Windows as well.
