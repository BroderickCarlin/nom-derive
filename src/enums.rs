use proc_macro::TokenStream;
use syn;
use syn::export::Span;

use crate::parsertree::ParserTree;
use crate::structs::{parse_fields,StructParserTree};

#[derive(Debug)]
struct VariantParserTree{
    pub ident: syn::Ident,
    pub selector: String,
    pub struct_def: StructParserTree,
}

fn parse_variant(variant: &syn::Variant) -> VariantParserTree {
    // eprintln!("variant: {:?}", variant);
    let selector = get_selector(&variant.attrs).expect(&format!("The 'Selector' attribute must be used to give the value of selector item (variant {})", variant.ident));
    let struct_def = parse_fields(&variant.fields);
    // discriminant ?
    VariantParserTree{
        ident: variant.ident.clone(),
        selector,
        struct_def
    }
}

fn get_selector(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if let Ok(ref meta) = attr.parse_meta() {
            match meta {
                syn::Meta::NameValue(ref namevalue) => {
                    if &namevalue.ident == &"Selector" {
                        match &namevalue.lit {
                            syn::Lit::Str(litstr) => {
                                return Some(litstr.value())
                            },
                            _ => panic!("unsupported namevalue type")
                        }
                    }
                }
                syn::Meta::List(ref metalist) => {
                    if &metalist.ident == &"Selector" {
                        for n in metalist.nested.iter() {
                            match n {
                                syn::NestedMeta::Literal(lit) => {
                                    match lit {
                                        syn::Lit::Str(litstr) => {
                                            return Some(litstr.value())
                                        },
                                        _ => panic!("unsupported literal type")
                                    }
                                },
                                _ => panic!("unsupported meta type")
                            }
                        }
                    }
                }
                syn::Meta::Word(_) => ()
            }
        }
    }
    None
}

fn get_repr(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if let Ok(ref meta) = attr.parse_meta() {
            match meta {
                syn::Meta::NameValue(_) => (),
                syn::Meta::List(ref metalist) => {
                    if &metalist.ident == &"repr" {
                        for n in metalist.nested.iter() {
                            match n {
                                syn::NestedMeta::Meta(meta) => {
                                    match meta {
                                        syn::Meta::Word(word) => {
                                            return Some(word.to_string())
                                        },
                                        _ => panic!("unsupported nested type for 'repr'")
                                    }
                                },
                                _ => panic!("unsupported meta type for 'repr'")
                            }
                        }
                    }
                }
                syn::Meta::Word(_) => ()
            }
        }
    }
    None
}

fn is_input_fieldless_enum(ast: &syn::DeriveInput) -> bool {
    match ast.data {
        syn::Data::Enum(ref data_enum) => {
            // eprintln!("{:?}", data_enum);
            data_enum.variants.iter()
                .fold(true,
                      |acc, v| {
                          if let syn::Fields::Unit = v.fields { acc } else { false }
                      })
        },
        _ => false
    }
}

fn impl_nom_fieldless_enums(ast: &syn::DeriveInput, repr:String, debug:bool) -> TokenStream {
    let parser = match repr.as_ref() {
        "u8"  |
        "u16" |
        "u32" |
        "u64" |
        "i8"  |
        "i16" |
        "i32" |
        "i64"    => ParserTree::Raw(format!("be_{}", repr)),
        _ => panic!("Cannot parse 'repr' content")
    };
    let variant_names : Vec<_> =
        match ast.data {
            syn::Data::Enum(ref data_enum) => {
                // eprintln!("{:?}", data_enum);
                data_enum.variants.iter()
                    .map(|v| {
                        v.ident.to_string()
                    })
                    .collect()
            },
            _ => { panic!("expect enum"); }
        };
    let generics = &ast.generics;
    let name = &ast.ident;
    let ty = syn::Ident::new(&repr, Span::call_site());
    let variants_code : Vec<_> =
        variant_names.iter()
            .map(|variant_name| {
                let id = syn::Ident::new(variant_name, Span::call_site());
                quote!{ if selector == #name::#id as #ty { return Some(#name::#id); } }
            })
            .collect();
    let tokens = quote!{
        impl#generics #name#generics {
            fn parse(i: &[u8]) -> IResult<&[u8],#name> {
                map_opt!(
                    i,
                    #parser,
                    |selector| {
                        #(#variants_code)*
                        None
                    }
                )
            }
        }
    };
    if debug {
        eprintln!("impl_nom_enums: {}", tokens);
    }

    tokens.into()
}

pub(crate) fn impl_nom_enums(ast: &syn::DeriveInput, debug:bool) -> TokenStream {
    let name = &ast.ident;
    // eprintln!("{:?}", ast.attrs);
    let selector = match get_selector(&ast.attrs) { //.expect("The 'Selector' attribute must be used to give the type of selector item");
        Some(s) => s,
        None    => {
            if is_input_fieldless_enum(ast) {
                // check that we have a repr attribute
                let repr = get_repr(&ast.attrs).expect("Nom-derive: fieldless enums must have a 'repr' attribute");
                return impl_nom_fieldless_enums(ast, repr, debug);
            } else {
                panic!("Nom-derive: enums must specify the 'selector' attribute");
            }
        }
    };
    let mut variants_defs : Vec<_> =
        match ast.data {
            syn::Data::Enum(ref data_enum) => {
                // eprintln!("{:?}", data_enum);
                data_enum.variants.iter()
                    .map(parse_variant)
                    .collect()
            },
            _ => { panic!("expect enum"); }
        };
    // parse string items and prepare tokens for each variant
    let generics = &ast.generics;
    let selector_type : proc_macro2::TokenStream = selector.parse().unwrap();
    let mut default_case_handled = false;
    let mut variants_code : Vec<_> = {
        variants_defs.iter()
            .map(|def| {
                if def.selector == "_" { default_case_handled = true; }
                let m : proc_macro2::TokenStream = def.selector.parse().expect("invalid selector value");
                let variantname = &def.ident;
                let (idents,parser_tokens) : (Vec<_>,Vec<_>) = def.struct_def.parsers.iter()
                    .map(|(name,parser)| {
                        let id = syn::Ident::new(name, Span::call_site());
                        (id,parser)
                    })
                    .unzip();
                let idents2 = idents.clone();
                let struct_def = match def.struct_def.unnamed {
                    false => quote!{ ( #name::#variantname { #(#idents2),* } ) },
                    true  => quote!{ ( #name::#variantname ( #(#idents2),* ) ) },
                };
                quote!{
                    #m => {
                        do_parse!{
                            i,
                            #(#idents: #parser_tokens >>)*
                            #struct_def
                        }
                        // Err(nom::Err::Error(error_position!(i, nom::ErrorKind::Switch)))
                    },
                }
            })
            .collect()
    };
    // if we have a default case, make sure it is the last entry
    if default_case_handled {
        let pos = variants_defs.iter()
            .position(|def| def.selector == "_")
            .expect("default case is handled but couldn't find index");
        let last_index = variants_defs.len() - 1;
        if pos != last_index {
            variants_defs.swap(pos, last_index);
            variants_code.swap(pos, last_index);
        }
    }
    // generate code
    let default_case =
        if default_case_handled { quote!{} }
        else { quote!{ _ => Err(nom::Err::Error(error_position!(i, nom::ErrorKind::Switch))) } };
    let tokens = quote!{
        impl#generics #name#generics {
            fn parse(i: &[u8], selector: #selector_type) -> IResult<&[u8],#name> {
                match selector {
                    #(#variants_code)*
                    #default_case
                }
            }
        }
    };

    if debug {
        eprintln!("impl_nom_enums: {}", tokens);
    }

    tokens.into()
}
