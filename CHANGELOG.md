## [0.6.7] - 2026-09-09

- Fix debug build label on some app layouts

## [0.6.6] - 2026-09-08

- Increase text visibility for some themes
- Make error dialog shrink when the window is too small

## [0.6.4] - 2026-09-07

- Fix tests

## [0.6.3] - 2026-09-07

- Fix testing workflow

## [0.6.2] - 2026-09-07

- Fix testing workflow

## [0.6.1] - 2026-09-07

- Fix compiling with theming feature off

## [0.6.0] - 2026-09-07

- Add storage feature
- Implement FileBackend storage via RON and MemoryBackend as a fallback
- Fix window being blank with glow after showing the window
- Add theming
- Add egui demo as an example

## [0.5.0] - 2026-07-29

- Update glow renderer to be more similar to wgpu
- Unify most renderer code into a shared runner

## [0.4.1] - 2026-07-26

- Fix docs.rs

## [0.4.0] - 2026-07-26

- Add widget lifecycle and shutdown hooks
- Add cancellable async tasks with result routing
- Add accessibility support
- Add wgpu screenshot support
- Add platform and renderer cfg aliases
- Exclude `android` example from workflow testing/linting

## [0.3.2] - 2026-07-25

- Add more examples
- Add more logging

## [0.3.1] - 2026-07-25

- Remove common commits from changelog
- Add unit tests
- Remove unused errors

## [0.3.0] - 2026-07-25

- Add rustfmt.toml file
- Add android support
- Add platform-specific features
- Add android eventloop entry point and native activity exports
- Handle android suspend and resume events by destroying and recreating the window
- Replace the global type-erased error channel with a typed per-application channel
- Create buildable android example
- Document the currently supported platforms
- Exclude android member from release

## [0.2.2] - 2026-07-24

- Update the about_dialog api
- Make fields in about_dialog optional
- Remove the need for AboutDialogSettings
- Create testing workflow

## [0.2.1] - 2026-07-23

- Ability to change the bg color
- **actually** make `wgpu` the default renderer

## [0.2.0] - 2026-07-23

- Add wgpu renderer
- Make wgpu the default
- Make `Runner` generic
- Fix `emoji` feature gate
- Remove unused `Widget::init` function

## [0.1.1] - 2026-07-23

- Fix release commit message
- Fix git-cliff

## [0.1.0] - 2026-07-23

- Init commit
- Add docs
- Call tick_children_auto() within update_route_error()
- Add cargo-release and git-cliff
- Edit cliff.toml

