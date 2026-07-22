//! Easily use Emojis in your egelm app

/// An Emoji type. Implements `ToString` for `Label`s
///
/// Get an `Emoji` by using [`emoji`]
pub struct Emoji {
	inner: &'static emojis::Emoji,
}

impl AsRef<emojis::Emoji> for Emoji {
	fn as_ref(&self) -> &'static emojis::Emoji {
		self.inner
	}
}

impl From<&'static emojis::Emoji> for Emoji {
	fn from(value: &'static emojis::Emoji) -> Self {
		Self { inner: value }
	}
}

impl std::fmt::Display for Emoji {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.inner.as_str())
	}
}

impl From<Emoji> for String {
	fn from(value: Emoji) -> Self {
		value.to_string()
	}
}

/// Lookup an emoji in *O(1)* time using `gemoji` shortcode: [here](https://www.webfx.com/tools/emoji-cheat-sheet).
///
/// Returns an [`Emoji`] that you can use in [`egui::Label`]s.
///
/// If the Emoji does not exist, it is replaced by a default and a [`tracing::warn`] will be created.
///
/// # Examples
///
/// ```
/// # use egelm::prelude::*;
/// assert_eq!(emoji("rocket").to_string(), "🚀");
/// assert_eq!(emoji("something_that_does_not_exist").to_string(), "⛔");
/// ```
///
/// ```
/// # use egelm::prelude::*;
/// # egelm::window::run_test(|ui| {
/// ui.label(format!("rocket emoji: {}", emoji("rocket")));
/// # })?;
/// # Ok::<(), egelm::error::Error>(())
/// ```
pub fn emoji(name: impl AsRef<str> + std::fmt::Display) -> Emoji {
	match emojis::get_by_shortcode(name.as_ref()).map(|x| x.into()) {
		Some(o) => o,
		None => {
			tracing::warn!(":{name}: does not exist");
			emojis::get_by_shortcode("no_entry")
				.map(|x| x.into())
				.unwrap()
		}
	}
}
