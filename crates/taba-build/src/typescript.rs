//! TypeScript declarations for Rust types.

use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fmt::{self, Write},
    iter,
};

use taba::{
    __private::typescript::{Direction, Named, Object, Rest, Shape},
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
            TSTypeName, TSTypeParameter, TSTypeParameterDeclaration, TSTypeParameterInstantiation,
        },
        builder::AstBuilder,
    },
    codegen::Codegen,
    parser::Parser,
    span::{SPAN, SourceType, Span},
    syntax::identifier::is_identifier_name_patched,
};

#[cfg(feature = "test")]
pub(crate) mod cases;
pub(crate) mod client;

/// The crate who represents `crate`.
type CrateCrate = &'static str;

/// The alias an `Input` object with [parts](Rest::Parts) goes through.
const INPUT: &str = "$Input";

/// The member of an input's namespace that declares it, like `Foo.Input`.
const INPUT_MEMBER: &str = "Input";

/// The alias an `Output` object with [parts](Rest::Parts) goes through.
const OUTPUT: &str = "$Output";

/// The namespace a file imports the declared types as, like `$.crate.Foo`.
const TYPES: &str = "$";

/// A TypeScript module declaring Rust types.
///
/// `Foo` is what `serde_json` writes for `Foo`, and `Foo.Input` is what it reads as `Foo`.
///
/// Other crates are top-level namespaces, so one named for a TypeScript reserved word, like
/// `default`, can't be declared.
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

    /// [`output`](Self::output) and [`input`](Self::input).
    pub fn both<T: Output + Input>(self) -> Self {
        self.output::<T>().input::<T>()
    }

    fn add(&mut self, shape: &Shape) {
        match shape {
            Shape::Reference(named, arguments) => {
                let named = named();

                // each type once, which also ends a recursive type's walk
                if self.types.insert(Declared(named)) {
                    self.add(&named.shape);
                }
                self.add_all(*arguments);
            }
            Shape::Array(item) | Shape::Record(item) => self.add(item),
            Shape::Tuple(items) | Shape::Union(items) => self.add_all(*items),
            Shape::Object(object) => {
                let shapes = object.properties.iter().map(|property| &property.shape);
                self.add_all(shapes);
                if let Rest::Parts(parts) = object.rest {
                    self.add_all(parts);
                }
            }
            Shape::Boolean
            | Shape::Number
            | Shape::String
            | Shape::Null
            | Shape::Never
            | Shape::Literal(_)
            | Shape::Parameter(_) => {}
        }
    }

    fn add_all<'s>(&mut self, shapes: impl IntoIterator<Item = &'s Shape>) {
        for shape in shapes {
            self.add(shape);
        }
    }

    /// `export namespace crate.foo { ... }` for each module's types, in `printer`'s file.
    fn modules<'a>(&self, printer: &mut Printer<'a>) -> Vec<Statement<'a>> {
        let types: Vec<_> = self.types.iter().map(|declared| declared.0).collect();
        let modules = types.chunk_by(|first, second| first.module_path == second.module_path);
        modules.filter_map(|types| printer.module(types)).collect()
    }
}

impl fmt::Display for Declarations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let allocator = Allocator::default();
        let ast = AstBuilder::new(&allocator);
        let mut printer = Printer::new(&ast, self.crate_);
        let modules = self.modules(&mut printer);
        let import = printer.import_self();
        f.write_str(&Codegen::new().build(&printer.program([import], modules)).code)
    }
}

/// The aliases an object with [parts](Rest::Parts) goes through, from `prelude.d.ts`.
fn prelude(allocator: &Allocator) -> ArenaVec<'_, Statement<'_>> {
    Parser::new(allocator, include_str!("prelude.d.ts"), SourceType::d_ts()).parse().program.body
}

/// The program's source text, written only to hold its comments.
///
/// Oxc prints each comment from this text, before the node starting at its `attached_to`.
#[derive(Default)]
struct Comments {
    source: String,
    comments: Vec<Comment>,
}

impl Comments {
    /// The span of a node led by the doc comment `docs`.
    fn jsdoc(&mut self, docs: &[&str]) -> Span {
        let text = text(docs);
        let kind = match &text {
            text if text.is_empty() => return SPAN,
            text if text.contains('\n') => CommentKind::MultiLineBlock,
            _ => CommentKind::SingleLineBlock,
        };
        self.leading(jsdoc(&text), kind, CommentContent::Jsdoc)
    }

    /// The span of a node led by `// {text}`.
    fn line(&mut self, text: &str) -> Span {
        self.leading(format_args!("// {text}"), CommentKind::Line, CommentContent::None)
    }

    /// A program of `body`, with these comments.
    fn program<'a>(
        self,
        ast: &AstBuilder<'a>,
        source_type: SourceType,
        body: impl IntoIterator<Item = Statement<'a>>,
    ) -> Program<'a> {
        let source = ast.allocator().alloc_str(&self.source);
        let comments = ArenaVec::from_iter_in(self.comments, ast);
        let body = ArenaVec::from_iter_in(body, ast);
        Program::new(SPAN, source_type, source, comments, None, [], body, ast)
    }

    fn leading(
        &mut self,
        comment: impl fmt::Display,
        kind: CommentKind,
        content: CommentContent,
    ) -> Span {
        let start = self.source.len() as u32;
        self.source.push_str(&comment.to_string());
        let end = self.source.len() as u32;
        self.comments.push(Comment {
            span: Span::new(start, end),
            attached_to: end,
            kind,
            position: CommentPosition::Leading,
            newlines: CommentNewlines::Trailing,
            content,
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

/// A declarations file as it's built, and what every declaration in it shares.
struct Printer<'a> {
    ast: &'a AstBuilder<'a>,
    crate_: CrateCrate,
    comments: Comments,
    prelude: bool,
}

impl<'a> Printer<'a> {
    fn new(ast: &'a AstBuilder<'a>, crate_: CrateCrate) -> Self {
        Self { ast, crate_, comments: Comments::default(), prelude: false }
    }

    /// The finished file, holding `imports`, any prelude, then `body`.
    fn program(
        self,
        imports: impl IntoIterator<Item = Statement<'a>>,
        body: Vec<Statement<'a>>,
    ) -> Program<'a> {
        let prelude = self.prelude.then(|| prelude(self.ast.allocator()));
        let statements = imports.into_iter().chain(prelude.into_iter().flatten()).chain(body);
        self.comments.program(self.ast, SourceType::d_ts(), statements)
    }

    /// `import type * as $ from "./taba.js";`, the file importing itself.
    fn import_self(&self) -> Statement<'a> {
        let stem = Declarations::FILE.trim_end_matches(".d.ts");
        self.import_types(SPAN, &format!("./{stem}.js"))
    }

    /// `import type * as $ from "{specifier}";`, which no Rust name can shadow.
    fn import_types(&self, span: Span, specifier: &str) -> Statement<'a> {
        let ast = self.ast;
        let specifier = ast.allocator().alloc_str(specifier);
        let local = BindingIdentifier::new(SPAN, TYPES, ast);
        let namespace =
            ImportDeclarationSpecifier::new_import_namespace_specifier(SPAN, local, ast);
        Statement::new_import_declaration(
            span,
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
        let input = self.alias(INPUT_MEMBER, named);
        self.namespace(named.name, &[], ArenaVec::from_array_in([input], self.ast))
    }

    /// `export type Foo<T> = ...;`, with `named`'s docs, parameters and shape.
    fn alias(&mut self, name: &'a str, named: &Named) -> Statement<'a> {
        let ast = self.ast;
        let span = self.comments.jsdoc(named.docs);
        let shape = Declaring { file: self, named }.ts(&named.shape);
        let parameters = (!named.parameters.is_empty()).then(|| {
            let parameters = named.parameters.iter().map(|&parameter| {
                let name = BindingIdentifier::new(SPAN, parameter, ast);
                TSTypeParameter::new(SPAN, name, None, None, false, false, false, ast)
            });
            let parameters = ArenaVec::from_iter_in(parameters, ast);
            TSTypeParameterDeclaration::boxed(SPAN, parameters, ast)
        });
        let id = BindingIdentifier::new(SPAN, name, ast);
        let alias =
            Declaration::new_ts_type_alias_declaration(SPAN, id, parameters, shape, false, ast);
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
}

/// One type's declaration, whose parameters and direction decide how its shapes print.
struct Declaring<'p, 'a> {
    file: &'p mut Printer<'a>,
    named: &'p Named,
}

impl<'a> Declaring<'_, 'a> {
    fn ts(&mut self, shape: &Shape) -> TSType<'a> {
        let ast = self.file.ast;
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
            Shape::Tuple(items) => self.tuple(items),
            Shape::Record(value) => self.record(value),
            Shape::Object(object) => self.object(object),
            Shape::Union(members) => self.union(members),
            Shape::Parameter(index) => {
                let parameter = self.named.parameters[*index];
                let name = TSTypeName::new_identifier_reference(SPAN, parameter, ast);
                TSType::new_ts_type_reference(SPAN, name, None, ast)
            }
            Shape::Reference(named, arguments) => self.reference(named(), arguments),
        }
    }

    /// `{ [key: string]: ... }`
    fn record(&mut self, value: &Shape) -> TSType<'a> {
        let ast = self.file.ast;
        let string = TSType::new_ts_string_keyword(SPAN, ast);
        let string = TSTypeAnnotation::boxed(SPAN, string, ast);
        let key = TSIndexSignatureName::new(SPAN, "key", string, ast);
        let value = TSTypeAnnotation::boxed(SPAN, self.ts(value), ast);
        let index = TSSignature::new_ts_index_signature(SPAN, key, value, false, false, ast);
        TSType::new_ts_type_literal(SPAN, [index], ast)
    }

    /// `{ ... }`, merged with [parts](Rest::Parts) through [`INPUT`] or [`OUTPUT`].
    fn object(&mut self, object: &Object) -> TSType<'a> {
        let ast = self.file.ast;
        let mut members = ArenaVec::new_in(ast);
        for property in object.properties {
            let span = self.file.comments.jsdoc(property.docs);
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
        let literal = TSType::new_ts_type_literal(SPAN, members, ast);
        let Rest::Parts(parts) = object.rest else {
            return literal;
        };
        self.file.prelude = true;
        let parts = self.tuple(parts);
        let merge = match self.named.direction {
            Direction::Input => INPUT,
            Direction::Output => OUTPUT,
        };
        let name = TSTypeName::new_identifier_reference(SPAN, merge, ast);
        let arguments = ArenaVec::from_array_in([literal, parts], ast);
        let arguments = TSTypeParameterInstantiation::boxed(SPAN, arguments, ast);
        TSType::new_ts_type_reference(SPAN, name, Some(arguments), ast)
    }

    fn tuple(&mut self, items: &[Shape]) -> TSType<'a> {
        let ast = self.file.ast;
        let items = items.iter().map(|item| TSTupleElement::from(self.ts(item)));
        TSType::new_ts_tuple_type(SPAN, ArenaVec::from_iter_in(items, ast), ast)
    }

    fn union(&mut self, members: &[Shape]) -> TSType<'a> {
        let ast = self.file.ast;
        let mut members = flat(members);
        // so `A | $Output<{ kind: "B" }, [never]>` prints as `A`
        members.retain(|member| !is_never(member));
        // so `Option<Option<T>>` prints as `T | null`
        members.dedup_by(|a, b| matches!((a, b), (Shape::Null, Shape::Null)));
        let members = members.into_iter().map(|member| self.ts(member));
        let mut members = ArenaVec::from_iter_in(members, ast);
        match members.len() {
            0 => TSType::new_ts_never_keyword(SPAN, ast),
            1 => members.remove(0),
            _ => TSType::new_ts_union_type(SPAN, members, ast),
        }
    }

    /// `$.crate.foo.Bar`, then `.Input` for an input, then its arguments.
    fn reference(&mut self, referenced: &Named, arguments: &[Shape]) -> TSType<'a> {
        let ast = self.file.ast;
        let input = (referenced.direction == Direction::Input).then_some(INPUT_MEMBER);
        let segments = self.file.segments(referenced.module_path);
        let path = segments.chain(iter::once(referenced.name)).chain(input);
        let root = TSTypeName::new_identifier_reference(SPAN, TYPES, ast);
        let name = path.fold(root, |left, segment| {
            let right = IdentifierName::new(SPAN, segment, ast);
            TSTypeName::new_qualified_name(SPAN, left, right, ast)
        });
        let arguments = (!arguments.is_empty()).then(|| {
            let arguments = arguments.iter().map(|argument| self.ts(argument));
            let arguments = ArenaVec::from_iter_in(arguments, ast);
            TSTypeParameterInstantiation::boxed(SPAN, arguments, ast)
        });
        TSType::new_ts_type_reference(SPAN, name, arguments, ast)
    }

    fn key(&self, key: &'a str) -> PropertyKey<'a> {
        let ast = self.file.ast;
        if is_identifier_name_patched(key) {
            PropertyKey::new_static_identifier(SPAN, key, ast)
        } else {
            PropertyKey::new_string_literal(SPAN, key, None, ast)
        }
    }
}

/// A union's `members`, with an inner union's own members in its place.
fn flat(members: &[Shape]) -> Vec<&Shape> {
    let mut shapes = vec![];
    for member in members {
        match member {
            Shape::Union(inner) => shapes.extend(flat(inner)),
            member => shapes.push(member),
        }
    }
    shapes
}

/// Whether `shape` is `never`, or an object with a `never` part.
///
/// `$Input` and `$Output` intersect every part's keys, so one `never` part makes the object `never`.
fn is_never(shape: &Shape) -> bool {
    match shape {
        Shape::Never => true,
        Shape::Object(Object { rest: Rest::Parts(parts), .. }) => parts.iter().any(is_never),
        _ => false,
    }
}
