
//! Provides #[derive(Vertex)], which is used to define custom types which can be stored in vertex
//! buffers and accessed from shaders

// TODO (Morten, 09.12.17) Check for repr(C)!

#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

extern crate gondola;

use syn::*;
use proc_macro::TokenStream;

#[proc_macro_derive(Vertex, attributes(location))]
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
        VariantData::Struct(fields) => {
            if fields.is_empty() {
                panic!("Can't #[derive(Vertex)] for a struct with no fields");
            }

            fn get_location(field: &Field) -> Option<usize> {
                for attribute in field.attrs.iter() {
                    if attribute.name() == "location" {
                        if let MetaItem::NameValue(_, Lit::Str(ref v, _)) = attribute.value  {
                            if let Ok(uint) = v.parse::<usize>() {
                                return Some(uint);
                            } else {
                                panic!("Expected #[location = \"<uint>\"], got #[location = \"{}\"]", v);
                            }
                        } else {
                            panic!("Expected #[location = \"<uint>\"]");
                        }
                    }
                }

                return None;
            }

            let expecting_location_attributes = get_location(&fields[0]).is_some();


            // Generate setup_attrib_pointers and shader_input_impl for individual fields
            let mut setup_attrib_pointers_impl = Vec::with_capacity(fields.len()); 
            let mut shader_input_impl = Vec::with_capacity(fields.len());

            let mut next_location = 0;
            for field in fields.iter() {
                let ty = field.ty.clone();
                let ident = field.ident.clone();

                let location;
                if let Some(given_location) = get_location(field) {
                    if !expecting_location_attributes {
                        panic!("Either all or no fields can have #[location = \"<uint>\"] attributes");
                    }

                    location = given_location;
                } else {
                    if expecting_location_attributes {
                        panic!("Either all or no fields can have #[location = \"<uint>\"] attributes");
                    }

                    location = next_location;
                    next_location += 1;
                }

                // NB the code in the quote! macro has access to local variables from the next
                // quote! macro, as it is interpolated into that one
                setup_attrib_pointers_impl.push(quote! {
                    ::gondola::buffer::AttribBinding {
                        index: #location,
                        primitives: <#ty as ::gondola::buffer::VertexData>::primitives(),
                        primitive_type: <<#ty as ::gondola::buffer::VertexData>::Primitive as ::gondola::buffer::GlPrimitive>::GL_ENUM,
                        normalized: false,
                        integer: <<#ty as ::gondola::buffer::VertexData>::Primitive as ::gondola::buffer::GlPrimitive>::IS_INTEGER,
                        stride,
                        offset,
                        divisor,
                    }.enable();

                    offset += ::std::mem::size_of::<#ty>();
                });


                shader_input_impl.push(quote! {
                    let line = format!(
                        "layout(location = {location}) in {glsl_type} {prefix}{name};",
                        name = stringify!(#ident),
                        prefix = name_prefix, // Passed as parameter to function, see final quote!{}
                        location = #location,
                        glsl_type = <#ty as ::gondola::buffer::VertexData>::get_glsl_type(),
                    );
                    result.push_str(&line);
                    result.push('\n');

                    index += 1;
                });
            }

            // Join all the attribute pointer setup code
            let setup_attrib_pointers_impl = quote! {
                let stride = ::std::mem::size_of::<#ident>();

                // This is accessed in the quote! block above
                let mut offset = 0;

                #( #setup_attrib_pointers_impl )*
            };

            // Join all the shader input setup code
            let field_count = fields.len();
            let shader_input_impl = quote! {
                let mut result = String::with_capacity(#field_count * 50); // Approx. 50 chars per primitive
                let mut index = 0; // Used in the above quote! block, which is inserted below

                result.push('\n');
                #( #shader_input_impl )*
                result
            };

            // Generate list of transform feedback outputs
            let field_names = fields.iter()
                .map(|field| field.ident.clone())
                .map(|ident| quote! { #ident })
                .collect::<Vec<_>>();

            // Generate gen_shader_input_decl code
            let transform_feedback_impl = fields.iter()
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
            let field_count = fields.len();
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
                    fn setup_attrib_pointers(divisor: usize) {
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

