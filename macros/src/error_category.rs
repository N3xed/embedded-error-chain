use crate::str_placeholder;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_error::{abort, emit_error};
use quote::quote;
use std::ops::Deref;
use syn::{
    parse::ParseStream, parse_quote, punctuated::Punctuated, token::Comma, Attribute, DeriveInput,
    Expr, ExprLit, Lit, Meta, MetaList, MetaNameValue, NestedMeta, Path,
};

mod consts {
    /// The maximum value an error code can have.
    pub const MAX_ERROR_CODE: usize = 15;
    /// Maximum number of links.
    pub const MAX_LINKS: usize = 6;

    pub const FMT_PLACEHOLDER_SUMMARY: &str = "summary";
    pub const FMT_PLACEHOLDER_DETAILS: &str = "details";
    pub const FMT_PLACEHOLDER_VARIANT: &str = "variant";
    pub const FMT_PLACEHOLDER_CATEGORY: &str = "category";
    pub const FMT_PLACEHOLDER_DELIM_L: char = '{';
    pub const FMT_PLACEHOLDER_DELIM_R: char = '}';
}

#[derive(Default)]
struct ErrorCategoryAttr {
    name: Option<String>,
    links: Vec<Path>,
    /// This value is `true`, if an enum variant can be trivially converted to and from an
    /// `ErrorCode`.
    ///
    /// For converting an error code back to a variant
    /// `core::mem::transmute()` will be used. This is only valid if the size of the enum
    /// type is equal to the size of `ErrorCode` (which is a `u8`). So for that to be
    /// possible the enum must be `repr(u8)`. When the enum has no variants, `unreachable!()`
    /// will be generated, because the type can never instantiated, so in that case it
    /// being `repr(u8)` is optional.
    is_repr_u8_compatible: bool,
}

impl ErrorCategoryAttr {
    /// Parse the `#[error_category(...)] attribute.
    fn parse(input: &DeriveInput, has_variants: bool) -> ErrorCategoryAttr {
        // Get attribute `error_category`.
        // Error if multiple `error_category` attributes exist.
        // Try to find `repr(u8)` attribute, if `has_variants` is `true`.
        let (attr, is_repr_u8_compatible) = {
            let metas: Vec<_> = input
                .attrs
                .iter()
                .filter_map(|a| a.parse_meta().map_err(|err| emit_error!(err)).ok())
                .collect();

            let is_repr_u8_compatible = !has_variants || {
                let contains_repr_u8 = metas
                    .iter()
                    .filter_map(|m| {
                        if let Meta::List(ml) = m {
                            Some(ml)
                        } else {
                            None
                        }
                    })
                    .find(|m| m.path.is_ident("repr"))
                    .and_then(|ml| {
                        ml.nested.iter().find(|nm| {
                            if let NestedMeta::Meta(m) = *nm {
                                if m.path().is_ident("u8") {
                                    return true;
                                }
                            }
                            false
                        })
                    })
                    .is_some();
                contains_repr_u8
            };

            // get all `error_category` attributes
            let attrs: Vec<_> = metas
                .iter()
                .filter_map(|m| match m {
                    Meta::List(ml) if ml.path.is_ident("error_category") => Some(ml),
                    _ => None,
                })
                .collect();

            // error if we found more than one attribute
            if attrs.len() > 1 {
                emit_error!(
                    attrs[1],
                    "only one `#[error_category(...)]` attribute allowed"
                );
            }

            (
                attrs.first().map(Deref::deref).cloned(),
                is_repr_u8_compatible,
            )
        };

        if let Some(attr) = attr {
            let (name_arg, links_arg, errors) = Self::validate_attr_args(attr.nested);

            // emit all the errors we got back
            errors.into_iter().for_each(|err| match err {
                ErrorCategoryArgError::InvalidArg(m) => emit_error!(
                    m,
                    "invalid attribute argument, expected `name = \"...\"` or `links(...)`"
                ),
                ErrorCategoryArgError::TooManyNameArgs(m) => {
                    emit_error!(m, "at most one `name = \"...\" is allowed")
                }
                ErrorCategoryArgError::TooManyLinksArgs(m) => {
                    emit_error!(m, "at most one `links(...)` is allowed")
                }
            });

            // get the potential `name = "..."` literal
            let name = name_arg.map(|nv| match nv.lit {
                // Note: This is already validated in `validate_error_category_attr_args()`
                syn::Lit::Str(lit) => lit.value(),
                _ => unreachable!(),
            });

            // validate and get the paths inside `links(...)`
            let links = links_arg
                .map(|ml| {
                    let (path_values, invalid): (Vec<_>, Vec<_>) = ml
                        .nested
                        .into_iter()
                        .partition(|nm| matches!(nm, NestedMeta::Meta(Meta::Path(_))));

                    if !invalid.is_empty() {
                        emit_error!(invalid[0], "expected type");
                    }
                    if path_values.len() > consts::MAX_LINKS {
                        emit_error!(
                            path_values[consts::MAX_LINKS],
                            "too many links, at most {} links are allowed",
                            consts::MAX_LINKS
                        );
                    }

                    path_values
                        .into_iter()
                        .map(|nm| match nm {
                            NestedMeta::Meta(Meta::Path(path)) => path,
                            _ => unreachable!(),
                        })
                        .collect()
                })
                .unwrap_or_else(Vec::new);

            ErrorCategoryAttr {
                name,
                links,
                is_repr_u8_compatible,
            }
        } else {
            ErrorCategoryAttr {
                is_repr_u8_compatible,
                ..ErrorCategoryAttr::default()
            }
        }
    }

    /// Validate `error_category` attribute args
    /// Parse `error_category` arguments:
    /// - one optional `name = "literal"`
    /// - one optional `links(<type-list>)` where <type-list> is a comma seperated list of
    ///   0 to 4 types.
    fn validate_attr_args(
        nested: Punctuated<NestedMeta, Comma>,
    ) -> (
        Option<MetaNameValue>,
        Option<MetaList>,
        Vec<ErrorCategoryArgError>,
    ) {
        let (args_matches, args_invalid): (Vec<_>, Vec<_>) =
            nested.into_iter().partition(|nm| matches!(nm, NestedMeta::Meta(Meta::NameValue(_)) | NestedMeta::Meta(Meta::List(_))));

        let mut errors = Vec::new();
        if !args_invalid.is_empty() {
            errors.push(ErrorCategoryArgError::InvalidArg(args_invalid[0].clone()));
        }

        let (name_value_args, list_args): (Vec<_>, Vec<_>) =
            args_matches.into_iter().partition(|nm| match nm {
                NestedMeta::Meta(Meta::NameValue(_)) => true,
                NestedMeta::Meta(Meta::List(_)) => false,
                _ => unreachable!(),
            });

        // validate `name = "..."` args
        let (name_args, invalid): (Vec<_>, Vec<_>) = name_value_args
            .into_iter()
            .map(|nm| match nm {
                NestedMeta::Meta(Meta::NameValue(nv)) => nv,
                _ => unreachable!(),
            })
            .partition(|nv| nv.path.is_ident("name") && matches!(nv.lit, syn::Lit::Str(_)));
        if !invalid.is_empty() {
            errors.push(ErrorCategoryArgError::InvalidArg(NestedMeta::Meta(
                invalid[0].clone().into(),
            )));
        }
        if name_args.len() > 1 {
            errors.push(ErrorCategoryArgError::TooManyNameArgs(name_args[1].clone()));
        }

        // validate `links(...)` args
        // Note: does not validate args inside `(...)`
        let (links_args, invalid): (Vec<_>, Vec<_>) = list_args
            .into_iter()
            .map(|nm| match nm {
                NestedMeta::Meta(Meta::List(nv)) => nv,
                _ => unreachable!(),
            })
            .partition(|nv| nv.path.is_ident("links"));

        if !invalid.is_empty() {
            errors.push(ErrorCategoryArgError::InvalidArg(NestedMeta::Meta(
                invalid[0].clone().into(),
            )));
        }
        if links_args.len() > 1 {
            errors.push(ErrorCategoryArgError::TooManyLinksArgs(
                links_args[1].clone(),
            ));
        }

        let name_arg = name_args.into_iter().next();
        let links_arg = links_args.into_iter().next();

        (name_arg, links_arg, errors)
    }
}

#[derive(Default)]
struct ErrorVariantAttr {
    format_str: String,
    format_args: Vec<Expr>,
    /// `true` if `format_str` contains at least one `{summary}` or `{details}` placeholder, `false` otherwise
    pub doc_comment_placeholder: bool,
}

impl ErrorVariantAttr {
    /// Parse the `args` of the `#[error(args)]` attribute.
    fn parse(args: ParseStream<'_>, attribute: &Attribute) -> Option<ErrorVariantAttr> {
        let args_list: Punctuated<Expr, Comma> = args
            .call(Punctuated::parse_terminated)
            .map_err(|err| emit_error!(err))
            .ok()?;

        let format_str = match args_list.first() {
            Some(Expr::Lit(ExprLit {
                lit: Lit::Str(str_lit),
                ..
            })) => str_lit.value(),
            _ => {
                emit_error!(
                    attribute.tokens,
                    "the first argument must be a format string literal"
                );
                String::new()
            }
        };

        let format_args = args_list.into_iter().skip(1).collect();

        let doc_comment_placeholder = str_placeholder::first_placeholder_range(
            &format_str,
            consts::FMT_PLACEHOLDER_SUMMARY,
            consts::FMT_PLACEHOLDER_DELIM_L,
            consts::FMT_PLACEHOLDER_DELIM_R,
        )
        .is_some()
            || str_placeholder::first_placeholder_range(
                &format_str,
                consts::FMT_PLACEHOLDER_DETAILS,
                consts::FMT_PLACEHOLDER_DELIM_L,
                consts::FMT_PLACEHOLDER_DELIM_R,
            )
            .is_some();

        Some(ErrorVariantAttr {
            format_str,
            format_args,
            doc_comment_placeholder,
        })
    }
}

struct ErrorVariant {
    variant_name: Ident,
    format_str: Option<String>,
    doc_summary: String,
    doc_details: String,
    error_attr: Option<ErrorVariantAttr>,
}

enum DocCommentSectionsParseState {
    Summary,
    EmptyLines,
    Details,
}

impl ErrorVariant {
    /// Parse doc comments
    ///
    /// Note: Multiline comments are not handled currently.
    /// This means that if you have a comment like:
    ///
    /// ```ingore
    /// /***
    ///  *
    ///  */
    /// ```
    ///
    /// All lines between the start (`/***`) and end (`*/`) will contain
    /// starting asterisks (`*`) and potentionally indented whitespace.
    /// We don't remove this because we can't know if it was intentionally
    /// included or just part of the comment format.
    fn parse_doc_comment(comment: String) -> Vec<String> {
        comment
            .split('\n')
            .map(|line| {
                // always remove the first character if it's a whitespace
                let mut chars = line.chars();
                match chars.next() {
                    Some(c) if c.is_whitespace() => chars.as_str().to_owned(),
                    _ => line.to_owned(),
                }
            })
            .collect()
    }

    /// Parse a sequence of lines so that they can be partitioned into all the lines
    /// belonging to the summary, and all lines belonging to the details.
    ///
    /// All lines until the first empty line belong to the summary.
    /// The empty line after the summary is preserved.
    /// The details start at the first non-empty line **after** this empty line, and all
    /// lines thereafter unconditionally belong to the details.
    ///
    /// Example:
    /// ```ignore
    /// <summmary> /// Summary starts here...
    ///            /// some more summary
    /// </summary> /// ...and ends here.
    /// *preserved ///
    ///            ///
    /// <details>  /// Details start here...
    ///            ///
    ///            /// more details
    /// </details> /// ...and end here.
    /// ```
    fn parse_doc_comment_sections(
        state: &mut DocCommentSectionsParseState,
        line: String,
    ) -> Option<(bool, String)> {
        match state {
            DocCommentSectionsParseState::Summary if line.trim().is_empty() => {
                *state = DocCommentSectionsParseState::EmptyLines;
                // preserve the first empty line after the summary
                Some((false, String::new()))
            }
            DocCommentSectionsParseState::Summary => Some((true, line)),
            DocCommentSectionsParseState::EmptyLines if !line.trim().is_empty() => {
                // The first non-empty line after the empty line after the summary
                // starts the details section.
                *state = DocCommentSectionsParseState::Details;
                Some((false, line))
            }
            DocCommentSectionsParseState::EmptyLines => {
                // All empty lines after the empty line after the summary are ignored.
                None
            }
            DocCommentSectionsParseState::Details => {
                // All lines (even empty one) are preserved in the details section.
                Some((false, line))
            }
        }
    }

    /// Parse a enum variant.
    ///
    /// Every enum variant can have one `#[error(...)]` attribute.
    fn parse(variant: &syn::Variant) -> ErrorVariant {
        // Get the `error` attribute.
        // Error there are multiple `error` attributes.
        let attr = {
            let attrs: Vec<_> = variant
                .attrs
                .iter()
                .filter(|a| a.path.is_ident("error"))
                .collect();

            if attrs.len() > 1 {
                emit_error!(attrs[1], "too many `error` attributes"; note = "at most one `#[error(...)]` attribute is allowed");
            }

            attrs.first().and_then(|a| {
                a.parse_args_with(|ps: ParseStream<'_>| Ok(ErrorVariantAttr::parse(ps, a)))
                    .unwrap()
            })
        };
        let parse_doc_comments = attr
            .as_ref()
            .map(|a| a.doc_comment_placeholder)
            .unwrap_or(true);

        // get doc comments only if `parse_doc_comments` is true
        let (summary, details) = if parse_doc_comments {
            let mut doc_comments = variant
                .attrs
                .iter()
                .filter_map(
                    |a| match a.parse_meta().map_err(|err| emit_error!(err)).ok()? {
                        Meta::NameValue(MetaNameValue {
                            lit: syn::Lit::Str(lit_str),
                            path,
                            ..
                        }) if path.is_ident("doc") => Some(lit_str.value()),
                        _ => None,
                    },
                )
                .flat_map(Self::parse_doc_comment)
                // skip all empty lines at the start of the doc comment
                .skip_while(|doc_line| doc_line.trim().is_empty())
                .scan(
                    DocCommentSectionsParseState::Summary,
                    Self::parse_doc_comment_sections,
                );

            let summary = {
                let summary = doc_comments
                    .by_ref()
                    .take_while(|(is_summary, _)| *is_summary)
                    .map(|(_, line)| line.trim().to_owned())
                    .collect::<Vec<String>>()
                    .join(&" ");
                let is_only_whitespace = summary.trim().is_empty();
                if is_only_whitespace {
                    String::new()
                } else {
                    summary
                }
            };

            let details = {
                let details_lines = doc_comments.map(|(_, line)| line).collect::<Vec<String>>();
                let is_only_whitespace = details_lines.iter().all(|line| {
                    line.trim_matches(|c: char| c.is_whitespace() || c == '\r')
                        .is_empty()
                });
                if is_only_whitespace {
                    String::new()
                } else {
                    details_lines.join(&"\n")
                }
            };

            (summary, details)
        } else {
            (String::new(), String::new())
        };

        if !variant.fields.is_empty() {
            emit_error!(
                variant.fields,
                "no fields allowed when deriving `ErrorCategory`"
            );
        }

        ErrorVariant {
            error_attr: attr,
            // This is set in `derive_error_category()`.
            format_str: None,
            doc_summary: summary,
            doc_details: details,
            variant_name: variant.ident.clone(),
        }
    }
}

enum ErrorCategoryArgError {
    InvalidArg(NestedMeta),
    TooManyNameArgs(MetaNameValue),
    TooManyLinksArgs(MetaList),
}

/// Derive the traits `ErrorCategory`, `From<ErrorCode>`, `Into<ErrorCode>` and `core::fmt::Debug`
/// for the given type.
pub fn derive_error_category(input: DeriveInput) -> TokenStream {
    // parse all variants
    let mut variants: Vec<ErrorVariant> = match &input.data {
        syn::Data::Enum(syn::DataEnum { variants, .. }) => variants,
        _ => abort!(input, "`ErrorCategory` can only be derived for enums"),
    }
    .into_iter()
    .map(ErrorVariant::parse)
    .collect();
    // parse the optional `#[error_category(...)]` attribute
    let error_category_attr = ErrorCategoryAttr::parse(&input, !variants.is_empty());

    let enum_ident = input.ident;
    let name_str = error_category_attr
        .name
        .unwrap_or_else(|| enum_ident.to_string());
    let links = error_category_attr.links;

    // replace placeholders in format string
    for v in variants.iter_mut() {
        let (mut format_str, doc_comments_placeholder) = match v {
            ErrorVariant {
                error_attr:
                    Some(ErrorVariantAttr {
                        format_str,
                        doc_comment_placeholder,
                        ..
                    }),
                ..
            } => (format_str.to_owned(), *doc_comment_placeholder),
            ErrorVariant { doc_summary, .. } if !doc_summary.is_empty() => {
                (doc_summary.to_owned(), true)
            }
            _ => continue,
        };

        if doc_comments_placeholder {
            str_placeholder::replace_all_placeholders(
                &mut format_str,
                consts::FMT_PLACEHOLDER_SUMMARY,
                &v.doc_summary,
                consts::FMT_PLACEHOLDER_DELIM_L,
                consts::FMT_PLACEHOLDER_DELIM_R,
            );
            str_placeholder::replace_all_placeholders(
                &mut format_str,
                consts::FMT_PLACEHOLDER_DETAILS,
                &v.doc_details,
                consts::FMT_PLACEHOLDER_DELIM_L,
                consts::FMT_PLACEHOLDER_DELIM_R,
            );
        }
        str_placeholder::replace_all_placeholders(
            &mut format_str,
            consts::FMT_PLACEHOLDER_CATEGORY,
            &name_str,
            consts::FMT_PLACEHOLDER_DELIM_L,
            consts::FMT_PLACEHOLDER_DELIM_R,
        );
        str_placeholder::replace_all_placeholders(
            &mut format_str,
            consts::FMT_PLACEHOLDER_VARIANT,
            &v.variant_name.to_string(),
            consts::FMT_PLACEHOLDER_DELIM_L,
            consts::FMT_PLACEHOLDER_DELIM_R,
        );

        v.format_str = Some(format_str);
    }

    let error_category_impl = {
        let assoc_types: Vec<_> = links
            .iter()
            .cloned()
            .chain(
                std::iter::repeat(parse_quote! { ::embedded_error_chain::marker::Unused })
                    .take(consts::MAX_LINKS - links.len()),
            )
            .enumerate()
            .map(|(i, t)| {
                let ident = Ident::new(&format!("L{}", i), Span::call_site());

                quote! { type #ident = #t; }
            })
            .collect();

        quote! {
            impl ::embedded_error_chain::ErrorCategory for #enum_ident {
                const NAME: &'static str = #name_str;

                #(#assoc_types)*

                fn chainable_category_formatters() -> &'static [::embedded_error_chain::ErrorCodeFormatter] {
                    &[#( ::embedded_error_chain::format_chained::<#links> ),*]
                }
            }
        }
    };

    let from_into_impls = if error_category_attr.is_repr_u8_compatible {
        let discriminant_value_checks: Vec<_> = variants.iter().map(|variant| {
            let max_val_plus_one = (consts::MAX_ERROR_CODE as isize) + 1;
            let variant_name = variant.variant_name.clone();

            let non_negative_msg = format!("`{}::{}` variant discriminant must not be negative", enum_ident.to_string(), variant_name.to_string());
            let err_msg = format!("`{}::{}` variant discriminant must be less than {}", enum_ident.to_string(), variant_name.to_string(), max_val_plus_one);
            quote! {
                ::embedded_error_chain::const_assert!((#enum_ident::#variant_name as isize) >= 0, #non_negative_msg);
                ::embedded_error_chain::const_assert!((#enum_ident::#variant_name as isize) < #max_val_plus_one, #err_msg);
            }
        }).collect();

        let from_error_code_impl = {
            let variant_vals = {
                let vals: Vec<_> = variants
                    .iter()
                    .map(|v| {
                        let variant_name = v.variant_name.clone();
                        quote! {
                            (#enum_ident::#variant_name as ::embedded_error_chain::ErrorCode)
                        }
                    })
                    .collect();

                if vals.is_empty() {
                    vec![quote! { val }]
                } else {
                    vals
                }
            };

            let logic = if variants.is_empty() {
                quote! { unreachable!() }
            } else if variants.len() == 1 {
                let variant_name = variants[0].variant_name.clone();

                quote! { #enum_ident::#variant_name }
            } else {
                quote! {
                    unsafe { ::embedded_error_chain::utils::mem::transmute::<u8, Self>(val as u8) }
                }
            };

            quote! {
                #[automatically_derived]
                impl ::embedded_error_chain::utils::From<::embedded_error_chain::ErrorCode> for #enum_ident {
                    fn from(val: ::embedded_error_chain::ErrorCode) -> #enum_ident {
                        debug_assert!(
                            #(#variant_vals == val)||*,
                            "tried to convert invalid error code to category type"
                        );
                        #logic
                    }
                }
            }
        };

        let into_error_code_impl = {
            let logic = if variants.is_empty() {
                quote! { match self {} }
            } else {
                quote! { self as ::embedded_error_chain::ErrorCode }
            };

            quote! {
                #[automatically_derived]
                impl ::embedded_error_chain::utils::Into<::embedded_error_chain::ErrorCode> for #enum_ident {
                    fn into(self) -> ::embedded_error_chain::ErrorCode {
                        #logic
                    }
                }
            }
        };

        quote! {
            #(#discriminant_value_checks)*
            #from_error_code_impl
            #into_error_code_impl
        }
    } else {
        quote!()
    };

    let fmt_debug_impl = {
        let match_arms: Vec<_> = variants
            .into_iter()
            .map(|v| {
                let write = match (v.format_str, v.error_attr) {
                    (Some(format_str), Some(ErrorVariantAttr { format_args, .. }))
                        if !format_args.is_empty() =>
                    {
                        quote! { ::core::write!(f, #format_str, #(#format_args),*) }
                    }
                    (Some(format_str), _) => quote! { ::core::write!(f, #format_str) },
                    (None, _) => {
                        let variant_name = v.variant_name.to_string();
                        quote! { ::core::write!(f, #variant_name) }
                    }
                };
                let variant_name = &v.variant_name;

                quote! {
                    Self::#variant_name => #write
                }
            })
            .collect();

        quote! {
            #[automatically_derived]
            impl ::embedded_error_chain::utils::Debug for #enum_ident {
                fn fmt(&self, f: &mut ::embedded_error_chain::utils::fmt::Formatter<'_>)
                -> ::embedded_error_chain::utils::fmt::Result {
                    match *self {
                        #(#match_arms),*
                    }
                }
            }
        }
    };

    quote! {
        #error_category_impl
        #from_into_impls
        #fmt_debug_impl
    }
}
