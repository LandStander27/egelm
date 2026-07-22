use std::any::Any;
use std::sync::{Arc, Mutex, OnceLock};

use egui_winit::winit::event_loop::EventLoop;

use crate::prelude::*;

pub mod about_dialog;
pub(crate) mod error_dialog;

pub(crate) static ERROR_TX: OnceLock<crossbeam_channel::Sender<Box<dyn AnyDebug + Send + Sync>>> = OnceLock::new();
pub(crate) static ERROR_RX: OnceLock<Mutex<crossbeam_channel::Receiver<Box<dyn AnyDebug + Send + Sync>>>> = OnceLock::new();

pub(crate) trait AnyDebug: std::fmt::Debug + Any {
	fn as_any(&self) -> &dyn Any;
}

impl<T: std::fmt::Debug + Any> AnyDebug for T {
	fn as_any(&self) -> &dyn Any {
		self
	}
}

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

	pub fn emit(&self, msg: M) {
		(self.f)(msg)
	}

	pub fn map<N: 'static>(&self, f: impl Fn(N) -> M + Send + Sync + 'static) -> Sender<N> {
		let inner = self.clone();
		Sender::new(move |n| inner.emit(f(n)))
	}
}

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

	pub fn emit(&self, msg: W::Message) {
		self.input.emit(msg);
	}

	pub fn output(&self, msg: W::Output) {
		if let Some(output) = &self.output {
			output.emit(msg);
		} else {
			tracing::warn!("`output` channel does not exist");
		}
	}

	pub fn error(&self, err: W::Error) {
		self.error.emit(err);
	}

	pub fn error_sender(&self) -> Sender<W::Error> {
		self.error.clone()
	}

	pub fn input_sender(&self) -> Sender<W::Message> {
		self.input.clone()
	}
}

pub trait Widget: TickChildren + std::fmt::Debug + Sized {
	type Message: Send + std::fmt::Debug + 'static;
	type Output: Send + 'static;
	type Error: std::fmt::Debug + Send + Sync + 'static;

	fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, ctx: &Context<Self>);

	#[expect(unused_variables)]
	fn update(&mut self, msg: Self::Message, handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
		Ok(())
	}

	#[expect(unused_variables)]
	fn tick(&mut self, ctx: &Context<Self>) -> Result<(), Self::Error> {
		Ok(())
	}

	#[expect(unused_variables)]
	fn init(&mut self, ctx: &Context<Self>) {}
}

pub trait LeafWidget: std::fmt::Debug {
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

pub trait RootWidget: Widget + 'static {
	fn close(&mut self, frame: &mut Frame) {
		frame.exit();
	}

	#[expect(unused_variables)]
	fn error(&mut self, err: &Self::Error) -> (String, Option<String>) {
		(
			"Unknown error".to_string(),
			Some("Error translating is not implemented.\nImplement RootWidget::error in order to view the actual error that happened.".to_string()),
		)
	}

	#[expect(unused_variables)]
	fn setup(&mut self, ctx: &egui::Context) {}
}

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

	pub fn render(&mut self, ui: &mut egui::Ui, frame: &mut Frame) {
		self.widget.view(ui, frame, &self.ctx);
	}

	pub fn update_route_error(&mut self) {
		while let Ok(msg) = self.rx.try_recv() {
			if let Err(e) = self.widget.update(msg, &self.handle, &self.ctx) {
				self.ctx.error(e);
			}
		}

		if let Err(e) = self.widget.tick(&self.ctx) {
			self.ctx.error(e);
		}
	}

	pub fn update(&mut self) -> Result<(), T::Error> {
		while let Ok(msg) = self.rx.try_recv() {
			self.widget.update(msg, &self.handle, &self.ctx)?;
		}

		self.widget.tick_children_auto();
		self.widget.tick(&self.ctx)
	}
}

pub trait TickChildren {
	fn tick_children_auto(&mut self) {}
}
// impl<T> TickChildren for T {}

pub struct App<T: RootWidget> {
	root: Managed<T>,
	ctrlc_handler: bool,
}

impl<T: RootWidget> App<T> {
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

	pub fn new_factory<F: FnOnce(&Context<T>, &Handle) -> T>(factory: F) -> Self {
		let (tx, rx) = crossbeam_channel::unbounded();
		crate::window::ERROR_RX
			.set(std::sync::Mutex::new(rx))
			.expect("cannot call App::new twice");
		crate::window::ERROR_TX
			.set(tx)
			.expect("cannot call App::new twice");

		let handle = Handle::new();
		let wake = handle.clone();
		let (tx, rx) = crossbeam_channel::unbounded();
		let ctx = Context {
			input: Sender::new(move |msg| {
				if let Err(e) = tx.send(msg).map_err(|_| Error::SendingOverChannel) {
					tracing::error!("{e}");
				}
				wake.request_repaint();
			}),
			output: None,
			error: Sender::new(move |err| {
				if let Err(e) = super::window::ERROR_TX
					.get()
					.expect("ERROR_TX not inited; was App::new called?")
					.send(Box::new(err) as Box<dyn crate::window::AnyDebug + Send + Sync + 'static>)
					.map_err(|_| Error::SendingOverChannel)
				{
					tracing::error!("{e}");
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
		}
	}

	pub fn without_ctrl_handler(mut self) -> Self {
		self.ctrlc_handler = false;
		self
	}

	#[tracing::instrument(skip(self, options))]
	pub fn run(self, options: ViewportBuilder) -> Result<(), Error> {
		let event_loop = EventLoop::<crate::native::glow::UserEvent>::with_user_event()
			.build()
			.map_err(Error::EventLoopBuildFail)?;
		let proxy = event_loop.create_proxy();
		self.root.handle.init(proxy.clone());
		let mut runner = crate::native::glow::Runner::new(self.root, options, proxy.clone());

		#[cfg(feature = "ctrlc")]
		if self.ctrlc_handler {
			ctrlc::set_handler(move || {
				println!();
				if let Err(e) = proxy
					.send_event(crate::native::glow::UserEvent::Exit)
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
