
//! Provides custom derives for the gondola crate.
//!
//! Currently only provides `#[derive(Vertex)]`

#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use syn::*;
use proc_macro::TokenStream;

#[proc_macro_derive(Vertex)]
pub fn vertex(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_macro_input(&s).unwrap();

    let ident = ast.ident;
    let gen = match ast.body {
        Body::Enum(..) => panic!("#[derive(Vertex)] is only defined for structs, not enums"),
        Body::Struct(variant_data) => impl_vertex(ident, variant_data)
    };

    gen.parse().unwrap()
}

fn impl_vertex(ident: Ident, variant_data: VariantData) -> quote::Tokens {
    match variant_data {
        VariantData::Struct(..) => {
            // Generate setup_attrib_pointers for individual fields
            // Note that the code in the quote! macro has access to local variables from
            // the next quote! macro, as it is interpolated into that one
            let setup_attrib_pointers_impl = variant_data.fields().iter()
                .map(|field| field.ty.clone())
                .map(|ty| {
                    quote! {
                        ::gondola::buffer::AttribBinding {
                            index: index,
                            primitives: <#ty as ::gondola::buffer::VertexData>::primitives(),
                            primitive_type: <<#ty as ::gondola::buffer::VertexData>::Primitive as ::gondola::buffer::GlPrimitive>::gl_enum(),
                            normalized: false,
                            integer: <<#ty as ::gondola::buffer::VertexData>::Primitive as ::gondola::buffer::GlPrimitive>::is_integer(),
                            stride: stride,
                            offset: offset,
                            divisor: 0,
                        }.enable();

                        index += 1;
                        offset += <#ty as ::gondola::buffer::VertexData>::bytes();
                    }
                });
            // Join all the attribute pointer setup code
            let setup_attrib_pointers_impl = quote! {
                let stride = <#ident as ::gondola::buffer::Vertex>::bytes_per_vertex();

                // This is accessed in the quote! block above
                let mut offset = 0;
                let mut index = 0;

                #( #setup_attrib_pointers_impl )*
            };

            // Generate bytes_per_vertex code
            let types = variant_data.fields().iter().map(|field| field.ty.clone());
            let bytes_per_vertex_impl = quote! {
                // Expands to "0 + <first_field as VertexData>::primitives() + ..."
                0 #( + <#types as ::gondola::buffer::VertexData>::bytes())*
            };

            // Generate gen_shader_input_decl code
            let shader_input_impl = variant_data.fields().iter()
                .map(|field| (field.ident.clone(), field.ty.clone()))
                .map(|(ident, ty)| {
                    quote! {
                        let line = format!(
                            "layout(location = {index}) in {glsl_type} {prefix}{name};",
                            name = stringify!(#ident),
                            prefix = name_prefix, // Passed as parameter to function, see final quote!{}
                            index = index,
                            glsl_type = <#ty as ::gondola::buffer::VertexData>::get_glsl_type(),
                        );
                        result.push_str(&line);
                        result.push('\n');

                        index += 1;
                    }
                });
            // Join all the shader input setup code
            let field_count = variant_data.fields().len();
            let shader_input_impl = quote! {
                let mut result = String::with_capacity(#field_count * 50); // Approx. 50 chars per primitive
                let mut index = 0; // Used in the above quote! block, which is inserted below
                result.push('\n');
                #( #shader_input_impl )*
                result
            };

            // Generate list of transform feedback outputs
            let field_names = variant_data.fields().iter()
                .map(|field| field.ident.clone())
                .map(|ident| quote! { #ident })
                .collect::<Vec<_>>();

            // Generate gen_shader_input_decl code
            let transform_feedback_impl = variant_data.fields().iter()
                .map(|field| (field.ident.clone(), field.ty.clone()))
                .map(|(ident, ty)| {
                    quote! {
                        let line = format!(
                            "out {glsl_type} {prefix}{name};",
                            name = stringify!(#ident),
                            prefix = name_prefix, // Passed as parameter to function, see final quote!{}
                            glsl_type = <#ty as ::gondola::buffer::VertexData>::get_glsl_type(),
                        );
                        result.push_str(&line);
                        result.push('\n');
                        index += 1;
                    }
                });
            // Join all the transform feedback output setup code
            let field_count = variant_data.fields().len();
            let transform_feedback_impl = quote! {
                let mut result = String::with_capacity(#field_count * 20); // Approx. 20 chars per primitive
                let mut index = 0; // Used in the above quote! block, which is inserted below
                result.push('\n');
                #( #transform_feedback_impl )*
                result
            };

            // Join all the code into a single implementation
            quote! {
                #[allow(unused_assignments, unused_variables)]
                impl ::gondola::buffer::Vertex for #ident {
                    fn bytes_per_vertex() -> usize {
                        #bytes_per_vertex_impl
                    }

                    fn setup_attrib_pointers() {
                        #setup_attrib_pointers_impl
                    }

                    fn gen_shader_input_decl(name_prefix: &str) -> String {
                        #shader_input_impl
                    }

                    fn gen_transform_feedback_decl(name_prefix: &str) -> String {
                        #transform_feedback_impl
                    }

                    fn gen_transform_feedback_outputs(name_prefix: &str) -> Vec<String> {
                        vec![
                            #(
                                // This line is repeated for each field name 
                                format!("{}{}", name_prefix, stringify!(#field_names))
                            ),*
                        ]
                    }
                }
            }
        },
        VariantData::Tuple(..) => {
            panic!("#[derive(Vertex)] is not defined for tupple structs");
        },
        VariantData::Unit => {
            panic!("#[derive(Vertex)] is not defined for unit structs");
        }
    }
}

