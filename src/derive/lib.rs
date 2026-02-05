// SPDX-License-Identifier: Apache-2.0

//! Derive for Nipart Internal use
//!
//! [JsonDisplay]: Implement `std::fmt::Display` trait using JSON output and
//! fallback to Debug display.
//!
//! [JsonDisplayHideSecrets]: Implement `std::fmt::Display` trait using JSON
//! output and fallback to Debug display. Will invoke
//! `self.clone().hide_secrets()` to hide the password before displaying.
//! User of derive should also make sure secrets not leak by Debug trait.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(JsonDisplay)]
pub fn derive_json_display(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let class_name = &input.ident;

    // Build the output, possibly using quasi-quotation
    let expanded = quote::quote! {
        impl std::fmt::Display for #class_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match serde_json::to_string(&self) {
                    Ok(s) => {
                        // For simple string, remove the quote.
                        if s.matches('"').count() == 2
                            && let Some(s) =
                                s.strip_prefix('"')
                                    .and_then(|s| s.strip_suffix('"'))
                        {
                            write!(f, "{}", s)
                        } else {
                            write!(f, "{}", s)
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "BUG: Failed to convert {self:?} into JSON: {e}"
                        );
                        write!(f, "{self:?}")
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(JsonDisplayHideSecrets)]
pub fn derive_json_display_hide_secrets(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let class_name = &input.ident;

    // Build the output, possibly using quasi-quotation
    let expanded = quote::quote! {
        impl std::fmt::Display for #class_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut self_clone = self.clone();
                self_clone.hide_secrets();
                match serde_json::to_string(&self_clone) {
                    Ok(s) => {
                        // For simple string, remove the quote.
                        if s.matches('"').count() == 2
                            && let Some(s) =
                                s.strip_prefix('"')
                                    .and_then(|s| s.strip_suffix('"'))
                        {
                            write!(f, "{}", s)
                        } else {
                            write!(f, "{}", s)
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "BUG: Failed to convert {self_clone:?} \
                            into JSON: {e}"
                        );
                        write!(f, "{self_clone:?}")
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(DebugHideSecrets)]
pub fn derive_debug_hide_secrets(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let class_name = &input.ident;

    // Build the output, possibly using quasi-quotation
    let expanded = quote::quote! {
        impl std::fmt::Debug for #class_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut self_clone = self.clone();
                self_clone.hide_secrets();
                match serde_json::to_string(&self_clone) {
                    Ok(s) => {
                        // For simple string, remove the quote.
                        if s.matches('"').count() == 2
                            && let Some(s) =
                                s.strip_prefix('"')
                                    .and_then(|s| s.strip_suffix('"'))
                        {
                            write!(f, "{}", s)
                        } else {
                            write!(f, "{}", s)
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "BUG: Failed to convert {self_clone:?} \
                            into JSON: {e}"
                        );
                        write!(f, "{self_clone:?}")
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}
