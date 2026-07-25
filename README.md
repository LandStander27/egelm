# egelm

[![Validation](https://codeberg.org/Land/egelm/badges/workflows/test.yml/badge.svg?branch=master)](https://codeberg.org/Land/egelm/actions?workflow=test.yml)
[![crates.io](https://img.shields.io/crates/v/egelm.svg)](https://crates.io/crates/egelm)
[![docs.rs](https://docs.rs/egelm/badge.svg)](https://docs.rs/egelm)
[![License](https://img.shields.io/crates/l/egelm.svg)](LICENSE)

`egelm` is a small app framework for building native GUI applications
in Rust with [`egui`](https://github.com/emilk/egui). It adds a typed,
message-driven widget lifecycle, child-widget management, asynchronous tasks,
error routing, and reusable dialogs on top of `egui`.

Currently, `egelm` supports Linux, Android, and Windows only. For an Android
example, see [`examples/android`](examples/android).

## Table of contents

- [Features](#features)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Examples](#examples)
- [How it works](#how-it-works)
- [AI usage](#ai-usage)
- [License](#license)

## Features

- Typed messages, outputs, and errors for each widget
- `Managed<T>` child widgets with automatic message processing
- A `#[derive(Widget)]` macro that updates managed children
- Background work through Tokio
- Window hide/show controls and customizable close behavior
- Root-level error handling with a built-in error dialog
- Reusable about dialog and emoji helpers

## Installation

Add `egelm` and Tokio to your application:

```toml
[dependencies]
egelm = "0.3.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Or via `cargo add`:

```sh
cargo add egelm tokio --features 'tokio/rt-multi-thread tokio/macros'
```

The default `egelm` features enable Ctrl-C handling, tracing, a quick
emoji lookup, and `wgpu` rendering.

## Quick start

```rust
use egelm::prelude::*;

#[derive(Debug)]
enum Message {
    Increment,
}

#[derive(Debug, Widget)]
struct Counter {
    count: u32,
}

impl Widget for Counter {
    type Message = Message;
    type Output = ();
    type Error = ();

    fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
        ui.heading(format!("Count: {}", self.count));

        if ui.button("Increment").clicked() {
            ctx.emit(Message::Increment);
        }
    }

    fn update(&mut self, message: Self::Message, _handle: &Handle, _ctx: &Context<Self>) -> Result<(), Self::Error> {
        match message {
            Message::Increment => self.count += 1,
        }

        Ok(())
    }
}

impl RootWidget for Counter {}

#[tokio::main]
async fn main() {
    App::new(Counter { count: 0 })
        .run(ViewportBuilder::default().with_title("Counter"))
        .unwrap();
}
```

Run the application with:

```sh
cargo run
```

## How it works

An egelm application is composed of widgets:

- `Widget` is the full lifecycle trait. Its `view` method renders UI, while
  `update` handles typed messages and `tick` performs per-cycle work.
- `LeafWidget` is a simpler rendering-only trait for components that do not
  need messages, outputs, or errors.
- `RootWidget` represents the top-level application and can customize window
  closing, error presentation, and initial `egui` setup.
- `Context<W>` lets a widget emit messages, send output to its parent, report
  errors, and spawn asynchronous tasks.
- `Managed<W>` owns a child widget together with its message queue and context.

Deriving `Widget` implements child ticking for all named `Managed<T>` fields:

```rust
#[derive(Debug, Widget)]
struct Parent {
    child: Managed<Child>,
}
```

This keeps child message routing automatic while preserving concrete,
compile-time message types.

## AI usage

AI tools are used only to help create documentation, including this README.
No AI-assisted content is committed without first being read and reviewed by a
human.

## License

This project is available under the [MIT License](LICENSE).
