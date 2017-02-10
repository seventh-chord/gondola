
#![recursion_limit = "128"]

extern crate gl;
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
            // Retrives the names (Will be needed later)
//            let identifiers = variant_data.fields().iter().map(|field| field.ident.clone());

            // Generate setup_attrib_pointers for individual fields
            // Note that the code in the quote! macro has access to local variables from
            // the next quote! macro, as it is interpolated into that one
            let setup_attrib_pointers_impl = variant_data.fields().iter()
                .map(|field| field.ty.clone())
                .map(|ty| {
                    quote! {
                        let primitives = <#ty as VertexComponent>::primitives();
                        let data_type = <#ty as VertexComponent>::data_type();

                        println!("Implementing for {}", stringify!(#ty));
                        println!("Index: {}, offset: {}", index, offset);

                        unsafe {
                            gl::EnableVertexAttribArray(index);
                            gl::VertexAttribPointer(
                                index as GLuint, primitives as GLint,
                                data_type,
                                false as GLboolean,
                                stride as GLsizei, offset as *const GLvoid
                            );
                        }

                        index += 1;
                        offset += <#ty as VertexComponent>::bytes();
                    }
                });
            // Join all the attrib pointer setup code
            let setup_attrib_pointers_impl = quote! {
                let stride = <#ident as Vertex>::bytes_per_vertex();

                // This is accessed in the quote! block above
                let mut offset = 0;
                let mut index = 0;

                #( #setup_attrib_pointers_impl )*
            };

            // Generate bytes_per_vertex code
            let types = variant_data.fields().iter().map(|field| field.ty.clone());
            let bytes_per_vertex_impl = quote! {
                // Expands to "0 + <first_field as VertexComponent>::primitives() + ..."
                0 #( + <#types as VertexComponent>::bytes())*
            };

            // Join all the code into a single implementation
            quote! {
                impl Vertex for #ident {
                    fn bytes_per_vertex() -> usize {
                        #bytes_per_vertex_impl
                    }

                    #[allow(unused_assignments)]
                    fn setup_attrib_pointers() {
                        #setup_attrib_pointers_impl
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

