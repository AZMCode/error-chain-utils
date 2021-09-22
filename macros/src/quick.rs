//! This module implements the `error_chain_quick!` macro for `error-chain-utils`
//! See the full documentation there

use std::fmt;
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use syn::{parse::{Parse, ParseStream, ParseBuffer}, parse2};
use quote::{quote,ToTokens};

trait TryParse {
    fn try_parse<T: Parse>(&self) -> syn::Result<T>;
}

impl TryParse for ParseBuffer<'_> {
    fn try_parse<T: Parse>(&self) -> syn::Result<T> {
        match self.fork().parse::<T>() {
            Ok(_) => Ok(self.parse::<T>()?),
            Err(e) => Err(e)
        }
    }
}

trait ProcessQuickError<T> {
    fn process_quick_error(self) -> T;
}

mod errors_child_element {
    use std::fmt;
    use syn::{LitStr, parenthesized, parse::{Parse, ParseStream, ParseBuffer}, token, punctuated};
    use proc_macro2::{Delimiter, Group, Ident, TokenStream, TokenTree};
    use quote::{ToTokens, quote};
    use crate::quick::{ProcessQuickError,TryParse};

    #[derive(Debug)]
    pub struct NormalError {
        ident: Ident,
        args: Option<Group>,
        body: Group
    }

    impl Parse for NormalError {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let ident = input.try_parse::<Ident>()?;
            let first_group = input.try_parse::<Group>()?;
            if first_group.delimiter() == Delimiter::Parenthesis {
                let second_group = input.try_parse::<Group>()?;
                if second_group.delimiter() == Delimiter::Brace {
                    Ok(NormalError {
                        ident,
                        args: Some(first_group),
                        body: second_group
                    })
                } else {
                    Err(syn::Error::new(second_group.span_open(),"Unexpected Delimiter Here"))
                }
            } else if first_group.delimiter() == Delimiter::Brace {
                Ok(NormalError {
                    ident,
                    args: None,
                    body: first_group
                })
            } else {
                Err(syn::Error::new(first_group.span_open(), "Unexpected delimiter here"))
            }
        }
    }

    impl ToTokens for NormalError {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.extend_one(TokenTree::from(self.ident.clone()));
            match self.args.clone() {
                Some(val) => tokens.extend_one(TokenTree::from(val)),
                _ => ()
            };
            tokens.extend_one(TokenTree::from(self.body.clone()));
        }
    }

    pub struct QuickError {
        err_ident: Ident,
        desc: LitStr,
        inner_args: punctuated::Punctuated<Ident,token::Comma>
    }

    impl fmt::Debug for QuickError {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(),fmt::Error> {

            struct LitStrDebug<'a> {
                inner: &'a LitStr
            }

            impl fmt::Debug for LitStrDebug<'_> {
                fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(),fmt::Error> {
                    fmt.write_str("LitStr( ")?;
                    self.inner.value().fmt(fmt)?;
                    fmt.write_str(" )")?;
                    Ok(())
                }
            }

            struct PunctuatedDebug<'a,T,U> {
                inner: &'a punctuated::Punctuated<T,U>
            }
    
            impl<T: fmt::Debug,U> fmt::Debug for PunctuatedDebug<'_,T,U> {
                fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(),fmt::Error> {
                    fmt.write_str("Punctuated( ")?;
                    self.inner.iter().collect::<Vec<&T>>().fmt(fmt)?;
                    fmt.write_str(" )")?;
                    Ok(())
                }
            }
    
            #[derive(Debug)]
            struct QuickErrorDebug<'a> {
                err_ident: &'a Ident,
                desc: LitStrDebug<'a>,
                inner_args: PunctuatedDebug<'a,Ident,token::Comma>
            }
    
            (QuickErrorDebug { 
                err_ident: &self.err_ident,
                desc: LitStrDebug { inner: &self.desc },
                inner_args: PunctuatedDebug {
                    inner: &self.inner_args
                }
            }).fmt(fmt)
        }

    }

    fn parse_parens(input: ParseStream) -> syn::Result<ParseBuffer> {
        let contents;
        parenthesized!(contents in input);
        Ok(contents)
    }

    fn try_parse_parens(input: ParseStream) -> syn::Result<ParseBuffer> {
        match parse_parens(&mut input.fork()) {
            Ok(_) => Ok(parse_parens(input).unwrap()),
            Err(e) => Err(e.clone()) 
        }
    }

    impl Parse for QuickError {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let ident = input.try_parse::<Ident>()?;
            (ident.to_string() == "quick")
                .then_some(())
                .ok_or(syn::Error::new(ident.span(),"Ident was not 'quick'"))?;
            input.try_parse::<token::Bang>()?;
            let late_fail: Result<QuickError,syn::Error> = try {
                let args = &mut try_parse_parens(input)?;
                let err_ident = args.try_parse::<Ident>()?;
                args.try_parse::<token::Comma>()?;
                let desc = args.try_parse::<LitStr>()?;
                let mut invalid_inner_args = false;
                let optional_paren: syn::Result<punctuated::Punctuated<Ident,token::Comma>> = try{
                    args.try_parse::<token::Comma>()?;
                    let inner_args_unparsed = &mut try_parse_parens(args)?;
                    match punctuated::Punctuated::parse_terminated(&inner_args_unparsed) {
                        Ok(val) => Ok(val),
                        Err(e) => {
                            invalid_inner_args = true;
                            Err(e)
                        }
                    }?
                };
                let inner_args = match optional_paren {
                    Ok(val) => Ok(val),
                    Err(_) => {
                        if invalid_inner_args {
                            Err(syn::Error::new(args.span(),"INV_QUICK"))
                        } else {
                            Ok(punctuated::Punctuated::new())
                        }
                    }
                };

                match args.try_parse::<token::Comma>() { _ => ()};
                args.is_empty().then_some(()).ok_or(syn::Error::new(args.span(),"INV_QUICK"))?;

                QuickError {
                    err_ident,
                    desc,
                    inner_args: inner_args?
                }
            };
            late_fail.or_else(|i:syn::Error| {Err(syn::Error::new(i.span(),"INV_QUICK"))})
            
        }
    }

    impl ProcessQuickError<NormalError> for QuickError {
        fn process_quick_error(self) -> NormalError {
            let ident = self.err_ident;
            let mut args_token_stream = TokenStream::new();
            let mut first_arg = true;
            for arg in self.inner_args.clone() {
                if first_arg {
                    first_arg = false;
                } else {
                    args_token_stream.extend(quote!(, ));
                }
                args_token_stream.extend_one(TokenTree::from(arg));
                args_token_stream.extend(quote!( : String));
            }
            let args;
            let are_args_empty = args_token_stream.is_empty();
            if are_args_empty {
                args = None;
            } else {
                args = Some(Group::new(Delimiter::Parenthesis,args_token_stream));
            }

            let mut body_token_stream = TokenStream::new();
            body_token_stream.extend(quote!(description));
            body_token_stream.extend_one(TokenTree::from(Group::new(Delimiter::Parenthesis,self.desc.to_token_stream())));
            body_token_stream.extend(quote!(display));
            
            let mut display_args_token_stream = TokenStream::new();
            if are_args_empty {
                display_args_token_stream.extend(self.desc.to_token_stream());
            } else {
                let mut format_str: String = self.desc.value();
                format_str += ":";
                for _ in self.inner_args.clone() {
                    format_str += " {},"
                }
                format_str.pop();
                display_args_token_stream.extend_one(LitStr::new(format_str.as_str(),self.desc.span()).into_token_stream());
            }

            for arg in self.inner_args {
                display_args_token_stream.extend(quote!(,));
                display_args_token_stream.extend_one(TokenTree::from(arg));
            }

            body_token_stream.extend_one(TokenTree::from(Group::new(Delimiter::Parenthesis,display_args_token_stream)));
            let body = Group::new(Delimiter::Brace,body_token_stream);
            NormalError { ident, args, body }
        }
    }
}

#[derive(Debug)]
enum ErrorsChildElementEnum {
    QuickError(errors_child_element::QuickError),
    NormalError(errors_child_element::NormalError)
}

impl Parse for ErrorsChildElementEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match input.try_parse() as syn::Result<errors_child_element::QuickError> {
            Ok(val) => return Ok(Self::QuickError(val)),
            Err(e) => {
                if e.to_string() == "INV_QUICK" {
                    return Err(e)
                } else {
                    ()
                }
            }
        };
        match input.try_parse() as syn::Result<errors_child_element::NormalError> {
            Ok(val) => return Ok(Self::NormalError(val)),
            Err(_) => ()
        };
        Err(input.error("Could not parse as ErrorsChildElementEnum"))
    }
}

impl ProcessQuickError<ErrorsChildElementEnum> for ErrorsChildElementEnum {
    fn process_quick_error(self) -> ErrorsChildElementEnum {
        match self {
            Self::QuickError(val) => Self::NormalError(val.process_quick_error()),
            _ => self
        }
    }
}

impl ToTokens for ErrorsChildElementEnum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::NormalError(ref val) => val.to_tokens(tokens),
            Self::QuickError(ref _val) => panic!("Not all QuickError structs were converted to NormalError ones")
        }
    }
}

mod root_element {
    use quote::ToTokens;
    use syn::{braced, parse::{Parse, ParseStream}};
    use proc_macro2::{Delimiter, Group, Ident, TokenStream, TokenTree};
    use crate::quick::{ErrorsChildElementEnum, ProcessQuickError, TryParse};

    #[derive(Debug)]
    pub struct ErrorsIdGroup {
        ident: Ident,
        items: Vec<ErrorsChildElementEnum>
    }
    
    impl Parse for ErrorsIdGroup {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let ident = input.try_parse::<Ident>()?;
            if ident.to_string() == "errors" {
                let errors;
                braced!(errors in input);
                let mut items = vec![];
                if errors.is_empty() {
                    Err(errors.error("Unexpected end of input"))
                } else {
                    while !errors.is_empty() {
                        items.push(errors.try_parse::<ErrorsChildElementEnum>()?);
                    }
                    Ok(ErrorsIdGroup {
                        ident,
                        items
                    })
                }
            } else {
                Err(syn::Error::new(ident.span(),"Expected 'errors'"))
            }
        }
    }

    impl ProcessQuickError<ErrorsIdGroup> for ErrorsIdGroup {
        fn process_quick_error(self) -> ErrorsIdGroup {
            let mut new_items = vec![];
            for item in self.items {
                new_items.push(item.process_quick_error());
            }
            ErrorsIdGroup {
                ident: self.ident,
                items: new_items
            }
        }
    }

    impl ToTokens for ErrorsIdGroup {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            tokens.extend_one(TokenTree::from(self.ident.clone()));
            let mut group_token_stream = TokenStream::new();
            for item in &self.items {
                item.to_tokens(&mut group_token_stream);
            }
            tokens.extend_one(TokenTree::from(Group::new(Delimiter::Brace,group_token_stream)));
        }
    }

    #[derive(Debug)]
    pub struct OtherIdGroup {
        ident: Ident,
        body: Option<Group>
    }
    
    impl Parse for OtherIdGroup {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let ident = input.try_parse::<Ident>()?;
            let body = input.try_parse() as syn::Result<Group>;
            match body {
                Ok(val) => Ok(OtherIdGroup {
                    ident,
                    body: Some(val)
                }),
                Err(_) => Ok(OtherIdGroup {
                    ident,
                    body: None
                })
            }
        }
    }

    impl ToTokens for OtherIdGroup {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.extend_one(TokenTree::from(self.ident.clone()));
            match self.body.clone() {
                Some(val) => tokens.extend_one(TokenTree::from(val)),
                None => ()
            };
        }
    }
}

#[derive(Debug)]
enum RootElementEnum {
    ErrorsIdGroup(root_element::ErrorsIdGroup),
    OtherIdGroup(root_element::OtherIdGroup)
}

impl Parse for RootElementEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match input.try_parse() as syn::Result<root_element::ErrorsIdGroup> {
            Ok(val) => Ok(RootElementEnum::ErrorsIdGroup(val)),
            Err(e) => {
                if e.to_string() == "INV_QUICK" {
                    return Err(e);
                }
                match input.try_parse() as syn::Result<root_element::OtherIdGroup> {
                    Ok(val) => Ok(RootElementEnum::OtherIdGroup(val)),
                    Err(e) => Err(e)
                }
            }
        }
    }
}

impl ProcessQuickError<RootElementEnum> for RootElementEnum {
    fn process_quick_error(self) -> RootElementEnum {
        match self {
            Self::ErrorsIdGroup(val) => Self::ErrorsIdGroup(val.process_quick_error()),
            _ => self
        }
    }
}

impl ToTokens for RootElementEnum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::ErrorsIdGroup(ref val) => val.to_tokens(tokens),
            Self::OtherIdGroup(ref val)  => val.to_tokens(tokens)
        }
    }
}

struct RootElementVec {
    items: Vec<RootElementEnum>
}

impl fmt::Debug for RootElementVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.items.fmt(f)
    }
}

impl Parse for RootElementVec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = vec![];
        if input.is_empty() {
            Err(input.error("Unexpected end of input"))
        } else {
            while !input.is_empty() {
                items.push(RootElementEnum::parse(input)?);
            }
            Ok(RootElementVec {
                items
            })
        }
    }
}

impl ProcessQuickError<RootElementVec> for RootElementVec {
    fn process_quick_error(self) -> RootElementVec {
        let mut new_items = vec![];
        for item in self.items {
            new_items.push(item.process_quick_error());
        }
        RootElementVec {
            items: new_items
        }
    }
}

impl ToTokens for RootElementVec {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut group_token_stream = TokenStream::new();
        for item in &self.items {
            item.to_tokens(&mut group_token_stream);
        }
        tokens.extend_one(TokenTree::from(Group::new(Delimiter::Brace,group_token_stream)));
    }
}



/// Main function for the [`error_chain_quick!`](../../error_chain_utils/macro.error_chain_quick.html) macro
pub fn main(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed_input: RootElementVec = match parse2(input) {
        Ok(val) => val,
        Err(e) => {
            if e.to_string() == "INV_QUICK" {
                let mut new_e = syn::Error::new(e.span(),"Invalid 'quick!()' macro");
                new_e.combine(e);
                return Err(new_e);
            } else {
                return Err(e);
            }
        }
    };
    let transformed_input: RootElementVec = parsed_input.process_quick_error();
    let mut output_stream: TokenStream = TokenStream::new();
    output_stream.extend(quote!(::error_chain::error_chain!));
    transformed_input.to_tokens(&mut output_stream);
    Ok(output_stream)
}


#[cfg(test)]
mod tests{
    use std::assert_eq;
    use quote::quote;
    use crate::quick;
    #[test]
    pub fn test() {
        let input = quote!{

            types {
                BuildError, BEKind, BETrait, BEResult;
            }
        
            errors {
                NormalError1 {
                    description("Error 1 Description: Without Arguments"),
                    display("Error 1 Display")
                }
                NormalError2 (arg1: String, arg2: String) {
                    description("Error 2 Description: With Arguments"),
                    display("Error 2 Display: {}, {}", arg1, arg2),
                }
                quick!(QuickError1, "Error 1 Description: Zero arguments")
                quick!(QuickError2, "Error 2 Description: One Argument",(arg1,))
                quick!(QuickError3, "Error 3 Description: Three Arguments",(arg1,arg2,arg3,))
                quick!(QuickError4, "Error 4 Description: Zero arguments, trailing comma",)
            }
        };
        let output = quick::main(input).unwrap();
        let expected_output = quote!{
            ::error_chain::error_chain!{
                types {
                    BuildError, BEKind, BETrait, BEResult;
                }
                
                errors {
                    NormalError1 {
                        description("Error 1 Description: Without Arguments"),
                        display("Error 1 Display")
                    }
                    NormalError2 (arg1: String , arg2: String){
                        description("Error 2 Description: With Arguments"),
                        display("Error 2 Display: {}, {}", arg1, arg2),
                    }
                    QuickError1 {
                        description("Error 1 Description: Zero arguments")
                        display ("Error 1 Description: Zero arguments")
                    }
                    QuickError2 (arg1: String){
                        description("Error 2 Description: One Argument")
                        display("Error 2 Description: One Argument: {}", arg1)
                    }
                    QuickError3 (arg1: String, arg2: String, arg3: String){
                        description("Error 3 Description: Three Arguments")
                        display("Error 3 Description: Three Arguments: {}, {}, {}", arg1, arg2, arg3)
                    }
                    QuickError4 {
                        description("Error 4 Description: Zero arguments, trailing comma")
                        display ("Error 4 Description: Zero arguments, trailing comma")
                    }
                }
            }
        };
        assert_eq!(output.to_string(),expected_output.to_string(),"Actual output and Expected output did not match.\n Expected Output: \n{:#?}\n Actual Output: \n{:#?}\n",expected_output,output);
    }
}