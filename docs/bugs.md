# BUGS

06.15.2026

- Microphone returns silence (`[BLANK_AUDIO]`) in `tauri dev` on macOS

### Symptom
The "Record" button captured audio and ran it through the full pipeline, but Whisper
always returned `[BLANK_AUDIO]` (its marker for silence). No error, no crash — just
silence reaching the model. Critically: **macOS never showed a microphone-permission
prompt, and the app never appeared in System Settings -> Privacy & Security ->
Microphone.**

### What made it confusing
The exact same capture + downsample + Whisper code had **already worked** moments
earlier when run as a `cargo test` (`transcribes_my_voice`) — it transcribed real
speech, peak amplitude ~0.12. So the code was provably correct. Only the *app* failed.

### Diagnosis (the useful part)
Bisected by asking "what is different between the working case and the broken case?"

1. The file-transcription path (`transcribe_sample`, reads a WAV) still worked in the
   app. So Whisper, the downsampler, and the Tauri command plumbing were all healthy.
   The problem was isolated to **microphone audio specifically**.
2. The mic worked from a `cargo test` but not from the app. The only difference is the
   **process**: the test binary is a CLI tool; the app is `target/debug/tauri-audio`.
3. macOS microphone permission (TCC = Transparency, Consent, and Control) is granted
   **per application**, keyed to the executable's identity. When the test ran, the
   "responsible process" was the **terminal** (which had already been granted mic
   access), so the CLI binary inherited that grant. The GUI app is a *separate*
   executable that had never been granted access.
4. No prompt + no Settings entry was the final clue: macOS wasn't even *asking*. That
   happens when the binary has **no usage-description string** (`NSMicrophoneUsageDescription`)
   that the OS can show in a prompt — so instead of prompting, it **silently denies and
   feeds the app zeros**.

### Root cause
Two layers stacked:
- **macOS TCC**: an app may not access the mic without a declared
  `NSMicrophoneUsageDescription`; lacking one, the OS silently denies rather than prompts.
- **Tauri dev specifics**: `tauri dev` runs an **unbundled, ad-hoc binary** (not a
  `.app` bundle). A normal Tauri *production* build embeds `Info.plist` into the bundle,
  so permissions work there — but in dev there is no bundle, so the usage description
  never reaches the OS. This is a known Tauri issue
  (github.com/tauri-apps/tauri/issues/11951).

So: permissions worked in a packaged build but silently failed in `tauri dev` — the
classic "works in prod, broken in dev" shape.

### Fix
1. Add `src-tauri/Info.plist` declaring `NSMicrophoneUsageDescription`. (Required for
   the eventual production bundle anyway.)
2. For **dev**, embed that `Info.plist` directly into the unbundled binary via a linker
   flag in `build.rs`, so macOS can read the usage string straight from the executable's
   Mach-O `__info_plist` section:

   ```rust
   // build.rs
   if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
       println!("cargo:rerun-if-changed=Info.plist");
       let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
       println!(
           "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{manifest_dir}/Info.plist"
       );
   }
   ```

   This is Apple's documented technique for command-line / unbundled tools that need
   TCC permissions. With the section embedded, macOS finally prompts, the user grants,
   and the app appears in the Microphone settings list.

### Gotcha that wasted time
Just adding `Info.plist` did nothing, because **cargo does not relink the binary when a
non-source file changes** — the restart showed `Finished ... in 0.51s`, i.e. it reused
the old binary. Editing `build.rs` is what forces the relink (and `rerun-if-changed`
makes future `Info.plist` edits re-trigger it).

### Interview talking points (the 30-second version)
- "Mic worked in a unit test but not the app — same code. That told me it wasn't the
  code, it was the *environment*."
- "macOS mic permission is per-executable. The test inherited the terminal's grant; the
  GUI app is a different binary with no grant."
- "No permission *prompt* at all was the tell: without an `NSMicrophoneUsageDescription`,
  macOS silently denies instead of asking."
- "In `tauri dev` there's no `.app` bundle, so the usage string never reaches the OS. I
  embedded the `Info.plist` into the dev binary with a linker `-sectcreate` flag."
- "And a cargo subtlety: changing a data file doesn't relink; changing `build.rs` does."

### Transferable lesson
When something works in one execution context and not another with identical code, stop
looking at the code and start enumerating what the *environment* provides differently —
here, process identity and OS-level permissions.

### CORRECTION (after deeper investigation)
The `build.rs` linker-embed above did **not** fix it. Inspecting the built binary showed why:
- `otool -P target/debug/tauri-audio` revealed Tauri **already embeds** the merged
  `Info.plist` (with the mic usage string) into the dev binary — so the linker hack was
  redundant and only created a duplicate `__info_plist` section. It was reverted.
- `codesign -dvvv` showed the binary is ad-hoc, **linker-signed**, with
  **`Info.plist=not bound`**. macOS TCC only honors a usage string that is *bound into the
  code signature*. Re-signing by hand (`codesign --force --sign -`) did not bind it either.

Real root cause: `tauri dev` runs a **non-bundled, ad-hoc-signed GUI binary**. macOS only
reliably grants the mic to a signed `.app` **bundle**; for a bundle-less GUI app the usage
string isn't bound, so TCC never prompts and silently feeds zeros. Known limitation
(tauri#11951): permissions work in production builds, not in `tauri dev`.

Real resolution: the capture code is correct (proven by the `transcribes_my_voice` test,
which inherits the terminal's mic grant). In-app mic permission is a **packaging** concern:
`tauri build` produces a signed `.app` where TCC works. So it's deferred to the packaging
milestone; mic-dependent logic is developed via `cargo test` until then.

Corrected lesson: a "fix" isn't a fix until verified against the real artifact. The
linker-embed *looked* right and even put the plist in the binary — but `otool`/`codesign`
on the actual executable proved it never took effect. Inspect the artifact; don't assume
the build step did what you intended.

---------

06.12.2026
- pnpm 10+ refuses to run dpendencies install scripts by default.
- needed to enable esbuild manually

06.12.2026

- Fresh Tauri scaffold won't compile: `time` crate vs. rustc coherence (E0119)

### Symptom
A brand-new, untouched `cargo create-tauri-app` project failed to compile with
`error[E0119]: conflicting implementations of trait` inside dependencies `cookie` and
`tauri-utils` — all originating in the `time` crate (v0.3.47/0.3.48). No code of mine
was involved; it was the scaffold's own dependency tree.

### What made it confusing
- The error pointed at three unrelated blanket impls (`From<...> for AssetKey`,
  `... for Value`, `... for Expiration`) all "conflicting" with the *same* internal
  `time` impl — a nonsensical, copy-pasted conflict that screamed "compiler bug," not
  "real overlap."
- It failed on **both** rustc 1.93.1 (latest stable) and 1.92.0, so it wasn't a
  brand-new regression to wait out.

### Root cause
rustc's **next-generation coherence solver** (stricter by design, default since ~1.84)
flags `time` 0.3.47+'s internal `ModifierValue`/`HourBase` impl as overlapping with the
blanket `From<T>` impls in `cookie`/`tauri-utils`. It's intended stricter behavior, not
a revertible regression — and not fixed on nightly (newest = strictest solver). Meanwhile
the modern Tauri tree **floors `time` at >= 0.3.47** in multiple places (`plist` 1.9.0,
`serde_with` 3.21.0), so the broken version is hard to escape.

### Diagnosis (what didn't work, and why that was informative)
- Update `time` *forward*: 0.3.48 was already latest. No escape upward.
- Pin the toolchain to 1.92.0: **same error** -> proved it's not a 1.93-only regression,
  so the toolchain lever was a dead end.
- Downgrade `time` at the leaf: rejected — `cargo update -p time --precise 0.3.46` showed
  `plist` *then* `serde_with` both require `>= 0.3.47`. So multiple crates pin it high.

### Fix
Add one direct constraint to `src-tauri/Cargo.toml` and let cargo's resolver cascade:

```toml
time = "=0.3.46"
```

Cargo backtracked automatically: `serde_with` 3.21.0 -> 3.17.0, `plist` 1.9.0 -> 1.8.0,
`time` 0.3.48 -> 0.3.46 (the last version *without* the impl that trips the solver).
`cargo tree -i time` confirmed a single, clean 0.3.46 in the graph.

### Interview talking points
- "A fresh scaffold didn't compile — so first I proved it wasn't my code, it was the
  dependency tree colliding with the compiler's coherence solver."
- "Cargo picks newest-compatible by default, which walked me straight into the broken
  `time`. The fix was to pin it *down* and let the resolver drag the rest with it."
- "I ruled out the toolchain lever empirically — pinning rustc to 1.92 reproduced it —
  before settling on the dependency lever."

### Transferable lesson
`cargo tree -i <crate>` and `cargo update -p <crate> --precise <ver>` are how you find
and pin around a bad transitive dependency. One well-placed `=` pin can cascade-downgrade
a whole subtree. Document the pin (and why) so it can be removed when upstream fixes it.

---------

06.12.2026 - 06.15.2026

- Example code won't compile: API drift across crate versions (`whisper-rs`, `cpal`)

### Symptom
Code copied from crate docs / online examples failed to compile against the *installed*
versions, with errors like "no method named `name`", "u32 doesn't have fields", and
"expected `StreamConfig`, found `&_`".

### Root cause
Crate public APIs change between versions, and examples (and model training data) lag
behind. Concrete drifts hit this session:
- **whisper-rs 0.16**: segment text comes from `state.as_iter()` + each segment's
  `Display` impl — *not* the older `full_n_segments()` / `full_get_segment_text(i)` loop
  most examples show.
- **cpal 0.18**: `Device::name()` was removed (use the `Display` impl); `SampleRate`
  became a plain `u32` alias (so no `.0` tuple field); `build_input_stream` takes the
  config **by value**, not `&config`.

### Fix / method
Treat the **installed crate source on disk as ground truth**, not online examples:
`~/.cargo/registry/src/index.crates.io-*/<crate>-<version>/` — read its `examples/` and
`src/` for the exact API. Also: trust the compiler's `help:` suggestions — rustc
literally printed the `name()` -> `Display` hint and a `- &config / + config` diff for
the by-value fix.

### Interview talking points
- "When example code doesn't compile, I stop trusting the example and read the installed
  version's own source — it's the only thing guaranteed to match what I'm building against."
- "rustc's error messages are verbose but high-signal — the `help:` block often *is* the
  fix; I just apply the suggested diff."

### Transferable lesson
Pin your mental model to the version you actually have, not the one the internet
remembers. `~/.cargo/registry/src/.../<crate>-<ver>/examples/` is the fastest source of
truth for a Rust crate's real API.
