//! Procedural macros used by `egelm`.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type, parse_macro_input};

/// Implements [`egelm::window::AutoLifecycle`] for a widget struct.
///
/// For every named field whose outer type is `Managed<T>`, the generated
/// implementation processes messages and ticks the child during
/// [`AutoLifecycle::tick_children_auto`], initializes it during
/// [`AutoLifecycle::init_children_auto`], and shuts it down during
/// [`AutoLifecycle::shutdown_children_auto`].
///
/// Tuple and unit structs receive an empty implementation. Applying this derive
/// to an enum or union produces a compile error.
///
/// [`AutoLifecycle::init_children_auto`]: egelm::window::AutoLifecycle::init_children_auto
/// [`AutoLifecycle::shutdown_children_auto`]: egelm::window::AutoLifecycle::shutdown_children_auto
/// [`AutoLifecycle::tick_children_auto`]: egelm::window::AutoLifecycle::tick_children_auto
/// [`Managed::update_route_error`]: egelm::window::Managed::update_route_error
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
			impl egelm::window::AutoLifecycle for #name {}
		}
		.into();
	};

	let field_names: Vec<proc_macro2::TokenStream> = fields
		.named
		.iter()
		.filter_map(|field| {
			if !is_managed(&field.ty) {
				return None;
			}
			let field_name = field.ident.as_ref()?;
			Some(quote! {
				#field_name
			})
		})
		.collect();

	let expanded = quote! {
		impl egelm::window::AutoLifecycle for #name {
			fn tick_children_auto(&mut self) {
				#(self.#field_names.update_route_error();)*
			}

			fn init_children_auto(&mut self) {
				#(self.#field_names.init();)*
			}

			fn shutdown_children_auto(&mut self) {
				#(self.#field_names.shutdown();)*
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

#[cfg(test)]
mod tests {
	use super::*;

	fn parse_type(source: &str) -> Type {
		syn::parse_str(source).unwrap()
	}

	#[test]
	fn recognizes_managed_widget_types() {
		assert!(is_managed(&parse_type("Managed<Child>")));
		assert!(is_managed(&parse_type("egelm::window::Managed<Child>")));
	}

	#[test]
	fn rejects_non_managed_and_malformed_types() {
		assert!(!is_managed(&parse_type("Child")));
		assert!(!is_managed(&parse_type("Option<Managed<Child>>")));
		assert!(!is_managed(&parse_type("Managed<'static>")));
		assert!(!is_managed(&parse_type("&Managed<Child>")));
	}
}
