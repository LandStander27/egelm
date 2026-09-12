//! Procedural macros used by `egelm`.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type, parse_macro_input};

/// Implements [`egelm::window::AutoLifecycle`] for a widget struct.
///
/// For every named field whose type is [`Managed<T>`], or a container/wrapper
/// containing managed widgets (such as [`Option`], [`Vec`], [`HashMap`], [`BTreeMap`],
/// slices, fixed arrays, `&mut` references, or `Box`), the generated implementation
/// processes messages and ticks the child during [`AutoLifecycle::tick_children_auto`],
/// initializes it during [`AutoLifecycle::init_children_auto`], and shuts it down
/// during [`AutoLifecycle::shutdown_children_auto`].
///
/// Shared references (such as `&Managed<T>`, `&Vec<Managed<T>>`, or `&[Managed<T>]`)
/// are safely ignored since immutable values cannot be updated or mutated.
///
/// Tuple and unit structs receive an empty implementation. Applying this derive
/// to an enum or union produces a compile error.
///
/// [`AutoLifecycle::init_children_auto`]: egelm::window::AutoLifecycle::init_children_auto
/// [`AutoLifecycle::shutdown_children_auto`]: egelm::window::AutoLifecycle::shutdown_children_auto
/// [`AutoLifecycle::tick_children_auto`]: egelm::window::AutoLifecycle::tick_children_auto
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
/// }
///
/// #[derive(Debug, Widget)]
/// struct Parent {
///     // The derive automatically updates all managed children and collections.
///     single: Managed<Child>,
///     optional: Option<Managed<Child>>,
///     list: Vec<Managed<Child>>,
/// }
/// ```
#[proc_macro_derive(Widget)]
pub fn derive_children(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	let Data::Struct(data) = &input.data else {
		return syn::Error::new_spanned(&input, "`Widget` can only be derived on structs")
			.to_compile_error()
			.into();
	};

	let Fields::Named(fields) = &data.fields else {
		return quote! {
			impl #impl_generics egelm::window::AutoLifecycle for #name #ty_generics #where_clause {}
		}
		.into();
	};

	let mut tick_stmts = Vec::new();
	let mut init_stmts = Vec::new();
	let mut shutdown_stmts = Vec::new();

	for field in &fields.named {
		let Some(wrapper) = inspect_type(&field.ty) else {
			continue;
		};
		let Some(field_name) = &field.ident else {
			continue;
		};

		if let Some(stmt) = generate_action_code(&wrapper, quote!(self.#field_name), Action::Tick, 0) {
			tick_stmts.push(stmt);
		}
		if let Some(stmt) = generate_action_code(&wrapper, quote!(self.#field_name), Action::Init, 0) {
			init_stmts.push(stmt);
		}
		if let Some(stmt) = generate_action_code(&wrapper, quote!(self.#field_name), Action::Shutdown, 0) {
			shutdown_stmts.push(stmt);
		}
	}

	let expanded = quote! {
		impl #impl_generics egelm::window::AutoLifecycle for #name #ty_generics #where_clause {
			fn tick_children_auto(&mut self) {
				#(#tick_stmts)*
			}

			fn init_children_auto(&mut self) {
				#(#init_stmts)*
			}

			fn shutdown_children_auto(&mut self) {
				#(#shutdown_stmts)*
			}
		}
	};

	expanded.into()
}

#[derive(Debug, PartialEq, Eq)]
enum LifecycleWrapper {
	/// Directly a `Managed<T>`
	Direct,
	/// `Option<T>`
	Option(Box<LifecycleWrapper>),
	/// `Vec<T>`, `VecDeque<T>`, `LinkedList<T>`, `[T; N]`, `[T]`
	Sequence(Box<LifecycleWrapper>),
	/// `HashMap<K, V>`, `BTreeMap<K, V>`
	Map(Box<LifecycleWrapper>),
	/// `&mut T`, `Box<T>`
	DerefMut(Box<LifecycleWrapper>),
	/// `&T` (immutable reference - mutation is impossible, so it is a no-op)
	ImmutableRef,
}

fn inspect_type(ty: &Type) -> Option<LifecycleWrapper> {
	match ty {
		Type::Path(type_path) => {
			let segment = type_path.path.segments.last()?;
			let ident = segment.ident.to_string();

			match ident.as_str() {
				"Managed" => {
					if let PathArguments::AngleBracketed(args) = &segment.arguments
						&& args
							.args
							.iter()
							.any(|arg| matches!(arg, GenericArgument::Type(_)))
					{
						return Some(LifecycleWrapper::Direct);
					}
					None
				}
				"Option" => {
					let inner = extract_generic_type(&segment.arguments, 0)?;
					let inner_wrapper = inspect_type(inner)?;
					Some(LifecycleWrapper::Option(Box::new(inner_wrapper)))
				}
				"Vec" | "VecDeque" | "LinkedList" => {
					let inner = extract_generic_type(&segment.arguments, 0)?;
					let inner_wrapper = inspect_type(inner)?;
					Some(LifecycleWrapper::Sequence(Box::new(inner_wrapper)))
				}
				"HashMap" | "BTreeMap" => {
					let inner = extract_generic_type(&segment.arguments, 1).or_else(|| extract_first_matching_generic_type(&segment.arguments))?;
					let inner_wrapper = inspect_type(inner)?;
					Some(LifecycleWrapper::Map(Box::new(inner_wrapper)))
				}
				"Box" => {
					let inner = extract_generic_type(&segment.arguments, 0)?;
					let inner_wrapper = inspect_type(inner)?;
					Some(LifecycleWrapper::DerefMut(Box::new(inner_wrapper)))
				}
				_ => None,
			}
		}
		Type::Reference(type_ref) => {
			let inner_wrapper = inspect_type(&type_ref.elem)?;
			if type_ref.mutability.is_some() {
				Some(LifecycleWrapper::DerefMut(Box::new(inner_wrapper)))
			} else {
				Some(LifecycleWrapper::ImmutableRef)
			}
		}
		Type::Slice(type_slice) => {
			let inner_wrapper = inspect_type(&type_slice.elem)?;
			Some(LifecycleWrapper::Sequence(Box::new(inner_wrapper)))
		}
		Type::Array(type_array) => {
			let inner_wrapper = inspect_type(&type_array.elem)?;
			Some(LifecycleWrapper::Sequence(Box::new(inner_wrapper)))
		}
		Type::Paren(type_paren) => inspect_type(&type_paren.elem),
		Type::Group(type_group) => inspect_type(&type_group.elem),
		_ => None,
	}
}

fn extract_generic_type(args: &PathArguments, index: usize) -> Option<&Type> {
	let PathArguments::AngleBracketed(bracketed) = args else {
		return None;
	};
	let type_args: Vec<&Type> = bracketed
		.args
		.iter()
		.filter_map(|arg| match arg {
			GenericArgument::Type(ty) => Some(ty),
			_ => None,
		})
		.collect();
	type_args.get(index).copied()
}

fn extract_first_matching_generic_type(args: &PathArguments) -> Option<&Type> {
	let PathArguments::AngleBracketed(bracketed) = args else {
		return None;
	};
	for arg in &bracketed.args {
		if let GenericArgument::Type(ty) = arg
			&& inspect_type(ty).is_some()
		{
			return Some(ty);
		}
	}
	None
}

#[derive(Clone, Copy)]
enum Action {
	Tick,
	Init,
	Shutdown,
}

impl Action {
	fn method_ident(self) -> proc_macro2::Ident {
		match self {
			Action::Tick => quote::format_ident!("update_route_error"),
			Action::Init => quote::format_ident!("init"),
			Action::Shutdown => quote::format_ident!("shutdown"),
		}
	}
}

fn generate_action_code(wrapper: &LifecycleWrapper, expr: proc_macro2::TokenStream, action: Action, depth: usize) -> Option<proc_macro2::TokenStream> {
	match wrapper {
		LifecycleWrapper::Direct => {
			let method = action.method_ident();
			Some(quote! {
				#expr.#method();
			})
		}
		LifecycleWrapper::Option(inner) => {
			let var = quote::format_ident!("_child_{depth}");
			let inner_code = generate_action_code(inner, quote!(#var), action, depth + 1)?;
			Some(quote! {
				if let Some(#var) = (#expr).as_mut() {
					#inner_code
				}
			})
		}
		LifecycleWrapper::Sequence(inner) => {
			let var = quote::format_ident!("_child_{depth}");
			let inner_code = generate_action_code(inner, quote!(#var), action, depth + 1)?;
			Some(quote! {
				for #var in (#expr).iter_mut() {
					#inner_code
				}
			})
		}
		LifecycleWrapper::Map(inner) => {
			let var = quote::format_ident!("_child_{depth}");
			let inner_code = generate_action_code(inner, quote!(#var), action, depth + 1)?;
			Some(quote! {
				for #var in (#expr).values_mut() {
					#inner_code
				}
			})
		}
		LifecycleWrapper::DerefMut(inner) => generate_action_code(inner, quote!((#expr)), action, depth),
		LifecycleWrapper::ImmutableRef => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse_type(source: &str) -> Type {
		syn::parse_str(source).unwrap()
	}

	#[test]
	fn recognizes_managed_widget_types() {
		assert_eq!(inspect_type(&parse_type("Managed<Child>")), Some(LifecycleWrapper::Direct));
		assert_eq!(inspect_type(&parse_type("egelm::window::Managed<Child>")), Some(LifecycleWrapper::Direct));
	}

	#[test]
	fn recognizes_option_managed_types() {
		assert_eq!(
			inspect_type(&parse_type("Option<Managed<Child>>")),
			Some(LifecycleWrapper::Option(Box::new(LifecycleWrapper::Direct)))
		);
		assert_eq!(
			inspect_type(&parse_type("std::option::Option<egelm::window::Managed<Child>>")),
			Some(LifecycleWrapper::Option(Box::new(LifecycleWrapper::Direct)))
		);
	}

	#[test]
	fn recognizes_vec_and_collection_types() {
		assert_eq!(
			inspect_type(&parse_type("Vec<Managed<Child>>")),
			Some(LifecycleWrapper::Sequence(Box::new(LifecycleWrapper::Direct)))
		);
		assert_eq!(
			inspect_type(&parse_type("std::collections::HashMap<String, Managed<Child>>")),
			Some(LifecycleWrapper::Map(Box::new(LifecycleWrapper::Direct)))
		);
		assert_eq!(
			inspect_type(&parse_type("BTreeMap<u32, Managed<Child>>")),
			Some(LifecycleWrapper::Map(Box::new(LifecycleWrapper::Direct)))
		);
		assert_eq!(
			inspect_type(&parse_type("[Managed<Child>; 4]")),
			Some(LifecycleWrapper::Sequence(Box::new(LifecycleWrapper::Direct)))
		);
	}

	#[test]
	fn recognizes_reference_types() {
		assert_eq!(
			inspect_type(&parse_type("&'a mut Managed<Child>")),
			Some(LifecycleWrapper::DerefMut(Box::new(LifecycleWrapper::Direct)))
		);
		assert_eq!(inspect_type(&parse_type("&'a Managed<Child>")), Some(LifecycleWrapper::ImmutableRef));
		assert_eq!(
			inspect_type(&parse_type("&'a mut [Managed<Child>]")),
			Some(LifecycleWrapper::DerefMut(Box::new(LifecycleWrapper::Sequence(Box::new(LifecycleWrapper::Direct)))))
		);
		assert_eq!(inspect_type(&parse_type("&'a [Managed<Child>]")), Some(LifecycleWrapper::ImmutableRef));
		assert_eq!(inspect_type(&parse_type("&'a Option<Managed<Child>>")), Some(LifecycleWrapper::ImmutableRef));
		assert_eq!(
			inspect_type(&parse_type("&'a mut Option<Managed<Child>>")),
			Some(LifecycleWrapper::DerefMut(Box::new(LifecycleWrapper::Option(Box::new(LifecycleWrapper::Direct)))))
		);
	}

	#[test]
	fn recognizes_nested_combinations() {
		assert_eq!(
			inspect_type(&parse_type("Option<Vec<Managed<Child>>>")),
			Some(LifecycleWrapper::Option(Box::new(LifecycleWrapper::Sequence(Box::new(LifecycleWrapper::Direct)))))
		);
		assert_eq!(
			inspect_type(&parse_type("HashMap<String, Option<Managed<Child>>>")),
			Some(LifecycleWrapper::Map(Box::new(LifecycleWrapper::Option(Box::new(LifecycleWrapper::Direct)))))
		);
		assert_eq!(
			inspect_type(&parse_type("Box<Managed<Child>>")),
			Some(LifecycleWrapper::DerefMut(Box::new(LifecycleWrapper::Direct)))
		);
	}

	#[test]
	fn rejects_non_managed_and_malformed_types() {
		assert_eq!(inspect_type(&parse_type("Child")), None);
		assert_eq!(inspect_type(&parse_type("String")), None);
		assert_eq!(inspect_type(&parse_type("Option<String>")), None);
		assert_eq!(inspect_type(&parse_type("Vec<u32>")), None);
		assert_eq!(inspect_type(&parse_type("HashMap<String, i32>")), None);
		assert_eq!(inspect_type(&parse_type("Managed<'static>")), None);
		assert_eq!(inspect_type(&parse_type("&str")), None);
	}
}
