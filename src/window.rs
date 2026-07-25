//! Define widgets, route their messages, and run an application.
//!
//! [`Widget`](crate::window::Widget) describes the event-driven lifecycle.
//! [`LeafWidget`](crate::window::LeafWidget) is a simpler rendering-only
//! alternative, while [`RootWidget`](crate::window::RootWidget) adds behavior for
//! the top-level application window. [`Context`](crate::window::Context) and
//! [`Sender`](crate::window::Sender) carry typed
//! messages between those widgets.
//!
//! # Examples
//!
//! ```
//! use egelm::prelude::*;
//!
//! #[derive(Debug)]
//! struct Counter(u32);
//!
//! impl LeafWidget for Counter {
//!     fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
//!         if ui.button(self.0.to_string()).clicked() {
//!             self.0 += 1;
//!         }
//!     }
//! }
//! ```

use std::sync::Arc;

use egui_winit::winit::event_loop::EventLoop;

use crate::prelude::*;

/// A reusable modal dialog containing application and license information.
pub mod about_dialog;
pub(crate) mod error_dialog;

/// A cloneable callback for delivering typed widget messages.
///
/// Senders are obtained from a [`Context`] and can be moved into callbacks or
/// transformed with [`map`](Self::map).
pub struct Sender<M> {
	f: Arc<dyn Fn(M) + Send + Sync>,
}

impl<M> Clone for Sender<M> {
	fn clone(&self) -> Self {
		Self { f: self.f.clone() }
	}
}

impl<M: 'static> Sender<M> {
	fn new(f: impl Fn(M) + Send + Sync + 'static) -> Self {
		Self { f: Arc::new(f) }
	}

	/// Delivers a message to this sender's callback.
	pub fn emit(&self, msg: M) {
		(self.f)(msg)
	}

	/// Creates a sender that transforms its input before forwarding it.
	///
	/// This is useful when a child component produces a different message type
	/// than its parent consumes.
	pub fn map<N: 'static>(&self, f: impl Fn(N) -> M + Send + Sync + 'static) -> Sender<N> {
		let inner = self.clone();
		Sender::new(move |n| inner.emit(f(n)))
	}
}

/// Communication channels and task utilities for a [`Widget`].
///
/// A context lets a widget enqueue its own messages, report errors, send
/// output to its parent, and start asynchronous work. Clones refer to the same
/// underlying channels.
pub struct Context<W: Widget> {
	input: Sender<W::Message>,
	error: Sender<W::Error>,
	output: Option<Sender<W::Output>>,
}

impl<W: Widget> Clone for Context<W> {
	fn clone(&self) -> Self {
		Self {
			error: self.error.clone(),
			input: self.input.clone(),
			output: self.output.clone(),
		}
	}
}

impl<W: Widget> std::fmt::Debug for Context<W> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_fmt(format_args!(
			"Context<Message = {}, Output = {}, Error = {}>",
			std::any::type_name::<W::Message>(),
			std::any::type_name::<W::Output>(),
			std::any::type_name::<W::Error>()
		))
	}
}

impl<W: Widget + 'static> Context<W> {
	/// Spawns an asynchronous task with a clone of this context.
	///
	/// The task can emit messages or errors after performing background work.
	///
	/// # Panics
	///
	/// Panics if called outside a Tokio runtime.
	pub fn spawn<F, Fut>(&self, f: F)
	where
		F: FnOnce(Context<W>) -> Fut + Send + 'static,
		Fut: Future<Output = ()> + Send + 'static,
	{
		let ctx = self.clone();

		tokio::spawn(async move {
			f(ctx).await;
		});
	}

	/// Enqueues a message for this widget and requests a repaint.
	pub fn emit(&self, msg: W::Message) {
		self.input.emit(msg);
	}

	/// Sends an output value to this widget's parent.
	///
	/// If the widget has no output channel, the value is discarded and a
	/// warning is logged.
	pub fn output(&self, msg: W::Output) {
		if let Some(output) = &self.output {
			output.emit(msg);
		} else {
			tracing::warn!("`output` channel does not exist");
		}
	}

	/// Routes an error toward the root widget's error handler.
	pub fn error(&self, err: W::Error) {
		self.error.emit(err);
	}

	/// Returns a cloneable sender for errors from this widget.
	pub fn error_sender(&self) -> Sender<W::Error> {
		self.error.clone()
	}

	/// Returns a cloneable sender for messages to this widget.
	pub fn input_sender(&self) -> Sender<W::Message> {
		self.input.clone()
	}
}

/// An event-driven user-interface component.
///
/// A widget renders itself in [`view`](Self::view), handles queued messages in
/// [`update`](Self::update), and may perform per-cycle work in
/// [`tick`](Self::tick). Deriving [`egelm_macros::Widget`] implements child
/// ticking for structs containing [`Managed`] fields.
///
/// # Examples
///
/// ```
/// use egelm::prelude::*;
///
/// #[derive(Debug, Widget)]
/// struct Label(String);
///
/// impl Widget for Label {
///     type Message = String;
///     type Output = ();
///     type Error = ();
///
///     fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, _ctx: &Context<Self>) {
///         ui.label(&self.0);
///     }
///
///     fn update(&mut self, message: Self::Message, _handle: &Handle, _ctx: &Context<Self>) -> Result<(), Self::Error> {
///         self.0 = message;
///         Ok(())
///     }
/// }
/// ```
pub trait Widget: TickChildren + std::fmt::Debug + Sized {
	/// Messages consumed by [`update`](Self::update).
	type Message: Send + std::fmt::Debug + 'static;
	/// Values this widget can send to its parent.
	type Output: Send + 'static;
	/// Errors produced while updating or ticking this widget.
	type Error: std::fmt::Debug + Send + Sync + 'static;

	/// Renders the widget into the current `egui` user interface.
	fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, ctx: &Context<Self>);

	/// Handles one queued message.
	///
	/// The default implementation ignores the message.
	///
	/// # Errors
	///
	/// Returns a widget-defined error when the message cannot be handled. The
	/// framework routes it to the root widget's [`RootWidget::error`] method.
	#[expect(unused_variables)]
	fn update(&mut self, msg: Self::Message, handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
		Ok(())
	}

	/// Performs work once during an application update cycle.
	///
	/// The default implementation does nothing.
	///
	/// # Errors
	///
	/// Returns a widget-defined error when the tick fails. The framework routes
	/// it to the root widget's [`RootWidget::error`] method.
	#[expect(unused_variables)]
	fn tick(&mut self, ctx: &Context<Self>) -> Result<(), Self::Error> {
		Ok(())
	}

	// /// Provides an optional widget-initialization hook.
	// ///
	// /// The default implementation does nothing. Callers that manage a widget's
	// /// lifecycle may invoke this hook with its communication context.
	// #[expect(unused_variables)]
	// fn init(&mut self, ctx: &Context<Self>) {}
}

/// A rendering-only widget with no messages, output, or errors.
///
/// Implementing this trait automatically implements [`Widget`] and
/// [`TickChildren`] with unit associated types.
///
/// # Examples
///
/// ```
/// use egelm::prelude::*;
///
/// #[derive(Debug)]
/// struct Heading;
///
/// impl LeafWidget for Heading {
///     fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
///         ui.heading("Settings");
///     }
/// }
/// ```
pub trait LeafWidget: std::fmt::Debug {
	/// Renders the leaf widget into the current `egui` user interface.
	fn render(&mut self, ui: &mut egui::Ui, frame: &mut Frame);
}

impl<T: LeafWidget> TickChildren for T {}
impl<T: LeafWidget> Widget for T {
	type Message = ();
	type Output = ();
	type Error = ();

	#[inline(always)]
	fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, _ctx: &Context<Self>) {
		self.render(ui, frame);
	}

	#[inline(always)]
	fn update(&mut self, _msg: (), _handle: &Handle, _ctx: &Context<Self>) -> Result<(), ()> {
		Ok(())
	}

	#[inline(always)]
	fn tick(&mut self, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		Ok(())
	}
}

/// Extra behavior for the root widget owned by an [`App`].
///
/// Implement this trait to customize window closing, error presentation, and
/// one-time rendering setup.
pub trait RootWidget: Widget + 'static {
	/// Handles a native request to close the window.
	///
	/// The default implementation exits the application. An implementation may
	/// hide the window or ask for confirmation instead.
	fn close(&mut self, frame: &mut Frame) {
		frame.exit();
	}

	/// Converts a widget error into dialog text.
	///
	/// The returned tuple contains a summary and optional details. The default
	/// implementation reports that error translation is not implemented.
	#[expect(unused_variables)]
	fn error(&mut self, err: &Self::Error) -> (String, Option<String>) {
		(
			"Unknown error".to_string(),
			Some("Error translating is not implemented.\nImplement RootWidget::error in order to view the actual error that happened.".to_string()),
		)
	}

	/// Configures `egui` after a native rendering context is created.
	///
	/// The default implementation does nothing. The method may be called again
	/// when a hidden window is shown and recreated.
	#[expect(unused_variables)]
	fn setup(&mut self, ctx: &egui::Context) {}

	/// Set the RGBA clear color for the native window. The default implementation returns a dark gray color.
	fn clear_color(&self) -> [u8; 4] {
		[27, 27, 27, 255]
	}
}

/// A widget together with its message queue and communication context.
///
/// `Managed<T>` dereferences to `T`, allowing direct access to the wrapped
/// widget. Its update methods drain queued messages before ticking the widget.
#[derive(Debug)]
pub struct Managed<T: Widget> {
	widget: T,
	rx: crossbeam_channel::Receiver<T::Message>,
	ctx: Context<T>,
	pub(crate) handle: Handle,
}

impl<T: Widget> std::ops::Deref for Managed<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.widget
	}
}

impl<T: Widget> std::ops::DerefMut for Managed<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.widget
	}
}

impl<T: Widget + 'static> Managed<T> {
	/// Wraps a widget with input, output, and error routing.
	///
	/// `output` may be `None` for a widget whose output should be discarded.
	/// Messages sent through the managed widget's context wake the supplied
	/// application `handle`.
	pub fn new(output: impl Into<Option<Sender<T::Output>>>, error: Sender<T::Error>, handle: &Handle, widget: T) -> Self {
		let (tx, rx) = crossbeam_channel::unbounded();
		let wake = handle.clone();
		Self {
			widget,
			rx,
			ctx: Context {
				input: Sender::new(move |msg| {
					if let Err(e) = tx.send(msg).map_err(|_| Error::SendingOverChannel) {
						tracing::error!("{e}");
					}
					wake.request_repaint();
				}),
				output: output.into(),
				error,
			},
			handle: handle.clone(),
		}
	}

	// fn new_root(widget: T) -> Self {
	// 	let (tx, rx) = crossbeam_channel::unbounded();
	// 	Self {
	// 		widget,
	// 		rx,
	// 		ctx: Context {
	// 			input: Sender::new(move |msg| {
	// 				if let Err(e) = tx.send(msg).map_err(|_| Error::SendingOverChannel) {
	// 					tracing::error!("{e}");
	// 				}
	// 			}),
	// 			output: None,
	// 			error: Sender::new(move |err| {
	// 				if let Err(e) = ERROR_TX
	// 					.get()
	// 					.expect("ERROR_TX not inited; was App::new called?")
	// 					.send(Box::new(err) as Box<dyn AnyDebug + Send + Sync + 'static>)
	// 					.map_err(|_| Error::SendingOverChannel)
	// 				{
	// 					tracing::error!("{e}");
	// 				}
	// 			}),
	// 		},
	// 	}
	// }

	/// Renders the wrapped widget.
	pub fn render(&mut self, ui: &mut egui::Ui, frame: &mut Frame) {
		self.widget.view(ui, frame, &self.ctx);
	}

	/// Drains messages, ticks the widget, and forwards errors through its context.
	///
	/// Unlike [`update`](Self::update), this method does not tick managed child
	/// widgets and does not return errors to its caller.
	pub fn update_route_error(&mut self) {
		while let Ok(msg) = self.rx.try_recv() {
			if let Err(e) = self.widget.update(msg, &self.handle, &self.ctx) {
				self.ctx.error(e);
			}
		}

		self.widget.tick_children_auto();
		if let Err(e) = self.widget.tick(&self.ctx) {
			self.ctx.error(e);
		}
	}

	/// Drains messages and ticks this widget and its managed children.
	///
	/// # Errors
	///
	/// Returns the first error produced by [`Widget::update`] or [`Widget::tick`]
	/// for the wrapped widget. Child errors are routed through their contexts.
	pub fn update(&mut self) -> Result<(), T::Error> {
		while let Ok(msg) = self.rx.try_recv() {
			self.widget.update(msg, &self.handle, &self.ctx)?;
		}

		self.widget.tick_children_auto();
		self.widget.tick(&self.ctx)
	}
}

/// Updates child widgets managed by a parent widget.
///
/// The [`egelm_macros::Widget`] derive macro implements this trait by calling
/// [`Managed::update_route_error`] on each named `Managed<T>` field.
///
/// # Examples
///
/// ```
/// use egelm::prelude::*;
///
/// #[derive(Debug, Widget)]
/// struct NoChildren;
///
/// let mut widget = NoChildren;
/// widget.tick_children_auto();
/// ```
pub trait TickChildren {
	/// Updates all managed child widgets.
	///
	/// The default implementation does nothing.
	fn tick_children_auto(&mut self) {}
}
// impl<T> TickChildren for T {}

/// Owns a root widget and runs it in a native event loop.
///
/// Create an application with [`new`](Self::new), or use
/// [`new_factory`](Self::new_factory) when construction needs the root context
/// or window handle. Call [`run`](Self::run) once to open its window.
pub struct App<T: RootWidget> {
	root: Managed<T>,
	ctrlc_handler: bool,
	error_rx: crossbeam_channel::Receiver<T::Error>,
}

impl<T: RootWidget> App<T> {
	/// Creates an application from an existing root widget.
	///
	/// # Panics
	///
	/// Panics if another `App` has already been constructed in this process.
	/// `egelm` currently maintains one global root-error channel.
	///
	/// # Examples
	///
	/// ```
	/// use egelm::prelude::*;
	///
	/// #[derive(Debug)]
	/// struct Root;
	/// impl LeafWidget for Root {
	///     fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
	///         ui.label("Ready");
	///     }
	/// }
	/// impl RootWidget for Root {}
	///
	/// let app = App::new(Root);
	/// ```
	pub fn new(root: T) -> Self {
		// let (tx, rx) = crossbeam_channel::unbounded();
		// crate::window::ERROR_RX
		// 	.set(std::sync::Mutex::new(rx))
		// 	.expect("cannot call App::new twice");
		// crate::window::ERROR_TX
		// 	.set(tx)
		// 	.expect("cannot call App::new twice");
		// Self {
		// 	root: Managed::new_root(root),
		// 	ctrlc_handler: true,
		// }

		App::new_factory(move |_, _| root)
	}

	/// Creates an application with access to its context and window handle.
	///
	/// The factory can clone either argument into the root widget before it is
	/// placed in the application.
	///
	/// # Panics
	///
	/// Panics if another `App` has already been constructed in this process.
	/// `egelm` currently maintains one global root-error channel.
	pub fn new_factory<F: FnOnce(&Context<T>, &Handle) -> T>(factory: F) -> Self {
		let (error_tx, error_rx) = crossbeam_channel::unbounded::<T::Error>();

		let handle = Handle::new();
		let (tx, rx) = crossbeam_channel::unbounded();
		let ctx = Context {
			input: Sender::new({
				let handle = handle.clone();
				move |msg| {
					if let Err(e) = tx.send(msg).map_err(|_| Error::SendingOverChannel) {
						tracing::error!("{e}");
					}
					handle.request_repaint();
				}
			}),
			output: None,
			error: Sender::new({
				let handle = handle.clone();
				move |err| {
					if let Err(e) = error_tx.send(err).map_err(|_| Error::SendingOverChannel) {
						tracing::error!("{e}");
					}
					handle.request_repaint();
				}
			}),
		};

		Self {
			root: Managed {
				widget: factory(&ctx, &handle),
				rx,
				ctx,
				handle,
			},
			ctrlc_handler: true,
			error_rx,
		}
	}

	/// Disables installation of the default Ctrl-C exit handler.
	///
	/// This has an effect only when the crate's `ctrlc` feature is enabled.
	pub fn without_ctrl_handler(mut self) -> Self {
		self.ctrlc_handler = false;
		self
	}

	/// Runs the native application event loop on Android with `wgpu` backend.
	///
	/// For more information, see [`App::run`](Self::run).
	#[cfg(all(feature = "android", target_os = "android"))]
	#[tracing::instrument(skip(self, options))]
	pub fn run_android(self, android_app: AndroidApp, options: ViewportBuilder) -> Result<(), Error> {
		use egui_winit::winit::platform::android::EventLoopBuilderExtAndroid;

		let event_loop = EventLoop::<crate::native::UserEvent>::with_user_event()
			.with_android_app(android_app)
			.build()
			.map_err(Error::EventLoopBuildFail)?;

		self.run_with_event_loop(event_loop, crate::native::Renderer::Wgpu, options)
	}

	/// Runs the native application event loop with the default rendering backend
	/// until exit is requested.
	///
	/// The `glow` backend is preferred when its crate feature is enabled.
	/// Otherwise, this uses `wgpu` when the `wgpu` feature is enabled. Use
	/// [`run_with_backend`](Self::run_with_backend) to select a backend at
	/// runtime when both features are enabled.
	///
	/// # Errors
	///
	/// Returns [`Error::EventLoopBuildFail`] if the event loop cannot be created,
	/// [`Error::SetSigHandler`] when the optional Ctrl-C handler cannot be
	/// installed, or [`Error::EventLoopFail`] if the running event loop fails.
	///
	/// # Panics
	///
	/// Platform-window or rendering-backend initialization failures currently
	/// panic. This method may also panic when called from a thread on which
	/// `winit` does not permit event-loop creation.
	///
	/// # Examples
	///
	/// ```no_run
	/// use egelm::prelude::*;
	///
	/// #[derive(Debug)]
	/// struct Root;
	/// impl LeafWidget for Root {
	///     fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
	///         ui.label("Hello");
	///     }
	/// }
	/// impl RootWidget for Root {}
	///
	/// App::new(Root).run(ViewportBuilder::default().with_title("Hello"))?;
	/// # Ok::<(), egelm::error::Error>(())
	/// ```
	#[tracing::instrument(skip(self, options))]
	pub fn run(self, options: ViewportBuilder) -> Result<(), Error> {
		#[cfg(feature = "glow")]
		return self.run_with_backend(crate::native::Renderer::Glow, options);
		#[cfg(all(not(feature = "glow"), feature = "wgpu"))]
		return self.run_with_backend(crate::native::Renderer::Wgpu, options);
	}

	/// Exactly the same as [`App::run`](Self::run) but allows the caller to select a specific rendering backend.
	#[tracing::instrument(skip(self, options))]
	pub fn run_with_backend(self, renderer: crate::native::Renderer, options: ViewportBuilder) -> Result<(), Error> {
		let event_loop = EventLoop::<crate::native::UserEvent>::with_user_event()
			.build()
			.map_err(Error::EventLoopBuildFail)?;
		self.run_with_event_loop(event_loop, renderer, options)
	}

	#[tracing::instrument(skip(self, event_loop, options))]
	fn run_with_event_loop(self, event_loop: EventLoop<crate::native::UserEvent>, renderer: crate::native::Renderer, options: ViewportBuilder) -> Result<(), Error> {
		let proxy = event_loop.create_proxy();
		self.root.handle.init(proxy.clone());

		let mut runner: Box<dyn crate::native::Runner> = match renderer {
			#[cfg(feature = "glow")]
			crate::native::Renderer::Glow => {
				tracing::info!("using glow renderer");
				Box::new(crate::native::_glow::GlowRunner::new(self.root, self.error_rx, options, proxy.clone()))
			}
			#[cfg(feature = "wgpu")]
			crate::native::Renderer::Wgpu => {
				tracing::info!("using wgpu renderer");
				Box::new(crate::native::_wgpu::WgpuRunner::new(self.root, self.error_rx, options, proxy.clone()))
			}
			#[cfg(not(feature = "glow"))]
			crate::native::Renderer::Glow => return Err(Error::RendererUnavailable("glow")),
			#[cfg(not(feature = "wgpu"))]
			crate::native::Renderer::Wgpu => return Err(Error::RendererUnavailable("wgpu")),
		};

		#[cfg(all(feature = "ctrlc", not(target_os = "android")))]
		if self.ctrlc_handler {
			ctrlc::set_handler(move || {
				println!();
				if let Err(e) = proxy
					.send_event(crate::native::UserEvent::Exit)
					.map_err(|_| Error::SendingOverChannel)
				{
					tracing::error!("{e}");
				}
			})
			.map_err(Error::SetSigHandler)?;
		}

		event_loop
			.run_app(&mut runner)
			.map_err(Error::EventLoopFail)
	}
}

#[doc(hidden)]
pub fn run_test<F: Fn(&mut egui::Ui)>(func: F) -> Result<(), Error> {
	let ctx = egui::Context::default();

	let output = ctx.run_ui(egui::RawInput::default(), |ui| {
		func(ui);
	});

	assert!(!output.textures_delta.set.is_empty());
	assert!(!output.shapes.is_empty());

	Ok(())
}
