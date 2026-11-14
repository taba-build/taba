//! TypeScript declarations for Rust types.

use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fmt::{self, Write},
    iter,
};

use taba::{
    __private::typescript::{Direction, Named, Object, Shape},
    Input, Output,
};
use taba_rolldown::oxc::{
    allocator::{Allocator, GetAllocator, Vec as ArenaVec},
    ast::{
        Comment, CommentContent, CommentKind, CommentPosition,
        ast::{
            BindingIdentifier, CommentNewlines, Declaration, IdentifierName,
            ImportDeclarationSpecifier, ImportOrExportKind, Program, PropertyKey, Statement,
            StringLiteral, TSIndexSignatureName, TSLiteral, TSNamespaceDeclarationBody,
            TSNamespaceDeclarationKind, TSSignature, TSTupleElement, TSType, TSTypeAnnotation,
            TSTypeName,
        },
        builder::AstBuilder,
    },
    codegen::Codegen,
    span::{SPAN, SourceType, Span},
    syntax::identifier::is_identifier_name_patched,
};

/// The crate who represents `crate`.
type CrateCrate = &'static str;

/// A TypeScript module declaring Rust types.
///
/// `Foo` is what `serde_json` writes for `Foo`, and `Foo.Input` is what it reads as `Foo`.
#[derive(Debug)]
pub struct Declarations {
    crate_: CrateCrate,
    types: BTreeSet<Declared>,
}

#[derive(Debug)]
struct Declared(&'static Named);

impl Ord for Declared {
    fn cmp(&self, other: &Self) -> Ordering {
        let module_path = self.0.module_path.cmp(other.0.module_path);
        let name = self.0.name.cmp(other.0.name);
        let direction = self.0.direction.cmp(&other.0.direction);
        module_path.then(name).then(direction)
    }
}

impl PartialEq for Declared {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Declared {}

impl PartialOrd for Declared {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Declarations {
    /// The file to write the module as, since it imports itself by that name.
    pub const FILE: &'static str = "taba.d.ts";

    /// Uses the name for the `crate` keyword.
    pub fn new(crate_: &'static str) -> Self {
        Self { crate_, types: BTreeSet::new() }
    }

    /// Declares what TypeScript needs to describe `T` as `serde_json` **writes** it.
    pub fn output<T: Output + ?Sized>(mut self) -> Self {
        self.add(&T::__SHAPE);
        self
    }

    /// Declares what TypeScript needs to describe `T` as `serde_json` **reads** it.
    pub fn input<T: Input>(mut self) -> Self {
        self.add(&T::__SHAPE);
        self
    }

    fn add(&mut self, shape: &Shape) {
        match shape {
            Shape::Reference(named) => {
                let named = named();

                // each type once, which also ends a recursive type's walk
                if self.types.insert(Declared(named)) {
                    self.add(&named.shape);
                }
            }
            Shape::Array(item) | Shape::Record(item) => self.add(item),
            Shape::Tuple(items) | Shape::Union(items) => self.add_all(*items),
            Shape::Object(object) => {
                let shapes = object.properties.iter().map(|property| &property.shape);
                self.add_all(shapes);
            }
            Shape::Choice(variants) => {
                let shapes = variants.iter().map(|variant| &variant.shape);
                self.add_all(shapes);
            }
            Shape::Boolean
            | Shape::Number
            | Shape::String
            | Shape::Null
            | Shape::Never
            | Shape::Literal(_) => {}
        }
    }

    fn add_all<'s>(&mut self, shapes: impl IntoIterator<Item = &'s Shape>) {
        for shape in shapes {
            self.add(shape);
        }
    }
}

impl fmt::Display for Declarations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let allocator = Allocator::default();
        let ast = AstBuilder::new(&allocator);
        let mut printer = Printer { ast: &ast, crate_: self.crate_, docs: Docs::default() };
        let import = printer.import_self();
        let types: Vec<_> = self.types.iter().map(|declared| declared.0).collect();
        let modules = types.chunk_by(|first, second| first.module_path == second.module_path);
        let modules = modules.filter_map(|types| printer.module(types));
        let body = ArenaVec::from_iter_in(iter::once(import).chain(modules), &ast);
        let source = allocator.alloc_str(&printer.docs.source);
        let program = Program::new(
            SPAN,
            SourceType::d_ts(),
            source,
            ArenaVec::from_iter_in(printer.docs.comments, &ast),
            None,
            [],
            body,
            &ast,
        );
        f.write_str(&Codegen::new().build(&program).code)
    }
}

/// The program's source text, written only to hold doc comments.
///
/// Oxc prints each comment from this text, before the node starting at its `attached_to`.
#[derive(Default)]
struct Docs {
    source: String,
    comments: Vec<Comment>,
}

impl Docs {
    fn span(&mut self, docs: &[&str]) -> Span {
        let text = text(docs);
        let kind = match &text {
            text if text.is_empty() => return SPAN,
            text if text.contains('\n') => CommentKind::MultiLineBlock,
            _ => CommentKind::SingleLineBlock,
        };
        let start = self.source.len() as u32;
        self.source.push_str(&jsdoc(&text).to_string());
        let end = self.source.len() as u32;
        self.comments.push(Comment {
            span: Span::new(start, end),
            attached_to: end,
            kind,
            position: CommentPosition::Leading,
            newlines: CommentNewlines::Trailing,
            content: CommentContent::Jsdoc,
        });
        Span::empty(end)
    }
}

/// A doc comment's text from its `#[doc]` values.
///
/// Only strip the first space, otherwise we mess with code block indentations.
fn text(docs: &[&str]) -> String {
    let lines: Vec<_> = docs.iter().map(|line| line.strip_prefix(' ').unwrap_or(line)).collect();
    lines.join("\n").trim().to_owned()
}

fn jsdoc(text: &str) -> impl fmt::Display {
    fmt::from_fn(move |f| {
        if !text.contains('\n') {
            return write!(f, "/** {} */", escaped(text));
        }
        f.write_str("/**\n")?;
        for line in text.lines() {
            match line {
                "" => f.write_str(" *\n")?,
                line => writeln!(f, " * {}", escaped(line))?,
            }
        }
        f.write_str(" */")
    })
}

/// `text` with a backslash inside each `*/`, which would end the comment.
fn escaped(text: &str) -> impl fmt::Display {
    fmt::from_fn(move |f| {
        let mut last = 0;
        for (index, _) in text.match_indices("*/") {
            f.write_str(&text[last..=index])?;
            f.write_char('\\')?;
            last = index + 1;
        }
        f.write_str(&text[last..])
    })
}

struct Printer<'a> {
    ast: &'a AstBuilder<'a>,
    crate_: CrateCrate,
    docs: Docs,
}

impl<'a> Printer<'a> {
    /// `import type * as $ from "./taba.js";`, which no Rust name can shadow.
    fn import_self(&self) -> Statement<'a> {
        let ast = self.ast;
        let stem = Declarations::FILE.trim_end_matches(".d.ts");
        let specifier = ast.allocator().alloc_str(&format!("./{stem}.js"));
        let local = BindingIdentifier::new(SPAN, "$", ast);
        let namespace =
            ImportDeclarationSpecifier::new_import_namespace_specifier(SPAN, local, ast);
        Statement::new_import_declaration(
            SPAN,
            Some(ArenaVec::from_array_in([namespace], ast)),
            StringLiteral::new(SPAN, specifier, None, ast),
            None,
            None,
            ImportOrExportKind::Type,
            ast,
        )
    }

    /// `export namespace crate.foo { ... }`, with a single Rust module's types.
    fn module(&mut self, types: &[&Named]) -> Option<Statement<'a>> {
        let path: Vec<_> = self.segments(types.first()?.module_path).collect();
        let (&name, inner) = path.split_first()?;
        let ast = self.ast;
        let declarations = types.iter().map(|named| match named.direction {
            Direction::Output => self.alias(named.name, named),
            Direction::Input => self.input(named),
        });
        let declarations = ArenaVec::from_iter_in(declarations, ast);
        Some(self.namespace(name, inner, declarations))
    }

    /// A module path's namespaces, with `self`'s named crate's as `crate`.
    fn segments(&self, module_path: &'static str) -> impl Iterator<Item = &'static str> {
        let mut segments = module_path.split("::");
        let root = segments.next().map(|first| if first == self.crate_ { "crate" } else { first });
        root.into_iter().chain(segments)
    }

    /// `export namespace Foo { export type Input = ...; }`
    fn input(&mut self, named: &Named) -> Statement<'a> {
        let input = self.alias("Input", named);
        self.namespace(named.name, &[], ArenaVec::from_array_in([input], self.ast))
    }

    /// `export type Foo = ...;`, with `named`'s docs and shape.
    fn alias(&mut self, name: &'a str, named: &Named) -> Statement<'a> {
        let ast = self.ast;
        let span = self.docs.span(named.docs);
        let shape = self.ts(&named.shape);
        let id = BindingIdentifier::new(SPAN, name, ast);
        let alias = Declaration::new_ts_type_alias_declaration(SPAN, id, None, shape, false, ast);
        Statement::new_export_declaration(span, alias, ast)
    }

    /// `export namespace foo.bar { ... }`, `name` holding one namespace per segment of `inner`.
    fn namespace(
        &self,
        name: &'a str,
        inner: &[&'a str],
        body: ArenaVec<'a, Statement<'a>>,
    ) -> Statement<'a> {
        let ast = self.ast;
        let kind = TSNamespaceDeclarationKind::Namespace;
        let block = TSNamespaceDeclarationBody::new_ts_module_block(SPAN, [], body, ast);
        let body = inner.iter().rev().fold(block, |body, &segment| {
            let id = BindingIdentifier::new(SPAN, segment, ast);
            TSNamespaceDeclarationBody::new_ts_namespace_declaration(
                SPAN, id, body, kind, false, ast,
            )
        });
        let id = BindingIdentifier::new(SPAN, name, ast);
        let namespace = Declaration::new_ts_namespace_declaration(SPAN, id, body, kind, false, ast);
        Statement::new_export_declaration(SPAN, namespace, ast)
    }

    fn ts(&mut self, shape: &Shape) -> TSType<'a> {
        let ast = self.ast;
        match shape {
            Shape::Boolean => TSType::new_ts_boolean_keyword(SPAN, ast),
            Shape::Number => TSType::new_ts_number_keyword(SPAN, ast),
            Shape::String => TSType::new_ts_string_keyword(SPAN, ast),
            Shape::Null => TSType::new_ts_null_keyword(SPAN, ast),
            Shape::Never => TSType::new_ts_never_keyword(SPAN, ast),
            Shape::Literal(text) => {
                let literal = TSLiteral::new_string_literal(SPAN, *text, None, ast);
                TSType::new_ts_literal_type(SPAN, literal, ast)
            }
            Shape::Array(item) => {
                let item = self.ts(item);
                TSType::new_ts_array_type(SPAN, item, ast)
            }
            Shape::Tuple(items) => {
                let items = items.iter().map(|item| TSTupleElement::from(self.ts(item)));
                TSType::new_ts_tuple_type(SPAN, ArenaVec::from_iter_in(items, ast), ast)
            }
            Shape::Record(value) => self.record(value),
            Shape::Object(object) => self.object(object),
            Shape::Union(members) => self.union(*members),
            Shape::Choice(variants) => self.union(variants.iter().map(|variant| &variant.shape)),
            Shape::Reference(named) => self.reference(named()),
        }
    }

    /// `{ [key: string]: ... }`
    fn record(&mut self, value: &Shape) -> TSType<'a> {
        let ast = self.ast;
        let string = TSType::new_ts_string_keyword(SPAN, ast);
        let string = TSTypeAnnotation::boxed(SPAN, string, ast);
        let key = TSIndexSignatureName::new(SPAN, "key", string, ast);
        let value = TSTypeAnnotation::boxed(SPAN, self.ts(value), ast);
        let index = TSSignature::new_ts_index_signature(SPAN, key, value, false, false, ast);
        TSType::new_ts_type_literal(SPAN, [index], ast)
    }

    fn object(&mut self, object: &Object) -> TSType<'a> {
        let ast = self.ast;
        let mut members = ArenaVec::new_in(ast);
        for property in object.properties {
            let span = self.docs.span(property.docs);
            let shape = self.ts(&property.shape);
            members.push(TSSignature::new_ts_property_signature(
                span,
                false,
                property.optional,
                false,
                self.key(property.key),
                Some(TSTypeAnnotation::boxed(SPAN, shape, ast)),
                ast,
            ));
        }
        TSType::new_ts_type_literal(SPAN, members, ast)
    }

    fn union<'s>(&mut self, shapes: impl IntoIterator<Item = &'s Shape>) -> TSType<'a> {
        let ast = self.ast;
        let members = shapes.into_iter().map(|shape| self.ts(shape));
        let mut members = ArenaVec::from_iter_in(members, ast);
        match members.len() {
            0 => TSType::new_ts_never_keyword(SPAN, ast),
            1 => members.remove(0),
            _ => TSType::new_ts_union_type(SPAN, members, ast),
        }
    }

    /// `$.crate.foo.Bar`, then `.Input` for an input.
    fn reference(&self, named: &Named) -> TSType<'a> {
        let ast = self.ast;
        let input = (named.direction == Direction::Input).then_some("Input");
        let path = self.segments(named.module_path).chain(iter::once(named.name)).chain(input);
        let root = TSTypeName::new_identifier_reference(SPAN, "$", ast);
        let name = path.fold(root, |left, segment| {
            let right = IdentifierName::new(SPAN, segment, ast);
            TSTypeName::new_qualified_name(SPAN, left, right, ast)
        });
        TSType::new_ts_type_reference(SPAN, name, None, ast)
    }

    fn key(&self, key: &'a str) -> PropertyKey<'a> {
        if is_identifier_name_patched(key) {
            PropertyKey::new_static_identifier(SPAN, key, self.ast)
        } else {
            PropertyKey::new_string_literal(SPAN, key, None, self.ast)
        }
    }
}
