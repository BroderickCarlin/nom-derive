//! This crate provides Nom's derive macros.
//!
//! ```rust
//! # use nom_derive::Nom;
//! # use nom::{do_parse,IResult,be_u32,call};
//! #
//! #[derive(Nom)]
//! # struct S(u32);
//! #
//! # fn main() {}
//! ```
//!
//! For more documentation and examples, see the [Nom derive](derive.Nom.html) documentation.

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::*;
use syn::export::Span;


mod parsertree;
mod structs;
mod enums;

use structs::parse_struct;
use enums::impl_nom_enums;

/// The `Nom` derive automatically generates a `parse` function for the structure
/// using [nom] parsers. It will try to infer parsers for primitive of known
/// types, but also allows you to specify parsers using custom attributes.
///
/// [nom]: https://github.com/Geal/nom
///
/// # Deriving parsers for `Struct`
///
/// For simple structures, the parsers are automatically generated:
///
/// ```rust
/// use nom_derive::Nom;
/// use nom::{do_parse,IResult,be_u16,be_u32,call};
///
/// #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S {
///   a: u32,
///   b: u16,
///   c: u16
/// }
///
/// # fn main() {
/// let input = b"\x00\x00\x00\x01\x12\x34\x56\x78";
/// let res = S::parse(input);
/// assert_eq!(res, Ok((&input[8..],S{a:1,b:0x1234,c:0x5678})));
/// # }
/// ```
///
/// This also work for tuple structs:
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,be_u16,be_u32,call};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S(u32);
/// #
/// # fn main() {
/// # let input = b"\x00\x00\x00\x01";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[4..],S(1))));
/// # }
/// ```
///
/// By default, integers are parsed are Big Endian.
///
/// `nom-derive` is also able to derive default parsers for some usual types:
///
/// ## Option types
///
/// If a field is an `Option<T>`, the generated parser is `opt!(complete!(T::parse))`
///
/// For ex:
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,opt,complete,be_u32};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S {
///   a: Option<u32>
/// }
///
/// # fn main() {
/// let input = b"\x00\x00\x00\x01";
/// let res = S::parse(input);
/// assert_eq!(res, Ok((&input[4..],S{a:Some(1)})));
/// # }
/// ```
///
/// ## Vec types
///
/// If a field is an `Vec<T>`, the generated parser is `many0!(complete!(T::parse))`
///
/// For ex:
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,many0,complete,be_u16};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S {
///   a: Vec<u16>
/// }
///
/// # fn main() {
/// let input = b"\x00\x00\x00\x01";
/// let res = S::parse(input);
/// assert_eq!(res, Ok((&input[4..],S{a:vec![0,1]})));
/// # }
/// ```
///
/// The `Count(n)` attribute can be used to specify the number of items to parse.
///
/// Notes:
///   - the subparser is inferred as usual (item type must be `Vec< ... >`)
///   - the number of items (`n`) can be any expression, and will be cast to `usize`
///
/// For ex:
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,call,count,be_u16};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S {
///   a: u16,
///   #[Count="a"]
///   b: Vec<u16>
/// }
/// #
/// # fn main() {
/// # let input = b"\x00\x01\x12\x34";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[4..],S{a:1, b:vec![0x1234]})));
/// # }
/// ```
///
/// ## Default parsing function
///
/// If a field with type `T` is not a primitive or known type, the generated parser is
/// `call!(T::parse)`.
///
/// This function can be automatically derived, or specified as a method for the struct.
/// In that case, the function must be a static method with the same API as a
/// [nom] combinator, returning the wrapped struct when parsing succeeds.
///
/// Example (using `Nom` derive):
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,call,be_u16};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S2 {
///   c: u16
/// }
///
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S {
///   a: u16,
///   b: S2
/// }
/// #
/// # fn main() {
/// # let input = b"\x00\x00\x00\x01";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[4..],S{a:0,b:S2{c:1}})));
/// # }
/// ```
///
/// Example (defining `parse` method):
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,call,map,be_u16,le_u16};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// // no Nom derive
/// struct S2 {
///   c: u16
/// }
///
/// impl S2 {
///     fn parse(i:&[u8]) -> IResult<&[u8],S2> {
///         map!(
///             i,
///             call!(le_u16), // little-endian
///             |c| S2{c} // return a struct S2
///             )
///     }
/// }
///
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S {
///   a: u16,
///   b: S2
/// }
/// #
/// # fn main() {
/// # let input = b"\x00\x00\x00\x01";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[4..],S{a:0,b:S2{c:256}})));
/// # }
/// ```
///
/// ## Specifying parsers
///
/// Sometimes, the default parsers generated automatically are not those you
/// want.
///
/// The `Parse` custom attribute allows for specifying the parser, using code that
/// will be inserted in the `do_parse` block of the nom parser.
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,call,le_u16};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S{
///     #[Parse="le_u16"]
///     a: u16
/// }
/// #
/// # fn main() {
/// # let input = b"\x00\x01";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[2..],S{a:256})));
/// # }
/// ```
///
/// The `Parse` argument can be a complex expression:
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,be_u8,be_u16,call,cond};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S{
///     pub a: u8,
///     #[Parse="cond!(a > 0,be_u16)"]
///     pub b: Option<u16>,
/// }
/// #
/// # fn main() {
/// # let input = b"\x01\x00\x01";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[3..],S{a:1,b:Some(1)})));
/// # }
/// ```
/// Note that you are responsible from providing correct code.
///
/// ## Adding conditions
///
/// The `Cond` custom attribute allows for specifying a condition.
/// The generated parser will use the `cond!` combinator, which calls the
/// child parser only if the condition is met.
/// The type with this attribute must be an `Option` type.
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,cond,complete,be_u8,be_u16,call};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S{
///     pub a: u8,
///     #[Cond="a == 1"]
///     pub b: Option<u16>,
/// }
/// #
/// # fn main() {
/// # let input = b"\x01\x00\x01";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[3..],S{a:1,b:Some(1)})));
/// # }
/// ```
///
/// ## Adding verifications
///
/// The `Verify` custom attribute allows for specifying a verifying function.
/// The generated parser will use the `verify!` combinator, which calls the
/// child parser only if is verifies a condition (and otherwise raises an error).
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::{do_parse,IResult,verify,complete,be_u8,be_u16,call};
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// struct S{
///     #[Verify="a == 1"]
///     pub a: u8,
/// }
/// #
/// # fn main() {
/// # let input = b"\x01";
/// # let res = S::parse(input);
/// # assert_eq!(res, Ok((&input[1..],S{a:1})));
/// # }
/// ```
///
/// ## Known problems
///
/// The generated parsers use the [nom] combinators directly, so they must be
/// visible in the current namespace (*i.e* imported in a `use` statement).
///
/// # Deriving parsers for `Enum`
///
/// The `Nom` attribute can also used to generate parser for `Enum` types.
/// The generated parser will used a value (called *selector*) to determine
/// which attribute variant is parsed.
/// Named and unnamed enums are supported.
///
/// In addition of `derive(Nom)`, a `Selector` attribute must be used:
///   - on the structure, to specify the type of selector to match
///   - on each variant, to specify the value associated with this variant.
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::*;
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// #[Selector="u8"]
/// pub enum U1{
///     #[Selector("0")] Field1(u32),
///     #[Selector("1")] Field2(Option<u32>),
/// }
/// #
/// # fn main() {
/// # let input = b"\x00\x00\x00\x02";
/// # let res = U1::parse(input, 0);
/// # assert_eq!(res, Ok((&input[4..],U1::Field1(2))));
/// # }
/// ```
///
/// The generated function will look like:
///
/// <pre>
/// impl U1{
///     pub fn parse(i:&[u8), selector: u8) -> IResult<&[u8],U1> {
///         match selector {
///             ...
///         }
///     }
/// }
/// </pre>
///
/// It can be called either directly (`U1::parse(n)`) or using nom
/// (`call!(U1::parse,n)`).
///
/// The selector can be a primitive type (`u8`), or any other type implementing the `PartialEq`
/// trait.
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::*;
/// #
/// #[derive(Debug,PartialEq,Eq,Clone,Copy,Nom)]
/// pub struct MessageType(pub u8);
///
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// #[Selector="MessageType"]
/// pub enum U1{
///     #[Selector("MessageType(0)")] Field1(u32),
///     #[Selector("MessageType(1)")] Field2(Option<u32>),
/// }
///
/// // Example of call from a struct:
/// #[derive(Nom)]
/// pub struct S1{
///     pub msg_type: MessageType,
///     #[Parse="call!(U1::parse,msg_type)"]
///     pub msg_value: U1
/// }
/// #
/// # fn main() {
/// # let input = b"\x00\x00\x00\x02";
/// # let res = U1::parse(input, MessageType(0));
/// # assert_eq!(res, Ok((&input[4..],U1::Field1(2))));
/// # }
/// ```
///
/// ## Default case
///
/// By default, if no value of the selector matches the input value, a nom error
/// `ErrorKind::Switch` is raised. This can be changed by using `_` as selector
/// value for one the variants.
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::*;
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[derive(Nom)]
/// #[Selector="u8"]
/// pub enum U2{
///     #[Selector("0")] Field1(u32),
///     #[Selector("_")] Field2(u32),
/// }
/// #
/// # fn main() {
/// # let input = b"\x00\x00\x00\x02";
/// # let res = U2::parse(input, 123);
/// # assert_eq!(res, Ok((&input[4..],U2::Field2(2))));
/// # }
/// ```
///
/// If the `_` selector is not the last variant, the generated code will use it
/// as the last match to avoid unreachable code.
///
/// ## Special case: specifying parsers for fields
///
/// Sometimes, an unnamed field requires a custom parser. In that case, the
/// *field* (not the variant) must be annotated with attribute `Parse`.
///
/// Named fields:
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::*;
/// #
/// # #[derive(Debug,PartialEq,Eq,Clone,Copy,Nom)]
/// # pub struct MessageType(pub u8);
/// #
/// #[derive(Nom)]
/// #[Selector="MessageType"]
/// pub enum U3<'a>{
///     #[Selector("MessageType(0)")] Field1{a:u32},
///     #[Selector("MessageType(1)")] Field2{
///         #[Parse="take!(4)"]
///         a: &'a[u8]
///     },
/// }
/// ```
///
/// Unnamed fields:
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::*;
/// #
/// # #[derive(Debug,PartialEq,Eq,Clone,Copy,Nom)]
/// # pub struct MessageType(pub u8);
/// #
/// #[derive(Nom)]
/// #[Selector="MessageType"]
/// pub enum U3<'a>{
///     #[Selector("MessageType(0)")] Field1(u32),
///     #[Selector("MessageType(1)")] Field2(
///         #[Parse="take!(4)"] &'a[u8]
///     ),
/// }
/// ```
///
/// ## Special case: fieldless enums
///
/// If the entire enum is fieldless (a list of constant integer values), a
/// parser can be derived if
///   - the `Enum` has a `repr(ty)` attribute, with `ty` an integer type
///   - the `Enum` implements the `Eq` trait
///
/// In that case, the `Selector` attribute must *not* be specified.
///
/// ```rust
/// # use nom_derive::Nom;
/// # use nom::*;
/// #
/// # #[derive(Debug,PartialEq)] // for assert_eq!
/// #[repr(u8)]
/// #[derive(Eq,Nom)]
/// pub enum U3{
///     A,
///     B = 2,
///     C
/// }
/// #
/// # fn main() {
/// # let empty : &[u8] = b"";
/// # assert_eq!(
/// #     U3::parse(b"\x00"),
/// #     Ok((empty,U3::A))
/// # );
/// # assert!(
/// #     U3::parse(b"\x01").is_err()
/// # );
/// # assert_eq!(
/// #     U3::parse(b"\x02"),
/// #     Ok((empty,U3::B))
/// # );
/// # }
/// ```
///
/// The generated parser will parse an element of type `ty` (as Big Endian), try
/// to match to enum values, and return an instance of `Enum` if it succeeds
/// (wrapped in an `IResult`).
///
/// For ex, `U3::parse(b"\x02")` will return `Ok((&b""[..],U3::B))`.
///
/// ## Limitations
///
/// Except if the entire enum is fieldless (a list of constant integer values),
/// unit fields are not supported.
#[proc_macro_derive(Nom, attributes(Parse,Verify,Cond,Count,Selector))]
pub fn nom(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let ast = parse_macro_input!(input as DeriveInput);

    // Build the impl
    let gen = impl_nom(&ast, false);

    // Return the generated impl
    gen
}

fn impl_nom(ast: &syn::DeriveInput, debug:bool) -> TokenStream {
    // eprintln!("ast: {:#?}", ast);
    // test if struct has a lifetime
    let s =
        match &ast.data {
            &syn::Data::Enum(_)       => { return impl_nom_enums(ast, debug); },
            &syn::Data::Struct(ref s) => parse_struct(s),
            &syn::Data::Union(_)       => panic!("Unions not supported"),
    };
    // parse string items and prepare tokens for each field parser
    let generics = &ast.generics;
    let name = &ast.ident;
    let (idents,parser_tokens) : (Vec<_>,Vec<_>) = s.parsers.iter()
        .map(|(name,parser)| {
            let id = syn::Ident::new(name, Span::call_site());
            (id,parser)
        })
        .unzip();
    let idents2 = idents.clone();
    // Code generation
    let struct_def = match s.unnamed {
        false => quote!{ ( #name { #(#idents2),* } ) },
        true  => quote!{ ( #name ( #(#idents2),* ) ) },
    };
    let tokens = quote! {
        impl#generics #name#generics {
            fn parse(i: &[u8]) -> IResult<&[u8],#name> {
                do_parse!{
                    i,
                    #(#idents: #parser_tokens >>)*
                    #struct_def
                }
            }
        }
    };
    if debug {
        eprintln!("tokens:\n{}", tokens);
    }
    tokens.into()
}

/// This derive macro behaves exactly like [Nom derive](derive.Nom.html), except it
/// prints the generated parser on stderr.
/// This is helpful for debugging generated parsers.
#[proc_macro_derive(NomDeriveDebug, attributes(Parse,Verify,Cond,Count,Selector))]
pub fn nom_derive_debug(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let ast = parse_macro_input!(input as DeriveInput);

    // Build the impl
    let gen = impl_nom(&ast, true);

    // Return the generated impl
    gen
}
