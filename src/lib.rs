pub mod emoji;
pub mod error;
pub mod native;
pub mod prelude;
pub mod window;

pub mod widgets {
	//! Contains common `egelm` widgets you can use
	pub use crate::window::about_dialog::{AboutDialog, AboutDialogSettings};

	/// Can be used to easily `use` common `egui` imports, as well as the widgets contained in `super`
	pub mod prelude {
		pub use super::*;
		pub use egui::{Align, Color32, Image, Label, Layout, Modal, RichText, ScrollArea, TextEdit, TextStyle, TextWrapMode};
	}
}
