//! Procedural macros used by `egelm`.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type, parse_macro_input};

/// Implements [`egelm::window::TickChildren`] for a widget struct.
///
/// The generated [`TickChildren::tick_children_auto`] implementation calls
/// [`Managed::update_route_error`] on every named field whose outer type is
/// `Managed<T>`. This lets the `egelm` runtime automatically process messages
/// and tick managed child widgets whenever their parent is updated.
///
/// Tuple and unit structs receive an empty implementation. Applying this derive
/// to an enum or union produces a compile error.
///
/// [`Managed::update_route_error`]: egelm::window::Managed::update_route_error
/// [`TickChildren::tick_children_auto`]: egelm::window::TickChildren::tick_children_auto
///
/// # Examples
///
/// ```
/// use egelm::prelude::*;
///
/// #[derive(Debug, Widget)]
/// struct Child;
///
/// impl Widget for Child {
///     type Message = ();
///     type Output = ();
///     type Error = ();
///
///     fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, _ctx: &Context<Self>) {
///         ui.label("Managed child");
///     }
///
///     // This will get called automatically whenever the parent widget is ticked and there is an available message,
///     // so we don't need to do anything here.
///     fn update(&mut self, _msg: Self::Message, _handle: &Handle, _ctx: &Context<Self>) -> Result<(), Self::Error> {
///         Ok(())
///     }
/// }
///
/// #[derive(Debug, Widget)]
/// struct Parent {
///     // The derive will automatically update this child widget whenever the parent is ticked.
///     child: Managed<Child>,
/// }
///
/// impl Widget for Parent {
///     type Message = ();
///     type Output = ();
///     type Error = ();
///
///     fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, _ctx: &Context<Self>) {
///         self.child.render(ui, frame);
///     }
/// }
///
/// impl RootWidget for Parent {}
///
/// let app = App::new_factory(|ctx, handle| Parent {
///     child: Managed::new(None, ctx.error_sender(), handle, Child),
/// });
/// ```
#[proc_macro_derive(Widget)]
pub fn derive_children(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = &input.ident;

	let Data::Struct(data) = &input.data else {
		return syn::Error::new_spanned(&input, "`Widget` can only be derived on structs")
			.to_compile_error()
			.into();
	};

	let Fields::Named(fields) = &data.fields else {
		// return syn::Error::new_spanned(&input, "`Widget` requires named fields")
		// 	.to_compile_error()
		// 	.into();
		return quote! {
			impl egelm::window::TickChildren for #name {
				fn tick_children_auto(&mut self) {}
			}
		}
		.into();
	};

	let ticks = fields.named.iter().filter_map(|field| {
		if !is_managed(&field.ty) {
			return None;
		}
		let field_name = field.ident.as_ref()?;
		Some(quote! {
			self.#field_name.update_route_error();
		})
	});

	let expanded = quote! {
		impl egelm::window::TickChildren for #name {
			fn tick_children_auto(&mut self) {
				#(#ticks)*
			}
		}
	};

	expanded.into()
}

fn is_managed(ty: &Type) -> bool {
	let Type::Path(type_path) = ty else {
		return false;
	};
	let Some(segment) = type_path.path.segments.last() else {
		return false;
	};
	if segment.ident != "Managed" {
		return false;
	}

	matches!(&segment.arguments, PathArguments::AngleBracketed(args) if args.args.len() == 1 && matches!(args.args.first(), Some(GenericArgument::Type(_))))
}
