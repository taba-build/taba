//! Taba's derives.
//!
//! [`Output`](macro@Output) describes what `#[derive(Serialize)]` writes as a TypeScript type,
//! and [`Input`](macro@Input) what `#[derive(Deserialize)]` reads. Both read serde's `#[serde(...)]`
//! attributes with serde's own parser, `serde_derive_internals`.
//!
//! # Attributes
//!
//! Every serde [container], [variant] and [field] attribute works as serde documents it, except
//! these, which fail to compile.
//!
//! Not supported yet:
//!
//! * generic types, lifetimes included
//! * an internally tagged enum with a newtype variant
//! * `untagged` on a variant of a tagged enum
//! * `flatten` on a field
//! * `remote`, `variant_identifier` and `field_identifier` on a type
//! * `with`, `serialize_with` and `deserialize_with` on a variant
//! * `skip_serializing_if` on a tuple field
//!
//! Never supported:
//!
//! * `skip` on a newtype's field (a newtype is its one value)
//!
//! `#[taba(with = "Type")]` names a type with the same JSON, for both directions. Its
//! `serialize_with` and `deserialize_with` keys do the same for [`Output`](macro@Output) or
//! [`Input`](macro@Input) alone.
//!
//! Use it on a field that has serde's `with`, `serialize_with` or `deserialize_with`, or whose type
//! is from another crate. Use it on a type whose serde impls you wrote by hand.
//!
//! [container]: https://serde.rs/container-attrs.html
//! [variant]: https://serde.rs/variant-attrs.html
//! [field]: https://serde.rs/field-attrs.html
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::iter;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{ToTokens, quote};
use serde_derive_internals::{
    Ctxt, Derive,
    ast::{Container, Data, Field, Style, Variant},
    attr::{self, Identifier, TagType},
    name::MultiName,
    replace_receiver,
};
use syn::{Attribute, DeriveInput, LitStr, ext::IdentExt, parse_macro_input};

/// Describes what `#[derive(Serialize)]` writes, from the same [attributes](crate#attributes).
#[proc_macro_derive(Output, attributes(serde, taba))]
pub fn output(input: TokenStream) -> TokenStream {
    derive(Direction::Output, parse_macro_input!(input))
}

/// Describes what `#[derive(Deserialize)]` reads, from the same [attributes](crate#attributes).
#[proc_macro_derive(Input, attributes(serde, taba))]
pub fn input(input: TokenStream) -> TokenStream {
    derive(Direction::Input, parse_macro_input!(input))
}

#[derive(Copy, Clone)]
enum Direction {
    Output,
    Input,
}

impl From<Direction> for Derive {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Output => Self::Serialize,
            Direction::Input => Self::Deserialize,
        }
    }
}

impl Direction {
    fn shape(self, ty: impl ToTokens) -> Tokens {
        let path = self.trait_path();
        quote!(<#ty as #path>::__SHAPE)
    }

    fn optional(self, ty: impl ToTokens) -> Option<Tokens> {
        match self {
            Direction::Output => None,
            Direction::Input => Some(quote!(<#ty as _taba::Input>::__OPTIONAL)),
        }
    }

    fn trait_path(self) -> Tokens {
        match self {
            Direction::Output => quote!(_taba::Output),
            Direction::Input => quote!(_taba::Input),
        }
    }

    fn with_key(self) -> &'static str {
        match self {
            Direction::Output => "serialize_with",
            Direction::Input => "deserialize_with",
        }
    }

    fn name(self, name: &MultiName) -> String {
        match self {
            Direction::Output => name.serialize_name().value.clone(),
            Direction::Input => name.deserialize_name().value.clone(),
        }
    }

    fn converted(self, container: &attr::Container) -> Option<&syn::Type> {
        match self {
            Direction::Output => container.type_into(),
            Direction::Input => container.type_from().or(container.type_try_from()),
        }
    }

    fn skips(self, field: &attr::Field) -> bool {
        match self {
            Direction::Output => field.skip_serializing(),
            Direction::Input => field.skip_deserializing(),
        }
    }

    fn skips_variant(self, variant: &attr::Variant) -> bool {
        match self {
            Direction::Output => variant.skip_serializing(),
            Direction::Input => variant.skip_deserializing(),
        }
    }

    /// Whether `skip_serializing_if` may leave the field out.
    fn skips_if(self, field: &attr::Field) -> bool {
        match self {
            Direction::Output => field.skip_serializing_if().is_some(),
            Direction::Input => false,
        }
    }

    /// Whether serde's `default` fills in the field when its key is missing.
    fn defaults(self, container: &attr::Container, field: &attr::Field) -> bool {
        match self {
            Direction::Output => false,
            Direction::Input => !field.default().is_none() || !container.default().is_none(),
        }
    }

    /// Whether serde promises to reject keys the type doesn't declare.
    fn closed(self, container: &attr::Container) -> bool {
        match self {
            Direction::Output => false,
            Direction::Input => container.deny_unknown_fields(),
        }
    }

    /// The `tag` key serde leads a named struct's object with, when writing.
    fn struct_tag(self, container: &attr::Container) -> Option<&str> {
        let TagType::Internal { tag } = container.tag() else { return None };
        match self {
            Direction::Output => Some(tag),
            Direction::Input => None,
        }
    }

    fn hides(self, field: &attr::Field) -> bool {
        match self {
            Direction::Output => field.serialize_with().is_some(),
            Direction::Input => field.deserialize_with().is_some(),
        }
    }

    fn hides_variant(self, variant: &attr::Variant) -> bool {
        match self {
            Direction::Output => variant.serialize_with().is_some(),
            Direction::Input => variant.deserialize_with().is_some(),
        }
    }
}

fn derive(direction: Direction, mut input: DeriveInput) -> TokenStream {
    // serde's parser requires its private module (serde_derive_internals)
    let private = syn::Ident::new("__private", Span::call_site());
    replace_receiver(&mut input);
    let serde = Ctxt::new();
    let container = Container::from_ast(&serde, &input, Derive::from(direction), &private);
    if let Err(error) = serde.check() {
        return error.to_compile_error().into();
    }
    let container = container.expect("serde's parser reports an error whenever it returns none");

    let ctxt = Ctxt::new();
    let expanded = Expander { ctxt: &ctxt, direction, container: &container }.expand();
    match ctxt.check() {
        Ok(()) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

struct Expander<'a> {
    ctxt: &'a Ctxt,
    direction: Direction,
    container: &'a Container<'a>,
}

/// Where a type's JSON comes from.
enum Source<'a> {
    /// A type `#[taba(...)]` names, on the type or a transparent type's field.
    StandIn(Box<syn::Type>),

    /// serde's `into`, `from` or `try_from` type, or a transparent type's field.
    Through(&'a syn::Type),

    /// Its own fields or variants.
    Own(&'a Data<'a>),
}

impl Expander<'_> {
    fn expand(&self) -> Tokens {
        self.refuse_unsupported();
        self.check_taba();

        let container = self.container;
        let source = self.source();
        let ident = &container.ident;
        let name = ident.unraw().to_string();
        let docs = docs(&container.original.attrs);
        let shape = self.shape(&source);
        let path = self.direction.trait_path();
        let optional = self.optional(&source);
        quote! {
            const _: () = {
                use ::taba::__private::typescript as _taba;

                static NAMED: _taba::Named = _taba::Named {
                    module_path: ::core::module_path!(),
                    name: #name,
                    docs: #docs,
                    file: ::core::file!(),
                    line: ::core::line!(),
                    shape: #shape,
                };

                impl #path for #ident {
                    const __SHAPE: _taba::Shape = _taba::Shape::Reference(|| &NAMED);
                    #optional
                }
            };
        }
    }

    /// Refuses a type the derive can't describe yet.
    fn refuse_unsupported(&self) {
        let container = self.container;
        let original = container.original;
        if !container.generics.params.is_empty() {
            let message = "generic types and lifetimes aren't supported yet";
            self.ctxt.error_spanned_by(container.generics, message);
        }
        if container.attrs.remote().is_some() {
            self.ctxt.error_spanned_by(original, "`remote` isn't supported yet");
        }
        if !matches!(container.attrs.identifier(), Identifier::No) {
            self.ctxt.error_spanned_by(original, "identifier enums aren't supported yet");
        }
        let tagged = !matches!(container.attrs.tag(), TagType::None);
        if tagged && let Data::Enum(variants) = &container.data {
            for variant in variants.iter().filter(|variant| variant.attrs.untagged()) {
                let message = "`untagged` on a variant of a tagged enum isn't supported yet";
                self.ctxt.error_spanned_by(variant.original, message);
            }
        }
    }

    fn source(&self) -> Source<'_> {
        let container = self.container;
        if let Some(ty) = self.stand_in(&container.original.attrs) {
            return Source::StandIn(Box::new(ty));
        }
        if let Some(ty) = self.direction.converted(&container.attrs) {
            return Source::Through(ty);
        }
        if !container.attrs.transparent() {
            return Source::Own(&container.data);
        }
        let field = container.data.all_fields().find(|field| field.attrs.transparent());
        let field = field.expect("serde marks a transparent type's field");
        match self.field_stand_in(field) {
            Some(ty) => Source::StandIn(Box::new(ty)),
            None => Source::Through(field.ty),
        }
    }

    fn shape(&self, source: &Source) -> Tokens {
        match source {
            Source::StandIn(ty) => self.direction.shape(ty),
            Source::Through(ty) => self.direction.shape(ty),
            Source::Own(Data::Enum(variants)) => self.variants(variants),
            Source::Own(Data::Struct(Style::Struct, fields)) => {
                self.struct_object(self.struct_tag(), fields)
            }
            Source::Own(Data::Struct(style, fields)) => self.fields(*style, fields),
        }
    }

    /// The type's `__OPTIONAL` item, if it has one.
    fn optional(&self, source: &Source) -> Option<Tokens> {
        let Source::Through(ty) = source else { return None };
        let optional = self.direction.optional(ty)?;
        Some(quote!(const __OPTIONAL: bool = #optional;))
    }

    fn struct_tag(&self) -> Option<Tokens> {
        let tag = self.direction.struct_tag(&self.container.attrs)?;
        let name = self.direction.name(self.container.attrs.name());
        Some(required(tag, literal(&name)))
    }

    /// An object of `fields`, led by `tag` when there is one.
    fn struct_object(&self, tag: Option<Tokens>, fields: &[Field]) -> Tokens {
        let closed = self.direction.closed(&self.container.attrs);
        let properties = self.present(fields).map(|field| self.property(field));
        object(closed, tag.into_iter().chain(properties))
    }

    /// The fields serde writes or reads, in order.
    fn present<'f>(&self, fields: &'f [Field<'f>]) -> impl Iterator<Item = &'f Field<'f>> {
        fields.iter().filter(|field| !self.direction.skips(&field.attrs))
    }

    fn fields(&self, style: Style, fields: &[Field]) -> Tokens {
        match style {
            Style::Struct => self.struct_object(None, fields),
            Style::Tuple => self.tuple(fields),
            Style::Newtype => self.newtype(&fields[0]),
            Style::Unit => quote!(_taba::Shape::Null),
        }
    }

    fn tuple(&self, fields: &[Field]) -> Tokens {
        let elements = self.present(fields).map(|field| self.element(field));
        quote!(_taba::Shape::Tuple(&[#(#elements),*]))
    }

    fn element(&self, field: &Field) -> Tokens {
        if self.direction.skips_if(&field.attrs) {
            // skipping a tuple element shifts later positions
            let message = "`skip_serializing_if` on a tuple field isn't supported";
            self.ctxt.error_spanned_by(field.original, message);
        }
        self.field_shape(field)
    }

    fn newtype(&self, field: &Field) -> Tokens {
        if self.direction.skips(&field.attrs) {
            // a newtype has only one field
            let message = "`skip` on a newtype's field isn't supported";
            self.ctxt.error_spanned_by(field.original, message);
        }
        self.field_shape(field)
    }

    fn property(&self, field: &Field) -> Tokens {
        if field.attrs.flatten() {
            self.ctxt.error_spanned_by(field.original, "`flatten` isn't supported yet");
        }
        let key = self.direction.name(field.attrs.name());
        let stand_in = self.field_stand_in(field);
        let optional = self.optional_key(field, stand_in.as_ref());
        let docs = docs(&field.original.attrs);
        let ty = stand_in.as_ref().unwrap_or(field.ty);
        let shape = self.direction.shape(ty);
        quote! {
            _taba::Property {
                key: #key,
                optional: #optional,
                docs: #docs,
                shape: #shape,
            }
        }
    }

    /// Whether serde lets the field's key be missing.
    fn optional_key(&self, field: &Field, stand_in: Option<&syn::Type>) -> Tokens {
        let skipped = self.direction.skips_if(&field.attrs);
        let defaulted = self.direction.defaults(&self.container.attrs, &field.attrs);
        if skipped || defaulted {
            return quote!(true);
        }
        if stand_in.is_some() {
            return quote!(false);
        }
        self.direction.optional(field.ty).unwrap_or_else(|| quote!(false))
    }

    fn field_shape(&self, field: &Field) -> Tokens {
        let stand_in = self.field_stand_in(field);
        self.direction.shape(stand_in.as_ref().unwrap_or(field.ty))
    }

    fn field_stand_in(&self, field: &Field) -> Option<syn::Type> {
        let stand_in = self.stand_in(&field.original.attrs);
        if stand_in.is_none() && self.direction.hides(&field.attrs) {
            let key = self.direction.with_key();
            let message = format!(
                "serde's `with` or `{key}` hides this field's JSON: name a type with the same JSON \
                 under the matching key, like `#[taba(with = \"String\")]`"
            );
            self.ctxt.error_spanned_by(field.original, message);
        }
        stand_in
    }

    /// Reports each `#[taba(...)]` that's malformed or on a variant, once.
    fn check_taba(&self) {
        let container = self.container;
        let field_attrs = container.data.all_fields().map(|field| &field.original.attrs);
        for attrs in iter::once(&container.original.attrs).chain(field_attrs) {
            if let Err(error) = self.parse_stand_in(attrs) {
                self.ctxt.syn_error(error);
            }
        }
        if let Data::Enum(variants) = &container.data {
            for variant in variants {
                if variant.original.attrs.iter().any(|attr| attr.path().is_ident("taba")) {
                    let message = "`#[taba(...)]` on a variant isn't supported";
                    self.ctxt.error_spanned_by(variant.original, message);
                }
            }
        }
    }

    /// The type `#[taba(...)]` names.
    ///
    /// Only its JSON is the same.
    fn stand_in(&self, attrs: &[Attribute]) -> Option<syn::Type> {
        self.parse_stand_in(attrs).ok().flatten()
    }

    fn parse_stand_in(&self, attrs: &[Attribute]) -> syn::Result<Option<syn::Type>> {
        let mut stand_in = None;
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("taba")) {
            attr.parse_nested_meta(|meta| {
                let keys = ["with", "serialize_with", "deserialize_with"];
                if !keys.iter().any(|key| meta.path.is_ident(key)) {
                    return Err(meta.error("unknown `taba` attribute"));
                }
                let ty: syn::Type = meta.value()?.parse::<LitStr>()?.parse()?;
                let applies =
                    meta.path.is_ident("with") || meta.path.is_ident(self.direction.with_key());
                if !applies {
                    return Ok(());
                }
                if stand_in.is_some() {
                    let key = meta.path.require_ident()?;
                    return Err(meta.error(format!("duplicate `taba` attribute `{key}`")));
                }
                stand_in = Some(ty);
                Ok(())
            })?;
        }
        Ok(stand_in)
    }

    fn variants(&self, variants: &[Variant]) -> Tokens {
        for variant in variants {
            if self.direction.hides_variant(&variant.attrs) {
                self.ctxt.error_spanned_by(variant.original, "`with` on a variant isn't supported");
            }
        }
        let present =
            variants.iter().filter(|variant| !self.direction.skips_variant(&variant.attrs));
        // refused in `refuse_unsupported`, this keeps them from a second error
        let tagged = present.clone().filter(|variant| !variant.attrs.untagged());
        match self.container.attrs.tag() {
            TagType::None => {
                let choices = present.map(|variant| self.choice(variant));
                quote!(_taba::Shape::Choice(&[#(#choices),*]))
            }
            TagType::External => union(tagged.map(|variant| self.externally_tagged(variant))),
            TagType::Internal { tag } => {
                union(tagged.map(|variant| self.internally_tagged(tag, variant)))
            }
            TagType::Adjacent { tag, content } => {
                union(tagged.map(|variant| self.adjacently_tagged(tag, content, variant)))
            }
        }
    }

    fn choice(&self, variant: &Variant) -> Tokens {
        let name = variant.ident.unraw().to_string();
        let shape = self.untagged(variant);
        quote!(_taba::Variant { name: #name, shape: #shape })
    }

    /// The variant's JSON without a tag.
    fn untagged(&self, variant: &Variant) -> Tokens {
        self.fields(variant.style, &variant.fields)
    }

    fn externally_tagged(&self, variant: &Variant) -> Tokens {
        let closed = self.direction.closed(&self.container.attrs);
        let name = self.direction.name(variant.attrs.name());
        if matches!(variant.style, Style::Unit) {
            return literal(&name);
        }
        object(closed, [required(&name, self.untagged(variant))])
    }

    fn internally_tagged(&self, tag: &str, variant: &Variant) -> Tokens {
        let name = self.direction.name(variant.attrs.name());
        let tag = required(tag, literal(&name));
        match variant.style {
            // serde takes extra keys here, with `deny_unknown_fields` or without
            Style::Unit => object(false, [tag]),
            Style::Struct => self.struct_object(Some(tag), &variant.fields),
            // let newtype refuse it, skip is never supported
            Style::Newtype if self.direction.skips(&variant.fields[0].attrs) => {
                self.newtype(&variant.fields[0])
            }
            Style::Newtype => {
                let message = "internally tagged newtype variants aren't supported yet";
                self.ctxt.error_spanned_by(variant.original, message);
                quote!(_taba::Shape::Never)
            }
            Style::Tuple => unreachable!("serde refuses tuple variants under internal tagging"),
        }
    }

    fn adjacently_tagged(&self, tag: &str, content: &str, variant: &Variant) -> Tokens {
        let closed = self.direction.closed(&self.container.attrs);
        let name = self.direction.name(variant.attrs.name());
        let tag = required(tag, literal(&name));
        if matches!(variant.style, Style::Unit) {
            return object(closed, [tag]);
        }
        let value = required(content, self.untagged(variant));
        object(closed, [tag, value])
    }
}

fn literal(name: &str) -> Tokens {
    quote!(_taba::Shape::Literal(#name))
}

fn required(key: &str, shape: Tokens) -> Tokens {
    quote!(_taba::Property { key: #key, optional: false, docs: &[], shape: #shape })
}

fn object(closed: bool, properties: impl IntoIterator<Item = Tokens>) -> Tokens {
    let properties = properties.into_iter();
    quote! {
        _taba::Shape::Object(&_taba::Object {
            properties: &[#(#properties),*],
            closed: #closed,
        })
    }
}

fn union(shapes: impl IntoIterator<Item = Tokens>) -> Tokens {
    let shapes = shapes.into_iter();
    quote!(_taba::Shape::Union(&[#(#shapes),*]))
}

/// The `#[doc]` values on a type or field, as written.
fn docs(attrs: &[Attribute]) -> Tokens {
    let metas = attrs.iter().filter_map(|attr| attr.meta.require_name_value().ok());
    let values = metas.filter(|meta| meta.path.is_ident("doc")).map(|meta| &meta.value);
    quote!(&[#(#values),*])
}
