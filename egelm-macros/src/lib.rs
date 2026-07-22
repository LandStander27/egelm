use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type, parse_macro_input};

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
			impl TickChildren for #name {
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
		impl TickChildren for #name {
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
