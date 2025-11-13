use proc_macro::{Ident, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use stringcase::snake_case;
use syn::{Data, DeriveInput, Fields, FieldsNamed, FieldsUnnamed, Index, parse_macro_input};

#[derive(Debug, Clone)]
struct HasComponentArms {
    get: TokenStream2,
    get_mut: TokenStream2,
}

#[proc_macro_derive(HasComponent)]
pub fn derive_has_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let arms = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => named_struct(fields_named.clone()),
            Fields::Unnamed(fields_unnamed) => unnamed_struct(fields_unnamed.clone()),
            Fields::Unit => vec![],
        },
        _ => vec![],
    };
    /////////////////////////////////////////////////////////////////////////////////////////////

    let impl_component_extraction = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => {
                impl_component_extraction_named(struct_name.clone(), fields_named.clone())
            }

            Fields::Unnamed(_) => {
                impl_component_extraction_unnamed(struct_name.clone(), arms.len())
            }

            Fields::Unit => panic!("This Macro is only available for Named or Unnamed Structs"),
        },
        _ => panic!("This Macro is only available for structs"),
    };
    let impl_mutcomponents = generate_has_mut_components(arms.len(), impl_component_extraction);

    /////////////////////////////////////////////////////////////////////////////////////////////

    let impl_component = generate_has_component_impl(
        struct_name.clone(),
        arms.clone(),
        &input.data,
        impl_mutcomponents,
    );

    let expanded = quote! {
        #impl_component
    };

    expanded.into()
}

fn generate_has_component_impl(
    struct_name: proc_macro2::Ident,
    arms: Vec<HasComponentArms>,
    data: &Data,
    impl_mut_components: TokenStream2,
) -> TokenStream2 {
    let get_arms = arms.iter().map(|arm| arm.get.clone());
    let get_mut_arms = arms.iter().map(|arm| arm.get_mut.clone());

    let component_types = match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => fields_named
                .named
                .iter()
                .map(|field| {
                    let ty = field.ty.clone();
                    quote! {std::any::TypeId::of::<#ty>()}
                })
                .collect::<Vec<_>>(),
            Fields::Unnamed(fields_unnamed) => fields_unnamed
                .unnamed
                .iter()
                .map(|field| {
                    let ty = field.ty.clone();
                    quote! {std::any::TypeId::of::<#ty>()}
                })
                .collect::<Vec<_>>(),
            Fields::Unit => vec![],
        },
        _ => panic!("Has Component can't be generated for Enums or Unions"),
    };

    quote! {
        impl HasComponent for #struct_name {
            fn get_component<C: 'static>(&self) -> Option<&C> {
                use   std::any::Any;

                use std::any::TypeId;
                match std::any::TypeId::of::<C>() {
                    #(#get_arms)*
                    _ => None
                }
            }

            fn component_types() -> Vec<std::any::TypeId>{
                vec![#(#component_types),*]
            }

            fn get_mut_component<C: 'static>(&mut self) -> Option<&mut C> {
                use   std::any::Any;
                use std::any::TypeId;

                match std::any::TypeId::of::<C>() {
                    #(#get_mut_arms)*
                    _ => None
                }
            }

            #impl_mut_components
        }
    }
}

fn impl_component_extraction_unnamed(struct_name: proc_macro2::Ident, len: usize) -> TokenStream2 {
    // generate identifiers f0, f1, ...
    let idents: Vec<proc_macro2::Ident> = (0..len)
        .map(|i| syn::Ident::new(&format!("f{}", i), proc_macro2::Span::call_site()))
        .collect();

    quote! {
            let #struct_name( #( #idents),* ) = self;
            let components: [Option<&mut dyn   std::any::Any>; #len] = [ #(Some(#idents)),* ];
    }
}

fn impl_component_extraction_named(
    ident: proc_macro2::Ident,
    fields_named: FieldsNamed,
) -> TokenStream2 {
    let fields: Vec<proc_macro2::Ident> = fields_named
        .named
        .iter()
        .map(|field| field.clone().ident)
        .filter_map(|x| x)
        .collect::<Vec<_>>();
    let count: usize = fields_named
        .named
        .iter()
        .filter_map(|x| x.clone().ident)
        .count();

    quote! {
        let #ident{
            #( #fields),*
        } = self;
        let components: [Option<&mut dyn   std::any::Any>; #count] = [
            #(Some(#fields)),*
        ];
    }
}
fn generate_has_mut_components(len: usize, component_extraction: TokenStream2) -> TokenStream2 {
    quote! {
        fn get_mut_components<'a,C: tuple_info::TupleInfo>(
            &'a mut self,
        ) -> Option<<C as tuple_info::TupleInfo>::MutDeconstructedReference<'a>> {
            pub fn reorder_components<'a, const LEN: usize>(
                mut components: [Option<&'a mut dyn  std::any::Any>; LEN],
                type_order: &[std::any::TypeId],
            ) -> Vec<&'a mut dyn  std::any::Any> {
                const NONE: Option<&mut dyn  std::any::Any> = None;
                let mut result: [Option<&'a mut dyn  std::any::Any>; LEN] = [NONE; LEN];

                for (i, &tid) in type_order.iter().enumerate() {
                    // find the first component whose std::any::TypeId matches
                    if let Some(pos) = components
                        .iter()
                        .position(|c| c.as_ref().map(|c| (**c).type_id()) == Some(tid))
                    {
                        // take it out of the original array and put it into result[i]
                        result[i] = components[pos].take();
                    } else {
                        // if any type is missing, fail
                        return vec![];
                    }
                }
                result
                    .into_iter()
                    .collect::<Option<Vec<_>>>()
                    .unwrap_or(vec![])
            }
            #component_extraction

            let type_order = C::types();
            let mut components = reorder_components::<#len>(components, &type_order);
            <C as tuple_info::TupleInfo>::try_mut_deconstruction( components)
        }
    }
}
fn named_struct(fields_named: FieldsNamed) -> Vec<HasComponentArms> {
    fields_named
        .named
        .iter()
        .filter_map(|field| {
            let ty_ident = match &field.ty {
                syn::Type::Path(type_path) => {
                    type_path.path.segments.last().map(|s| s.ident.clone())
                }
                _ => None,
            };
            match (field.ident.clone(), ty_ident) {
                (Some(ident), Some(ty_ident)) => Some((ident, ty_ident)),
                _ => None,
            }
        })
        .filter(|(ident, ty_ident)| {
            let field_name = ident.to_string();
            let ty_name = snake_case(&ty_ident.to_string());
            ty_name == field_name
        })
        .map(|(field_ident, ty_ident)| HasComponentArms {
            get: quote! {
                id if id == std::any::TypeId::of::<#ty_ident>() => {
                    let any = &self.#field_ident as &dyn   std::any::Any;
                    any.downcast_ref::<C>()
                }
            },
            get_mut: quote! {
                id if id == std::any::TypeId::of::<#ty_ident>() => {
                    let any = &mut self.#field_ident as &mut dyn   std::any::Any;
                    any.downcast_mut::<C>()
                }
            },
        })
        .collect()
}

fn unnamed_struct(fields_unnamed: FieldsUnnamed) -> Vec<HasComponentArms> {
    fields_unnamed
        .unnamed
        .iter()
        .enumerate()
        .filter_map(|(field_number, field)| {
            let ty_ident = match &field.ty {
                syn::Type::Path(type_path) => {
                    type_path.path.segments.last().map(|s| s.ident.clone())
                }
                _ => None,
            };
            ty_ident.map(|ty| (Index::from(field_number), ty))
        })
        .map(|(field_number, ty_ident)| HasComponentArms {
            get: quote! {
                id if id == std::any::TypeId::of::<#ty_ident>() => {
                    let any = &self.#field_number as &dyn   std::any::Any;
                    any.downcast_ref::<C>()
                }
            },
            get_mut: quote! {
                id if id == std::any::TypeId::of::<#ty_ident>() => {
                    let any = &mut self.#field_number as &mut dyn   std::any::Any;
                    any.downcast_mut::<C>()
                }
            },
        })
        .collect()
}
