//! Common imports for building an `egelm` application.
//!
//! Importing this module brings the widget traits, application types, derive
//! macro, selected `egui` types, and [`crate::emoji::emoji`] into scope.
//!
//! # Examples
//!
//! ```
//! use egelm::prelude::*;
//!
//! let options = ViewportBuilder::default().with_title("Example");
//! assert_eq!(options.title.as_deref(), Some("Example"));
//! ```

#[cfg(feature = "emoji")]
pub use crate::emoji::emoji;

pub(crate) use crate::error::Error;
pub use crate::native::{Frame, Handle};
pub use crate::window::{App, Context, LeafWidget, Managed, RootWidget, Sender, TickChildren, Widget};
pub use egelm_macros::Widget;
pub use egui::{self, TextBuffer, ViewportBuilder};
