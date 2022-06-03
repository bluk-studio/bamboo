use super::gen_docs;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
  parse_macro_input,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Bracket, Paren},
  Attribute, Field, Fields, FieldsNamed, Ident, ItemEnum, ItemStruct, ItemUnion, Path,
  PathArguments, PathSegment, Token, Type, TypePath, TypeTuple, VisPublic, Visibility,
};

macro_rules! punct {
  [ $($field:expr),* ] => {{
    let mut punct = Punctuated::new();
    punct.extend(vec![$($field),*]);
    punct
  }}
}
macro_rules! fields_named {
  { $($name:ident: $ty:expr,)* } => {
    FieldsNamed { brace_token: Brace { span: Span::call_site() }, named: punct![$(
      Field {
        attrs: vec![],
        vis: Visibility::Public(VisPublic { pub_token: Token![pub](Span::call_site()) }),
        ident: Some(Ident::new(stringify!($name), Span::call_site())),
        colon_token: Some(Token![:](Span::call_site())),
        ty: $ty,
      }
    ),*] }
  }
}
macro_rules! path {
  ( :: $($ident:ident)::* ) => {
    Path {
      leading_colon: Some(Token![::](Span::call_site())),
      segments: punct![$($ident),*],
    }
  };
  ( $($ident:ident)::* ) => {
    Path {
      leading_colon: None,
      segments: punct![$(
        PathSegment {
          ident: Ident::new(stringify!($ident), Span::call_site()),
          arguments: PathArguments::None,
        }
      ),*],
    }
  };
}

#[allow(clippy::collapsible_match)]
pub fn cenum(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemEnum);

  let original_docs = gen_docs(&input);
  let input_attrs = &input.attrs;

  if input.variants.is_empty() {
    let name = &input.ident;
    return quote!(
      #(#input_attrs)*
      #[doc = "Original enum:"]
      #[doc = #original_docs]
      #[repr(C)]
      #[derive(Clone)]
      pub struct #name {}
    )
    .into();
  }

  let name = &input.ident;
  let data_name = Ident::new(&format!("{name}Data"), name.span());
  let fields = input.variants.iter().map(|v| Field {
    attrs:       vec![],
    vis:         Visibility::Public(VisPublic { pub_token: Token![pub](Span::call_site()) }),
    ident:       Some(Ident::new(&format!("f_{}", to_lower(&v.ident.to_string())), v.ident.span())),
    colon_token: Some(Token![:](Span::call_site())),
    ty:          {
      let ty = Type::Tuple(TypeTuple {
        paren_token: Paren { span: v.fields.span() },
        elems:       {
          let mut punct = Punctuated::<Type, Token![,]>::new();
          punct.extend(v.fields.iter().map(|field| field.ty.clone()));
          punct
        },
      });
      if is_copy(&ty) {
        ty
      } else {
        Type::Path(TypePath {
          qself: None,
          path:  Path {
            leading_colon: Some(Token![::](Span::call_site())),
            segments:      punct![
              Ident::new("std", Span::call_site()).into(),
              Ident::new("mem", Span::call_site()).into(),
              PathSegment {
                ident:     Ident::new("ManuallyDrop", Span::call_site()),
                arguments: PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                  colon2_token: None,
                  lt_token:     Token![<](Span::call_site()),
                  args:         punct![syn::GenericArgument::Type(ty)],
                  gt_token:     Token![>](Span::call_site()),
                }),
              }
            ],
          },
        })
      }
    },
  });
  let fields = FieldsNamed {
    brace_token: Brace { span: Span::call_site() },
    named:       {
      let mut punct = Punctuated::new();
      punct.extend(fields);
      punct
    },
  };
  let new_funcs = input.variants.iter().enumerate().map(|(variant, v)| {
    let name = to_lower(&v.ident.to_string());
    let field = Ident::new(&format!("f_{name}"), v.ident.span());
    let new_name = Ident::new(&format!("new_{name}"), v.ident.span());
    let ty = Type::Tuple(TypeTuple {
      paren_token: Paren { span: v.fields.span() },
      elems:       {
        let mut punct = Punctuated::<Type, Token![,]>::new();
        punct.extend(v.fields.iter().map(|field| field.ty.clone()));
        punct
      },
    });
    let convert_manually_drop =
      if is_copy(&ty) { quote!(value) } else { quote!(::std::mem::ManuallyDrop::new(value)) };
    quote!(
      #[allow(unused_parens)]
      pub fn #new_name(value: #ty) -> Self {
        Self {
          variant: #variant,
          data: #data_name { #field: #convert_manually_drop },
        }
      }
    )
  });
  let as_funcs = input.variants.iter().enumerate().map(|(variant, v)| {
    let name = to_lower(&v.ident.to_string());
    let field = Ident::new(&format!("f_{name}"), v.ident.span());
    let as_name = Ident::new(&format!("as_{name}"), v.ident.span());
    let ty = Type::Tuple(TypeTuple {
      paren_token: Paren { span: v.fields.span() },
      elems:       {
        let mut punct = Punctuated::<Type, Token![,]>::new();
        punct.extend(v.fields.iter().map(|field| field.ty.clone()));
        punct
      },
    });
    // Deref will convert the `ManuallyDrop` types into references here.
    quote!(
      #[allow(unused_parens)]
      pub fn #as_name(&self) -> Option<&#ty> {
        if self.variant == #variant {
          unsafe {
            Some(&self.data.#field)
          }
        } else {
          None
        }
      }
    )
  });
  let into_funcs = input.variants.iter().enumerate().map(|(variant, v)| {
    let name = to_lower(&v.ident.to_string());
    let field = Ident::new(&format!("f_{name}"), v.ident.span());
    let into_name = Ident::new(&format!("into_{name}"), v.ident.span());
    let ty = Type::Tuple(TypeTuple {
      paren_token: Paren { span: v.fields.span() },
      elems:       {
        let mut punct = Punctuated::<Type, Token![,]>::new();
        punct.extend(v.fields.iter().map(|field| field.ty.clone()));
        punct
      },
    });
    let convert_manually_drop = if is_copy(&ty) {
      quote!(self.data.#field)
    } else {
      quote!(::std::mem::ManuallyDrop::into_inner(self.data.#field))
    };
    quote!(
      #[allow(unused_parens)]
      pub fn #into_name(self) -> Option<#ty> {
        if self.variant == #variant {
          unsafe {
            Some(#convert_manually_drop)
          }
        } else {
          None
        }
      }
    )
  });
  let clone_match_cases = input.variants.iter().enumerate().map(|(variant, v)| {
    let field = Ident::new(&format!("f_{}", to_lower(&v.ident.to_string())), v.ident.span());
    quote!(
      #variant => #data_name { #field: self.data.#field.clone() },
    )
  });
  let debug_match_cases = input.variants.iter().enumerate().map(|(variant, v)| {
    let field = Ident::new(&format!("f_{}", to_lower(&v.ident.to_string())), v.ident.span());
    let fmt_str = format!("{}({{:?}})", v.ident);
    quote!(
      #variant => write!(f, #fmt_str, self.data.#field.clone()),
    )
  });

  let name = &input.ident;

  let gen_struct = ItemStruct {
    attrs:        vec![Attribute {
      pound_token:   Token![#](Span::call_site()),
      style:         syn::AttrStyle::Outer,
      bracket_token: Bracket { span: Span::call_site() },
      path:          path!(repr),
      tokens:        quote!((C)),
    }],
    vis:          input.vis,
    struct_token: Token![struct](input.enum_token.span()),
    ident:        input.ident.clone(),
    generics:     input.generics.clone(),
    fields:       Fields::Named(fields_named! {
      variant: Type::Path(TypePath { qself: None, path: path!(usize) }),
      data: Type::Path(TypePath { qself: None, path: data_name.clone().into() }),
    }),
    semi_token:   None,
  };
  let gen_union = ItemUnion {
    attrs: vec![Attribute {
      pound_token:   Token![#](Span::call_site()),
      style:         syn::AttrStyle::Outer,
      bracket_token: Bracket { span: Span::call_site() },
      path:          path!(repr),
      tokens:        quote!((C)),
    }],
    vis: Visibility::Public(VisPublic { pub_token: Token![pub](Span::call_site()) }),
    union_token: Token![union](input.enum_token.span()),
    ident: data_name.clone(),
    generics: input.generics,
    fields,
  };

  let struct_docs = gen_docs(&gen_struct);
  let union_docs = gen_docs(&gen_union);

  let out = quote! {
    #(#input_attrs)*
    /// This enum has been converted into a C safe struct and union. The `variant` is a
    /// hint for which variant is stored in the union.
    ///
    /// This struct and union are designed to have any bit configuration, and still be safe
    /// to use. This means that if the `variant` is invalid, the union will contain garbage
    /// data. In the `Clone` impl, the union is literally filled with
    /// `MaybeUninit::uninit().assume_init()`. This is safe, because all the `as_` functions
    /// will return `None` in this case.
    ///
    /// In order for this to truly be valid in every bit configuration, the variant can be
    /// changed without modifying the union. This means that every type in the union must
    /// be valid in any bit configuration. I don't enforce this, but this means that every
    /// variant should implement `wasmer::ValueType`.
    ///
    #[doc = "Original enum:"]
    #[doc = #original_docs]
    #[doc = "Converted to struct:"]
    #[doc = #struct_docs]
    #[doc = "Along with the union:"]
    #[doc = #union_docs]
    #gen_struct
    #[doc = concat!("See [`", stringify!(#name), "`].")]
    #[allow(unused_parens)]
    #[cfg_attr(feature = "host", derive(Clone))]
    #gen_union

    #[cfg(feature = "host")]
    impl Copy for #name {}
    #[cfg(feature = "host")]
    impl Copy for #data_name {}

    impl #name {
      #(#new_funcs)*
      #(#as_funcs)*
      #(#into_funcs)*
    }

    impl Clone for #name {
      fn clone(&self) -> Self {
        unsafe {
          #name {
            variant: self.variant,
            data: match self.variant {
              #(#clone_match_cases)*
              _ => ::std::mem::MaybeUninit::uninit().assume_init(),
            },
          }
        }
      }
    }
    impl ::std::fmt::Debug for #name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        unsafe {
          match self.variant {
            #(#debug_match_cases)*
            _ => write!(f, "<unknown variant {}>", self.variant),
          }
        }
      }
    }
  };
  out.into()
}

fn to_lower(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    if c.is_ascii_uppercase() {
      if !out.is_empty() {
        out.push('_');
      }
      out.push(c.to_ascii_lowercase());
    } else {
      out.push(c);
    }
  }
  out
}

fn is_copy(ty: &Type) -> bool {
  match ty {
    Type::Path(ty) => {
      if let Some(ident) = ty.path.get_ident() {
        ident == "u8"
          || ident == "i8"
          || ident == "u16"
          || ident == "i16"
          || ident == "u32"
          || ident == "i32"
          || ident == "u64"
          || ident == "i64"
          || ident == "f32"
          || ident == "f64"
          || ident == "CBool"
          || ident == "CPos"
      } else {
        false
      }
    }
    Type::Tuple(ty) => ty.elems.iter().any(is_copy),
    _ => todo!("type {ty:?}"),
  }
}