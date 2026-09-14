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
use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// A reusable modal dialog containing application and license information.
pub mod about_dialog;

pub(crate) mod error_dialog;

/// In-app floating toast notifications.
pub mod toast;

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
/// output to its parent, and start cancellable asynchronous work. Clones refer
/// to the same underlying channels and cancellation scope.
pub struct Context<W: Widget> {
	input: Sender<W::Message>,
	error: Sender<W::Error>,
	output: Option<Sender<W::Output>>,
	handle: Handle,
	cancellation: CancellationToken,
}

impl<W: Widget> Clone for Context<W> {
	fn clone(&self) -> Self {
		Self {
			error: self.error.clone(),
			input: self.input.clone(),
			output: self.output.clone(),
			handle: self.handle.clone(),
			cancellation: self.cancellation.clone(),
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
	/// Spawns a cancellable asynchronous task with a clone of this context.
	///
	/// When the future completes, its successful value is enqueued as a widget
	/// message and its error is routed through this context. The future is
	/// dropped without producing either value when the managed widget shuts
	/// down.
	///
	/// # Panics
	///
	/// Panics if called outside a Tokio runtime.
	pub fn spawn<F, Fut>(&self, f: F)
	where
		F: FnOnce(Context<W>) -> Fut + Send + 'static,
		Fut: Future<Output = Result<W::Message, W::Error>> + Send + 'static,
	{
		let ctx = self.clone();
		let cancellation = self.cancellation.child_token();

		tokio::spawn(async move {
			let Some(out) = cancellation.run_until_cancelled(f(ctx.clone())).await else {
				return;
			};

			match out {
				Ok(msg) => ctx.emit(msg),
				Err(err) => ctx.error(err),
			}
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
			tracing::warn!(widget = std::any::type_name::<W>(), "discarding widget output because no output channel is configured");
		}
	}

	/// Routes an error toward the root widget's error handler.
	pub fn error(&self, err: W::Error) {
		self.error.emit(err);
	}

	/// Enqueues an in-app floating toast notification and requests a window repaint.
	///
	/// The toast will be displayed in a non-intrusive floating overlay above the
	/// application interface and automatically dismisses after its configured duration
	/// unless made sticky.
	///
	/// Any interactive action callbacks configured on the [`Toast`] will receive a clone
	/// of this [`Context`] when clicked.
	///
	/// # Examples
	///
	/// ```ignore
	/// use std::time::Duration;
	/// use egelm::window::toast::Toast;
	///
	/// ctx.toast(
	///     Toast::new("Settings updated")
	///         .success()
	///         .body("Your preferences have been saved.")
	///         .duration(Duration::from_secs(4)),
	/// );
	/// ```
	///
	/// Toasts can also include interactive action callbacks:
	///
	/// ```ignore
	/// ctx.toast(
	///     Toast::new("Failed to connect to server")
	///         .error()
	///         .body("Check your internet connection.")
	///         .action("Retry", |ctx| ctx.emit(Message::RetryConnect)),
	/// );
	/// ```
	pub fn toast(&self, toast: toast::Toast<W>) {
		toast::add_toast(toast.into_active(self));
		self.handle.request_repaint();
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
/// [`tick`](Self::tick). [`init`](Self::init) and
/// [`shutdown`](Self::shutdown) bracket its managed lifetime. Deriving
/// [`egelm_macros::Widget`] propagates ticking, initialization, and shutdown to
/// named [`Managed`] fields.
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
pub trait Widget: AutoLifecycle + std::fmt::Debug + Sized {
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

	/// Provides an optional widget-initialization hook.
	///
	/// [`Managed::init`] invokes this at most once per managed lifetime, before
	/// initializing managed children. The application initializes its root
	/// widget before entering the event loop. The default implementation does
	/// nothing.
	#[expect(unused_variables)]
	fn init(&mut self, ctx: &Context<Self>) {}

	/// Provides an optional widget-shutdown hook.
	///
	/// [`Managed::shutdown`] invokes this at most once after shutting down
	/// managed children. Shutdown also cancels tasks spawned from the widget's
	/// context. Dropping an initialized [`Managed`] widget invokes shutdown
	/// automatically. The default implementation does nothing.
	#[expect(unused_variables)]
	fn shutdown(&mut self, ctx: &Context<Self>) {}
}

/// A rendering-only widget with no messages, output, or errors.
///
/// Implementing this trait automatically implements [`Widget`] and
/// [`AutoLifecycle`] with unit associated types and no lifecycle hooks.
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

impl<T: LeafWidget> AutoLifecycle for T {}
impl<T: LeafWidget> Widget for T {
	type Message = ();
	type Output = ();
	type Error = ();

	#[inline(always)]
	fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, _ctx: &Context<Self>) {
		self.render(ui, frame);
	}
}

/// Extra behavior for the root widget owned by an [`App`].
///
/// Implement this trait to customize window closing, error presentation, and
/// one-time rendering setup.
pub trait RootWidget: Widget + 'static {
	/// Handles a native window close request, such as clicking the title-bar
	/// close button. You can use this to close the window, but keep the app
	/// running through a tray icon, for example.
	///
	/// This callback is only invoked for a native close request. Calling
	/// [`Handle::exit`]—including as `frame.exit()` through [`Frame`]—exits the
	/// application directly and does not invoke this method.
	///
	/// The default implementation exits the application. Override it to hide
	/// the window, ask for confirmation, or ignore the request.
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

	/// Configures the shared `egui` context before the native surface is created.
	///
	/// This is called once when the application runner is constructed. The
	/// context and its configuration persist when a hidden or suspended window
	/// is recreated. The default implementation does nothing.
	#[expect(unused_variables)]
	fn setup(&mut self, ctx: &egui::Context) {}

	/// Set the RGBA background color for the native window. The default implementation returns a dark gray color.
	fn clear_color(&self) -> [u8; 4] {
		[27, 27, 27, 255]
	}
}

/// A widget together with its message queue and communication context.
///
/// `Managed<T>` dereferences to `T`, allowing direct access to the wrapped
/// widget. Its update methods drain queued messages before ticking the widget.
/// It also owns the widget's initialization state and asynchronous-task
/// cancellation scope.
#[derive(Debug)]
pub struct Managed<T: Widget + 'static> {
	widget: T,
	rx: crossbeam_channel::Receiver<T::Message>,
	ctx: Context<T>,
	pub(crate) handle: Handle,
	initialized: bool,
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
	/// application `handle`. The returned widget is not initialized until
	/// [`init`](Self::init) is called.
	pub fn new(output: impl Into<Option<Sender<T::Output>>>, error: Sender<T::Error>, handle: &Handle, widget: T) -> Self {
		let (tx, rx) = crossbeam_channel::unbounded();
		let wake = handle.clone();
		Self {
			widget,
			rx,
			ctx: Context {
				input: Sender::new(move |msg| {
					if let Err(e) = tx.send(msg).map_err(|_| Error::SendingOverChannel) {
						tracing::warn!(widget = std::any::type_name::<T>(), error = %e, "could not enqueue widget message");
					}
					wake.request_repaint();
				}),
				output: output.into(),
				error,
				cancellation: CancellationToken::new(),
				handle: handle.clone(),
			},
			handle: handle.clone(),
			initialized: false,
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
				tracing::error!(widget = std::any::type_name::<T>(), error = ?e, "widget update failed");
				self.ctx.error(e);
			}
		}

		self.widget.tick_children_auto();
		if let Err(e) = self.widget.tick(&self.ctx) {
			tracing::error!(widget = std::any::type_name::<T>(), error = ?e, "widget tick failed");
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

	/// Initializes this widget and then its managed children.
	///
	/// Calling this more than once without an intervening
	/// [`shutdown`](Self::shutdown) has no effect.
	pub fn init(&mut self) {
		if self.initialized {
			return;
		}

		self.widget.init(&self.ctx);
		self.widget.init_children_auto();
		self.initialized = true;
	}

	/// Shuts down this widget and cancels its asynchronous tasks.
	///
	/// Managed children shut down before this widget's [`Widget::shutdown`]
	/// hook runs. Calling this before initialization or more than once has no
	/// effect. Dropping an initialized managed widget calls this automatically.
	pub fn shutdown(&mut self) {
		if !self.initialized {
			return;
		}

		self.widget.shutdown_children_auto();
		self.widget.shutdown(&self.ctx);
		self.ctx.cancellation.cancel();
		self.initialized = false;
	}
}

impl<W: Widget + 'static> Drop for Managed<W> {
	fn drop(&mut self) {
		self.shutdown();
	}
}

/// Propagates lifecycle operations to widgets managed by a parent.
///
/// The [`egelm_macros::Widget`] derive macro implements this trait by calling
/// the corresponding update, initialization, or shutdown operation on every
/// named `Managed<T>` field.
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
pub trait AutoLifecycle {
	/// Processes messages and ticks all managed child widgets.
	///
	/// The default implementation does nothing.
	fn tick_children_auto(&mut self) {}

	/// Initializes all managed child widgets in declaration order.
	///
	/// The default implementation does nothing.
	fn init_children_auto(&mut self) {}

	/// Shuts down all managed child widgets in declaration order.
	///
	/// The default implementation does nothing.
	fn shutdown_children_auto(&mut self) {}
}

/// Parameters provided when instantiating a root widget through a factory.
pub struct FactoryContext<'a, T: Widget> {
	/// Communication context provided to the root widget.
	pub ctx: &'a Context<T>,
	/// Initialized native window handle.
	pub handle: &'a Handle,

	#[cfg(feature = "storage")]
	/// Access to persistent data stores for this application.
	pub storage: &'a Storage,
}

/// Owns a root widget and runs it in a native event loop.
///
/// Create an application with [`new`](Self::new), or use
/// [`new_factory`](Self::new_factory) when construction needs the root context
/// or window handle. Call [`run`](Self::run) once to open its window.
pub struct App<T: RootWidget> {
	root: Managed<T>,
	ctrlc_handler: bool,
	error_rx: crossbeam_channel::Receiver<T::Error>,
	options: ViewportBuilder,
}

impl<T: RootWidget> App<T> {
	/// Creates an application from an existing root widget.
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
		App::new_factory(ViewportBuilder::default(), move |_| root)
	}

	/// Creates an application with specific initial window attributes.
	pub fn new_with_options(options: ViewportBuilder, root: T) -> Self {
		App::new_factory(options, move |_| root)
	}

	/// Computes the root widget inside a provided closure that may fail.
	///
	/// The closure is passed a [`FactoryContext`] allowing the root widget
	/// to access window handles or storage options during initialization.
	pub fn try_new_factory<E, F>(options: ViewportBuilder, factory: F) -> Result<Self, E>
	where
		F: FnOnce(FactoryContext<'_, T>) -> Result<T, E>,
	{
		let (error_tx, error_rx) = crossbeam_channel::unbounded::<T::Error>();

		let handle = Handle::new();
		let (tx, rx) = crossbeam_channel::unbounded();
		let ctx = Context {
			input: Sender::new({
				let handle = handle.clone();
				move |msg| {
					if let Err(e) = tx.send(msg).map_err(|_| Error::SendingOverChannel) {
						tracing::warn!(widget = std::any::type_name::<T>(), error = %e, "could not enqueue root widget message");
					}
					handle.request_repaint();
				}
			}),
			output: None,
			error: Sender::new({
				let handle = handle.clone();
				move |err| {
					if let Err(e) = error_tx.send(err).map_err(|_| Error::SendingOverChannel) {
						tracing::warn!(widget = std::any::type_name::<T>(), error = %e, "could not enqueue root widget error");
					}
					handle.request_repaint();
				}
			}),
			cancellation: CancellationToken::new(),
			handle: handle.clone(),
		};

		#[cfg(feature = "storage")]
		let storage = if let Some(app_id) = &options.app_id {
			#[cfg(any(linux, windows))]
			'inner: {
				let cache = dirs::cache_dir()
					.unwrap_or_else(|| {
						tracing::warn!("cache directory not found; using system temp directory");
						std::env::temp_dir()
					})
					.join(app_id);

				if let Err(e) = std::fs::create_dir_all(&cache) {
					tracing::error!(error = %e, "failed to create cache directory; falling back to in-memory storage");
					break 'inner Storage::default();
				}

				match crate::storage::file::FileBackend::new(cache.join("storage.ron")) {
					Err(e) => {
						tracing::error!(error = %e, "failed to initialize filesystem storage backend; falling back to in-memory storage");
						Storage::default()
					}
					Ok(o) => Storage::new(o),
				}
			}

			#[cfg(android)]
			todo!("android storage backend not implemented yet");
		} else {
			tracing::warn!("no app_id provided; using in-memory storage backend");
			Storage::new(crate::storage::memory::MemoryBackend::default())
		};

		let factory_ctx = FactoryContext {
			ctx: &ctx,
			handle: &handle,

			#[cfg(feature = "storage")]
			storage: &storage,
		};

		Ok(Self {
			root: Managed {
				widget: factory(factory_ctx)?,
				rx,
				ctx,
				handle,
				initialized: false,
			},
			ctrlc_handler: true,
			error_rx,
			options,
		})
	}

	/// Creates an application with access to its context and window handle.
	///
	/// The factory can clone either argument into the root widget before it is
	/// placed in the application.
	pub fn new_factory<F>(options: ViewportBuilder, factory: F) -> Self
	where
		F: FnOnce(FactoryContext<'_, T>) -> T,
	{
		Self::try_new_factory(options, |ctx| Ok::<T, ()>(factory(ctx))).unwrap()
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
	#[cfg(android)]
	#[tracing::instrument(skip(self, options))]
	pub fn run_android(self, android_app: AndroidApp) -> Result<(), Error> {
		use egui_winit::winit::platform::android::EventLoopBuilderExtAndroid;

		let event_loop = EventLoop::<crate::native::UserEvent>::with_user_event()
			.with_android_app(android_app)
			.build()
			.map_err(Error::EventLoopBuildFail)?;

		self.run_with_event_loop(event_loop, crate::native::Renderer::Wgpu, self.options)
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
	/// App::new(Root).run()?;
	/// # Ok::<(), egelm::error::Error>(())
	/// ```
	#[tracing::instrument(skip(self))]
	pub fn run(self) -> Result<(), Error> {
		#[cfg(feature = "glow")]
		return self.run_with_backend(crate::native::Renderer::Glow);
		#[cfg(all(not(feature = "glow"), feature = "wgpu"))]
		return self.run_with_backend(crate::native::Renderer::Wgpu);

		#[cfg(all(not(wgpu), not(glow)))]
		Ok(())
	}

	/// Runs the native event loop with a specific rendering backend.
	///
	/// The corresponding `glow` or `wgpu` crate feature must be enabled.
	///
	/// # Errors
	///
	/// Returns [`Error::RendererUnavailable`] when the selected backend was not
	/// compiled in. Other errors are the same as [`App::run`](Self::run).
	#[tracing::instrument(skip(self, renderer))]
	pub fn run_with_backend(self, renderer: crate::native::Renderer) -> Result<(), Error> {
		let event_loop = EventLoop::<crate::native::UserEvent>::with_user_event()
			.build()
			.map_err(Error::EventLoopBuildFail)?;
		self.run_with_event_loop(event_loop, renderer)
	}

	#[tracing::instrument(skip(self, renderer, event_loop))]
	fn run_with_event_loop(mut self, event_loop: EventLoop<crate::native::UserEvent>, renderer: crate::native::Renderer) -> Result<(), Error> {
		let proxy = event_loop.create_proxy();
		self.root.handle.init(proxy.clone());

		tracing::info!(?renderer, "starting application event loop");
		let backend: Box<dyn crate::native::Backend> = match renderer {
			#[cfg(glow)]
			crate::native::Renderer::Glow => {
				tracing::debug!("initializing glow renderer");
				Box::new(crate::native::_glow::GlowBackend::default())
			}
			#[cfg(wgpu)]
			crate::native::Renderer::Wgpu => {
				tracing::debug!("initializing wgpu renderer");
				Box::new(crate::native::_wgpu::WgpuBackend::default())
			}
			#[cfg(not(glow))]
			crate::native::Renderer::Glow => return Err(Error::RendererUnavailable("glow")),
			#[cfg(not(wgpu))]
			crate::native::Renderer::Wgpu => return Err(Error::RendererUnavailable("wgpu")),
		};
		self.root.init();
		let mut runner = crate::native::Runner::new(self.root, self.error_rx, self.options, proxy.clone(), backend);

		#[cfg(ctrlc)]
		if self.ctrlc_handler {
			tracing::debug!("installing sigint handler");
			ctrlc::set_handler(move || {
				println!();
				if let Err(e) = proxy
					.send_event(crate::native::UserEvent::Exit)
					.map_err(|_| Error::SendingOverChannel)
				{
					tracing::debug!(error = %e, "could not request exit from sigint handler; event loop is likely closed");
				}
			})
			.map_err(Error::SetSigHandler)?;
		}

		event_loop
			.run_app(&mut runner)
			.map_err(Error::EventLoopFail)
	}
}

#[cfg(test)]
mod tests {
	use std::sync::{Arc, Mutex};

	use super::*;

	#[derive(Debug)]
	struct LifecycleChild {
		events: Arc<Mutex<Vec<&'static str>>>,
	}

	impl AutoLifecycle for LifecycleChild {}

	impl Widget for LifecycleChild {
		type Message = ();
		type Output = ();
		type Error = ();

		fn view(&mut self, _ui: &mut egui::Ui, _frame: &mut Frame, _ctx: &Context<Self>) {}

		fn init(&mut self, _ctx: &Context<Self>) {
			self.events.lock().unwrap().push("child init");
		}

		fn shutdown(&mut self, _ctx: &Context<Self>) {
			self.events.lock().unwrap().push("child shutdown");
		}
	}

	#[derive(Debug)]
	struct LifecycleParent {
		child: Managed<LifecycleChild>,
		events: Arc<Mutex<Vec<&'static str>>>,
	}

	impl AutoLifecycle for LifecycleParent {
		fn tick_children_auto(&mut self) {
			self.child.update_route_error();
		}

		fn init_children_auto(&mut self) {
			self.child.init();
		}

		fn shutdown_children_auto(&mut self) {
			self.child.shutdown();
		}
	}

	impl Widget for LifecycleParent {
		type Message = ();
		type Output = ();
		type Error = ();

		fn view(&mut self, _ui: &mut egui::Ui, _frame: &mut Frame, _ctx: &Context<Self>) {}

		fn init(&mut self, _ctx: &Context<Self>) {
			self.events.lock().unwrap().push("parent init");
		}

		fn shutdown(&mut self, _ctx: &Context<Self>) {
			self.events.lock().unwrap().push("parent shutdown");
		}
	}

	#[derive(Debug)]
	struct TestWidget {
		updates: Vec<u32>,
		ticks: usize,
		output_on_update: bool,
		fail_update: bool,
		fail_tick: bool,
	}

	impl AutoLifecycle for TestWidget {}

	impl Widget for TestWidget {
		type Message = u32;
		type Output = String;
		type Error = &'static str;

		fn view(&mut self, _ui: &mut egui::Ui, _frame: &mut Frame, _ctx: &Context<Self>) {}

		fn update(&mut self, msg: Self::Message, _handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
			self.updates.push(msg);
			if self.output_on_update {
				ctx.output(format!("updated {msg}"));
			}
			if self.fail_update {
				return Err("update failed");
			}
			Ok(())
		}

		fn tick(&mut self, _ctx: &Context<Self>) -> Result<(), Self::Error> {
			self.ticks += 1;
			if self.fail_tick {
				return Err("tick failed");
			}
			Ok(())
		}
	}

	fn sender<T: Send + 'static>() -> (Sender<T>, crossbeam_channel::Receiver<T>) {
		let (tx, rx) = crossbeam_channel::unbounded();
		(Sender::new(move |value| tx.send(value).unwrap()), rx)
	}

	fn managed(widget: TestWidget) -> (Managed<TestWidget>, crossbeam_channel::Receiver<String>, crossbeam_channel::Receiver<&'static str>) {
		let (output, output_rx) = sender();
		let (error, error_rx) = sender();
		(Managed::new(output, error, &Handle::default(), widget), output_rx, error_rx)
	}

	fn test_widget() -> TestWidget {
		TestWidget {
			updates: Vec::new(),
			ticks: 0,
			output_on_update: false,
			fail_update: false,
			fail_tick: false,
		}
	}

	fn lifecycle_managed(events: Arc<Mutex<Vec<&'static str>>>) -> Managed<LifecycleParent> {
		let handle = Handle::default();
		let (error, _error_rx) = sender();
		let child = Managed::new(None, error.clone(), &handle, LifecycleChild { events: events.clone() });
		Managed::new(None, error, &handle, LifecycleParent { child, events })
	}

	#[test]
	fn sender_map_transforms_and_forwards_values() {
		let values = Arc::new(Mutex::new(Vec::new()));
		let received = values.clone();
		let sender = Sender::new(move |value: usize| received.lock().unwrap().push(value));
		let mapped = sender.map(|value: &str| value.len());

		mapped.emit("egelm");

		assert_eq!(*values.lock().unwrap(), vec![5]);
	}

	#[test]
	fn managed_update_drains_messages_in_order_and_ticks_once() {
		let (mut managed, _output_rx, _error_rx) = managed(test_widget());
		let input = managed.ctx.input_sender();
		input.emit(3);
		input.emit(1);
		input.emit(4);

		managed.update().unwrap();

		assert_eq!(managed.updates, [3, 1, 4]);
		assert_eq!(managed.ticks, 1);
	}

	#[test]
	fn context_forwards_outputs_and_errors() {
		let mut widget = test_widget();
		widget.output_on_update = true;
		let (mut managed, output_rx, error_rx) = managed(widget);
		managed.ctx.input_sender().emit(7);
		managed.ctx.error("reported directly");

		managed.update().unwrap();

		assert_eq!(output_rx.try_recv(), Ok("updated 7".to_owned()));
		assert_eq!(error_rx.try_recv(), Ok("reported directly"));
	}

	#[test]
	fn update_route_error_routes_update_errors_and_continues_to_tick() {
		let mut widget = test_widget();
		widget.fail_update = true;
		let (mut managed, _output_rx, error_rx) = managed(widget);
		let input = managed.ctx.input_sender();
		input.emit(10);
		input.emit(20);

		managed.update_route_error();

		assert_eq!(managed.updates, [10, 20]);
		assert_eq!(managed.ticks, 1);
		assert_eq!(error_rx.try_iter().collect::<Vec<_>>(), ["update failed", "update failed"]);
	}

	#[test]
	fn update_route_error_routes_tick_errors() {
		let mut widget = test_widget();
		widget.fail_tick = true;
		let (mut managed, _output_rx, error_rx) = managed(widget);

		managed.update_route_error();

		assert_eq!(managed.ticks, 1);
		assert_eq!(error_rx.try_recv(), Ok("tick failed"));
	}

	#[test]
	fn managed_lifecycle_is_ordered_and_idempotent() {
		let events = Arc::new(Mutex::new(Vec::new()));
		let mut managed = lifecycle_managed(events.clone());

		managed.init();
		managed.init();
		assert_eq!(*events.lock().unwrap(), ["parent init", "child init"]);

		managed.shutdown();
		managed.shutdown();
		assert_eq!(*events.lock().unwrap(), ["parent init", "child init", "child shutdown", "parent shutdown"]);
	}

	#[test]
	fn dropping_initialized_managed_widget_runs_shutdown() {
		let events = Arc::new(Mutex::new(Vec::new()));

		{
			let mut managed = lifecycle_managed(events.clone());
			managed.init();
		}

		assert_eq!(*events.lock().unwrap(), ["parent init", "child init", "child shutdown", "parent shutdown"]);
	}

	#[tokio::test]
	async fn shutdown_cancels_spawned_tasks() {
		struct NotifyOnDrop(Option<tokio::sync::oneshot::Sender<()>>);

		impl Drop for NotifyOnDrop {
			fn drop(&mut self) {
				if let Some(tx) = self.0.take() {
					_ = tx.send(());
				}
			}
		}

		let (mut managed, _output_rx, _error_rx) = managed(test_widget());
		managed.init();

		let (started_tx, started_rx) = tokio::sync::oneshot::channel();
		let (cancelled_tx, cancelled_rx) = tokio::sync::oneshot::channel();
		managed.ctx.spawn(move |_ctx| async move {
			let _notify = NotifyOnDrop(Some(cancelled_tx));
			_ = started_tx.send(());
			std::future::pending::<Result<u32, &'static str>>().await
		});

		started_rx.await.unwrap();
		managed.shutdown();

		tokio::time::timeout(std::time::Duration::from_secs(1), cancelled_rx)
			.await
			.expect("task was not cancelled")
			.expect("task cancellation signal was dropped");
	}
}
