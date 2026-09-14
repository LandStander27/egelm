//! In-app toast notifications.
//!
//! Toasts present temporary feedback or alerts in a non-intrusive overlay floating
//! above the application view.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use egui::{Align2, Area, Color32, CornerRadius, Frame, Id, Order, Pos2, Rect, RichText, Sense, Vec2};

use crate::window::{Context, LeafWidget, Widget};

static TOAST_CHANNEL: OnceLock<(Sender<ActiveToast>, Receiver<ActiveToast>)> = OnceLock::new();

fn toast_channel() -> &'static (Sender<ActiveToast>, Receiver<ActiveToast>) {
	TOAST_CHANNEL.get_or_init(crossbeam_channel::unbounded)
}

pub(crate) fn add_toast(toast: ActiveToast) {
	let (tx, _) = toast_channel();
	_ = tx.send(toast);
}

pub(crate) fn iter_toasts() -> impl Iterator<Item = ActiveToast> {
	let (_, rx) = toast_channel();
	rx.try_iter()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ToastKind {
	#[default]
	Info,
	Success,
	Warning,
	Error,
}

impl ToastKind {
	#[cfg(feature = "emoji")]
	fn icon(&self) -> crate::emoji::Emoji {
		match self {
			Self::Info => crate::emoji::emoji("information_source"),
			Self::Success => crate::emoji::emoji("white_check_mark"),
			Self::Warning => crate::emoji::emoji("warning"),
			Self::Error => crate::emoji::emoji("x"),
		}
	}

	#[cfg(not(feature = "emoji"))]
	fn icon(&self) -> &'static str {
		match self {
			Self::Info => "ℹ️",
			Self::Success => "✅",
			Self::Warning => "⚠️",
			Self::Error => "❌",
		}
	}

	fn accent_color(&self, visuals: &egui::Visuals) -> Color32 {
		match self {
			Self::Info => visuals.hyperlink_color,
			Self::Warning => visuals.warn_fg_color,
			Self::Error => visuals.error_fg_color,
			Self::Success => {
				if visuals.dark_mode {
					Color32::from_rgb(100, 220, 140)
				} else {
					Color32::from_rgb(30, 160, 60)
				}
			}
		}
	}
}

/// A toast notification configuration.
///
/// Build a toast using chainable builder methods and dispatch it via [`Context::toast`].
///
/// # Examples
///
/// ```ignore
/// ctx.toast(
///     Toast::new("There was an error")
///         .error()
///         .body("long error description")
///         .duration(Duration::from_secs(5))
///         .action("Retry", |ctx| ctx.emit(Message::Retry))
/// );
/// ```
pub struct Toast<W: Widget + 'static> {
	pub(crate) id: u64,
	pub(crate) kind: ToastKind,
	pub(crate) title: String,
	pub(crate) body: Option<String>,
	pub(crate) duration: Option<Duration>,
	pub(crate) closable: bool,

	#[allow(clippy::type_complexity)]
	pub(crate) actions: Vec<(String, Box<dyn FnOnce(&Context<W>) + Send + 'static>)>,
}

impl<W: Widget> std::fmt::Debug for Toast<W> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Toast")
			.field("id", &self.id)
			.field("kind", &self.kind)
			.field("title", &self.title)
			.field("body", &self.body)
			.field("duration", &self.duration)
			.field("closable", &self.closable)
			.finish()
	}
}

impl<W: Widget> Toast<W> {
	/// Creates a new toast notification with the given title.
	pub fn new(title: impl Into<String>) -> Self {
		static COUNTER: AtomicU64 = AtomicU64::new(1);
		Self {
			id: COUNTER.fetch_add(1, Ordering::Relaxed),
			kind: ToastKind::Info,
			title: title.into(),
			body: None,
			duration: Some(Duration::from_secs(5)),
			closable: true,
			actions: Vec::new(),
		}
	}

	/// Sets the toast kind to informational.
	pub fn info(mut self) -> Self {
		self.kind = ToastKind::Info;
		self
	}

	/// Sets the toast kind to success.
	pub fn success(mut self) -> Self {
		self.kind = ToastKind::Success;
		self
	}

	/// Sets the toast kind to warning.
	pub fn warning(mut self) -> Self {
		self.kind = ToastKind::Warning;
		self
	}

	/// Sets the toast kind to error.
	pub fn error(mut self) -> Self {
		self.kind = ToastKind::Error;
		self
	}

	/// Sets secondary body description text.
	pub fn body(mut self, body: impl Into<String>) -> Self {
		self.body = Some(body.into());
		self
	}

	/// Sets the duration before the toast automatically dismisses.
	pub fn duration(mut self, duration: Duration) -> Self {
		self.duration = Some(duration);
		self
	}

	/// Makes this toast persistent until manually closed by the user.
	pub fn sticky(mut self) -> Self {
		self.duration = None;
		self
	}

	/// Configures whether a manual close `✕` button is shown.
	pub fn closable(mut self, closable: bool) -> Self {
		self.closable = closable;
		self
	}

	/// Adds an interactive action button that executes a callback with the widget's context when clicked.
	pub fn action(mut self, label: impl Into<String>, callback: impl FnOnce(&Context<W>) + Send + 'static) -> Self {
		self.actions.push((label.into(), Box::new(callback)));
		self
	}

	pub(crate) fn into_active(self, ctx: &Context<W>) -> ActiveToast {
		let actions = self
			.actions
			.into_iter()
			.map(|(label, action)| {
				let ctx = ctx.clone();
				let erased: Box<dyn FnOnce() + Send + 'static> = Box::new(move || action(&ctx));
				(label, Some(erased))
			})
			.collect();

		ActiveToast {
			id: self.id,
			kind: self.kind,
			title: self.title,
			body: self.body,
			duration: self.duration,
			closable: self.closable,
			actions,
			created_at: Instant::now(),
			paused_duration: Duration::ZERO,
			last_hovered: None,
			dismissed: false,
		}
	}
}

pub(crate) struct ActiveToast {
	pub(crate) id: u64,
	pub(crate) kind: ToastKind,
	pub(crate) title: String,
	pub(crate) body: Option<String>,
	pub(crate) duration: Option<Duration>,
	pub(crate) closable: bool,
	#[allow(clippy::type_complexity)]
	pub(crate) actions: Vec<(String, Option<Box<dyn FnOnce() + Send + 'static>>)>,
	pub(crate) created_at: Instant,
	pub(crate) paused_duration: Duration,
	pub(crate) last_hovered: Option<Instant>,
	pub(crate) dismissed: bool,
}

impl std::fmt::Debug for ActiveToast {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Toast")
			.field("id", &self.id)
			.field("kind", &self.kind)
			.field("title", &self.title)
			.field("body", &self.body)
			.field("duration", &self.duration)
			.field("closable", &self.closable)
			.finish()
	}
}

impl ActiveToast {
	fn remaining_ratio(&self) -> Option<f32> {
		let total = self.duration?;
		let elapsed = (Instant::now().saturating_duration_since(self.created_at)).saturating_sub(self.paused_duration);
		if elapsed >= total {
			Some(0.0)
		} else {
			Some(1.0 - (elapsed.as_secs_f32() / total.as_secs_f32()))
		}
	}

	fn is_expired(&self) -> bool {
		self.dismissed || self.remaining_ratio().is_some_and(|r| r <= 0.0)
	}
}

#[derive(Default, Debug)]
pub(crate) struct Toasts {
	pub(crate) toasts: Vec<ActiveToast>,
	pub(crate) margin: Vec2,
	pub(crate) spacing: f32,
	pub(crate) width: f32,
}

impl Toasts {
	pub(crate) fn new() -> Self {
		Self {
			toasts: Vec::new(),
			margin: Vec2::new(16.0, 16.0),
			spacing: 8.0,
			width: 300.0,
		}
	}

	pub(crate) fn add(&mut self, toast: ActiveToast) {
		self.toasts.push(toast);
	}
}

impl LeafWidget for Toasts {
	fn render(&mut self, ui: &mut egui::Ui, _frame: &mut super::Frame) {
		if self.toasts.is_empty() {
			return;
		}

		let ctx = ui.ctx();

		ctx.request_repaint_after(Duration::from_millis(16));

		let mut actions_to_run = Vec::new();
		let screen_rect = ctx.content_rect();
		let anchor_pos = Pos2::new(screen_rect.right() - self.margin.x, screen_rect.bottom() - self.margin.y);

		Area::new(Id::new("egelm_toasts_area"))
			.order(Order::Foreground)
			.anchor(Align2::CENTER_BOTTOM, Vec2::ZERO)
			.fixed_pos(anchor_pos)
			.show(ctx, |ui| {
				ui.set_width(self.width);

				let style = ui.style().clone();
				let visuals = &style.visuals;

				for active in self.toasts.iter_mut() {
					if active.dismissed {
						continue;
					}

					let accent = active.kind.accent_color(visuals);

					let frame = Frame::window(&style).inner_margin(egui::Margin::symmetric(12, 10));

					let response = frame.show(ui, |ui| {
						ui.set_width(self.width);

						ui.horizontal(|ui| {
							ui.label(
								RichText::new(active.kind.icon())
									.size(15.0)
									.color(accent)
									.strong(),
							);
							ui.add_space(4.0);

							ui.vertical(|ui| {
								ui.label(
									RichText::new(&active.title)
										.size(13.0)
										.color(visuals.text_color())
										.strong(),
								);
								if let Some(body) = &active.body {
									ui.add_space(2.0);
									ui.label(
										RichText::new(body)
											.size(12.0)
											.color(visuals.weak_text_color()),
									);
								}
							});

							ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
								if active.closable
									&& ui
										.button(
											#[cfg(feature = "emoji")]
											RichText::new(crate::emoji::emoji("x"))
												.size(11.0)
												.color(visuals.weak_text_color()),
											#[cfg(not(feature = "emoji"))]
											RichText::new("❌")
												.size(11.0)
												.color(visuals.weak_text_color()),
										)
										.clicked()
								{
									active.dismissed = true;
								}

								for (label, action_cb) in &mut active.actions {
									if ui
										.button(RichText::new(&*label).size(12.0).color(accent))
										.clicked()
									{
										active.dismissed = true;
										if let Some(action) = action_cb.take() {
											actions_to_run.push(action);
										}
									}
								}
							});
						});

						if let Some(ratio) = active.remaining_ratio() {
							ui.add_space(6.0);
							let progress_height = 2.0;
							let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), progress_height), Sense::hover());
							let filled_width = rect.width() * ratio;
							let progress_rect = Rect::from_min_size(rect.min, Vec2::new(filled_width, progress_height));

							ui.painter()
								.rect_filled(rect, CornerRadius::same(1), visuals.faint_bg_color);
							ui.painter()
								.rect_filled(progress_rect, CornerRadius::same(1), accent);
						}
					});

					if response.response.hovered() {
						let now = Instant::now();
						if let Some(last) = active.last_hovered {
							active.paused_duration += now.saturating_duration_since(last);
						}
						active.last_hovered = Some(now);
					} else {
						active.last_hovered = None;
					}

					ui.add_space(self.spacing);
				}
			});

		for action in actions_to_run {
			action();
		}

		self.toasts.retain(|t| !t.is_expired());
	}
}
