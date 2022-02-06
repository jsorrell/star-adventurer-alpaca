extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate rocket;
extern crate syn;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn alpaca_request_data(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_struct = syn::parse_macro_input!(input as syn::ItemStruct);
    let _ = parse_macro_input!(args as syn::parse::Nothing);

    if let syn::Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote::quote! {
                    #[field(name = "ClientID")]
                    pub client_id: u32
                })
                .unwrap(),
        );

        fields.named.push(
            syn::Field::parse_named
                .parse2(quote::quote! {
                    #[field(name = "ClientTransactionID")]
                    pub client_transaction_id: u32
                })
                .unwrap(),
        );
    }

    return quote::quote! {
        #item_struct
    }
    .into();
}

fn get_named_arg_type(arg: syn::FnArg) -> syn::Type {
    match arg {
        syn::FnArg::Typed(a) => *a.ty.to_owned(),
        _ => panic!("Must be typed"),
    }
}

#[proc_macro_attribute]
pub fn alpaca_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let syn::ItemFn {
        attrs,
        vis,
        mut sig,
        block,
    } = syn::parse_macro_input!(input as syn::ItemFn);
    let _ = parse_macro_input!(args as syn::parse::Nothing);

    let stmts = &block.stmts;

    let mut new_sig = sig.clone();

    if sig.inputs.len() == 1 {
        let blank_data_arg = syn::parse2(quote! {
            _data: EmptyData
        })
        .unwrap();

        sig.inputs.insert(0, blank_data_arg)
    }

    // let data_ident = get_named_arg_ident(new_sig.inputs[0].to_owned());
    let data_type = get_named_arg_type(sig.inputs[0].to_owned());
    let state_type = match get_named_arg_type(sig.inputs[1].to_owned()) {
        syn::Type::Reference(p) => *p.elem,
        _ => panic!("State must be reference"),
    };
    // let state_ident = get_named_arg_ident(new_sig.inputs[1].to_owned());
    const WRAPPER_DATA_NAME: &str = "_data";
    let wrapper_data_ident = syn::Ident::new(WRAPPER_DATA_NAME, proc_macro2::Span::call_site());
    const WRAPPER_STATE_NAME: &str = "_state";
    let wrapper_state_ident = syn::Ident::new(WRAPPER_STATE_NAME, proc_macro2::Span::call_site());

    let result_type = match new_sig.output.to_owned() {
        syn::ReturnType::Type(_, t) => match *t {
            syn::Type::Path(tp) => match tp.path.segments.last().unwrap().arguments.to_owned() {
                syn::PathArguments::AngleBracketed(a) => a.args[0].to_owned(),
                _ => panic!("Wrong arguments"),
            },
            _ => panic!("Must be path"),
        },
        syn::ReturnType::Default => panic!("Must return a result"),
    };

    let fn_ident = sig.ident;
    let fn_name = fn_ident.to_string();
    let (req_type, path) = fn_name.split_once('_').unwrap();
    let new_fn_name = format!("_{}", fn_name);
    let new_fn_ident = syn::Ident::new(new_fn_name.as_str(), proc_macro2::Span::call_site());
    new_sig.ident = new_fn_ident.clone();

    let (rocket_attribute, wrapper_data_type, unwrapped_wrapper_data_ident) = match req_type {
        "get" => {
            let path = format!("/{}?<{}..>", path.replace("_", ""), WRAPPER_DATA_NAME);
            let wrapper_data_ident = wrapper_data_ident.clone();
            (
                quote! {
                    #[get(#path)]
                },
                data_type,
                quote! {
                    #wrapper_data_ident
                },
            )
        }
        "put" => {
            let path = format!("/{}", path.replace("_", ""),);
            let data_str = format!("<{}>", WRAPPER_DATA_NAME);
            let wrapper_data_ident = wrapper_data_ident.clone();
            (
                quote! {
                    #[put(#path, data = #data_str)]
                },
                syn::parse2(quote! {
                    rocket::form::Form<#data_type>
                })
                .unwrap(),
                quote! {
                    #wrapper_data_ident.clone()
                },
            )
        }
        _ => panic!("Must be prefixed with get_ or put_"),
    };

    let inner_fn_call = if new_sig.inputs.len() == 1 {
        quote! {
            #new_fn_ident(#wrapper_state_ident.inner())
        }
    } else {
        quote! {
            #new_fn_ident(#unwrapped_wrapper_data_ident, #wrapper_state_ident.inner())
        }
    };

    return quote! {
        #rocket_attribute
        #vis async fn #fn_ident(#wrapper_data_ident: #wrapper_data_type, #wrapper_state_ident: &State<#state_type>) -> rocket::serde::json::Json<response::AlpacaResponse<#result_type>> {
            let result = #inner_fn_call.await;
            rocket::serde::json::Json(response::AlpacaResponse::new(result, #wrapper_data_ident.client_transaction_id, &_state.sti))
        }

        #(#attrs)* #vis #new_sig {#(#stmts)*}
    }
        .into();
}
