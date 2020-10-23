use proc_macro2::{Ident, Span};
use quote::quote;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use toml;

static DATA: &str = include_str!("data.toml");

#[derive(Debug, Deserialize, Clone)]
enum DeserializableTocFormat {
    NoTocEntry,
    TitleOnly,
    TitleAndLabel,
    Provided(String),
}

impl DeserializableTocFormat {
    fn to_representation(self) -> proc_macro2::TokenStream {
        use DeserializableTocFormat::*;

        match self {
            NoTocEntry => quote! {TocFormat::NoTocEntry},
            TitleOnly => quote! {TocFormat::TitleOnly},
            TitleAndLabel => quote! {TocFormat::TitleAndLabel},
            Provided(s) => quote! {TocFormat::Provided(#s)},
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct Specification {
    header_level: Option<usize>,
    header_classes: Option<String>,
    section_classes: Option<String>,
    epub_type: String,
    matter: String,
    default_toc_format: DeserializableTocFormat,
    #[serde(default = "default_true")]
    include_stylesheet: bool,
    additional_head: Option<String>,
    section_wrapper: Option<String>,
    default_toc_level: Option<usize>,
}

fn main() {
    let data: HashMap<String, Specification> = toml::from_str(DATA).expect("Error deserializing: ");

    macro_rules! optional_data_pattern {
		($field:ident) => {{
			data.iter()
				.map(|(role, vals)| {
					let role = Ident::new(&role, Span::call_site());
					(role, vals.$field.clone())
				})
				.map(|(role, val)| {
					let val = if let Some(v) = val {
						quote!(Some(#v))
					} else {
						quote!(None)
					};
					(role, val)
				})
				.map(|(role, val)| quote!(#role => #val))
		}};
	}

    macro_rules! required_data_pattern {
		($field:ident) => {{
			data.iter()
				.map(|(role, vals)| {
					let role = Ident::new(&role, Span::call_site());
					(role, vals.$field.clone())
				})
				.map(|(role, val)| quote!(#role => #val))
		}};
	}

    let header_level_data = optional_data_pattern!(header_level);
    let header_classes_data = optional_data_pattern!(header_classes);
    let section_classes_data = optional_data_pattern!(section_classes);
    let epub_type_data = required_data_pattern!(epub_type);
    let matter_data = required_data_pattern!(matter);
    let default_toc_format_data = data
        .iter()
        .map(|(role, vals)| {
            let role = Ident::new(&role, Span::call_site());
            (role, vals.default_toc_format.clone())
        })
        .map(|(role, val)| (role, val.to_representation()))
        .map(|(role, val)| quote! {#role => #val});
    let include_stylesheet_data = required_data_pattern!(include_stylesheet);
    let additional_head_data = optional_data_pattern!(additional_head);
    let section_wrapper_data = optional_data_pattern!(section_wrapper);
    let default_toc_level_data = optional_data_pattern!(default_toc_level);

    let header_level_func = quote! {
        const fn get_header_level(role: SemanticRole) -> Option<usize> {
            use SemanticRole::*;
            match role {
                #(#header_level_data),*
            }
        }
    };
    let header_classes_func = quote! {
        const fn get_header_classes(role: SemanticRole) -> Option<&'static str> {
            use SemanticRole::*;
            match role {
                #(#header_classes_data),*
            }
        }
    };
    let section_classes_func = quote! {
        const fn get_section_classes(role: SemanticRole) -> Option<&'static str> {
            use SemanticRole::*;
            match role {
                #(#section_classes_data),*

            }
        }
    };
    let epub_type_func = quote! {
        const fn get_epub_type(role: SemanticRole) -> &'static str {
            use SemanticRole::*;
            match role {
                #(#epub_type_data),*
            }
        }
    };
    let matter_func = quote! {
        const fn get_matter(role: SemanticRole) -> &'static str {
            use SemanticRole::*;
            match role {
                #(#matter_data),*
            }
        }
    };
    let default_toc_format_func = quote! {
        const fn get_default_toc_format(role: SemanticRole) -> TocFormat {
            use SemanticRole::*;
            match role {
                #(#default_toc_format_data),*
            }
        }
    };
    let include_stylesheet_func = quote! {
        const fn get_include_stylesheet(role: SemanticRole) -> bool {
            use SemanticRole::*;
            match role {
                #(#include_stylesheet_data),*
            }
        }
    };
    let additional_head_func = quote! {
        const fn get_additional_head(role: SemanticRole) -> Option<&'static str> {
            use SemanticRole::*;
            match role {
                #(#additional_head_data),*
            }
        }
    };
    let section_wrapper_func = quote! {
        const fn get_section_wrapper_div_classes(role: SemanticRole) -> Option<&'static str> {
            use SemanticRole::*;
            match role {
                #(#section_wrapper_data),*
            }
        }
    };

    let default_toc_level_func = quote! {
        const fn get_default_toc_level(role: SemanticRole) -> Option<usize> {
            use SemanticRole::*;
            match role {
                #(#default_toc_level_data),*
            }
        }
    };

    let semantic_role_const_fns = quote! {
        #header_level_func
        #header_classes_func
        #section_classes_func
        #section_wrapper_func
        #epub_type_func
        #matter_func
        #include_stylesheet_func
        #additional_head_func
        #default_toc_format_func
        #default_toc_level_func
    };

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("semantic_role_const_fns.rs");
    fs::write(&dest_path, semantic_role_const_fns.to_string()).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data.toml");
}
