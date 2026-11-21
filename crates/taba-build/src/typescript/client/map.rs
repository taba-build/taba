//! A declarations file's map back to the Rust behind each name, for an editor's go to definition.

use std::{collections::BTreeMap, panic::Location, slice};

use taba_rolldown::{
    oxc::{
        allocator::Allocator,
        ast::ast::{
            BindingIdentifier, Function, Statement, TSNamespaceDeclaration, TSPropertySignature,
            TSTypeAliasDeclaration,
        },
        ast_visit::{Visit, walk},
        parser::Parser,
        span::{GetSpan, SourceType, Span},
        syntax::{line_terminator::is_line_terminator, scope::ScopeFlags},
    },
    rolldown_sourcemap::SourceMapBuilder,
};

use super::super::Origins;

/// The declaration map of `text`, the finished `file`, whose declarations come from `origins`.
///
/// `source` names each Rust file.
pub(super) fn map(
    file: &str,
    text: &str,
    origins: &Origins,
    source: impl Fn(&str) -> String,
) -> String {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, text, SourceType::d_ts()).parse().program;
    let mut tokens = Tokens {
        origins,
        path: Vec::new(),
        statement: None,
        keys: slice::Iter::default(),
        boundaries: BTreeMap::new(),
    };
    tokens.visit_program(&program);

    let locations = tokens.boundaries.values().flatten();
    let sources: BTreeMap<_, _> = locations.map(|at| (at.file(), source(at.file()))).collect();
    let positions = positions(text, tokens.boundaries.keys().copied());

    let mut builder = SourceMapBuilder::default();
    builder.set_file(file);
    for ((line, column), location) in positions.zip(tokens.boundaries.values()) {
        match location {
            Some(at) => {
                let id = builder.add_source_and_content(&sources[at.file()], "");
                builder.add_token(line, column, at.line() - 1, at.column() - 1, Some(id), None);
            }
            None => builder.add_token(line, column, 0, 0, None, None),
        }
    }
    let mut map = builder.into_sourcemap();
    map.set_source_contents(Vec::new());
    map.to_json_string()
}

/// Each token's offset in the text, and where the ones that map into Rust land.
struct Tokens<'a> {
    origins: &'a Origins,

    /// The namespaces around the current node.
    path: Vec<&'a str>,

    /// Where the current statement starts, until its first declaration takes it.
    statement: Option<u32>,

    /// The current declaration's keys still to come.
    keys: slice::Iter<'a, &'static Location<'static>>,

    /// Where each node starts and ends, and where in Rust a declaration or key maps one to.
    ///
    /// TypeScript reads a position with no token as the next token's, so every boundary gets one.
    boundaries: BTreeMap<u32, Option<&'static Location<'static>>>,
}

impl<'a> Tokens<'a> {
    /// Maps `id`, a declaration's name, and its statement's start to its origin, if it has one.
    ///
    /// TypeScript 7 takes a definition's file from the statement and its line from the name.
    fn declare(&mut self, id: &BindingIdentifier<'a>) {
        let statement = self.statement.take();
        let path: Vec<_> =
            self.path.iter().chain([&id.name.as_str()]).map(|&name| name.to_owned()).collect();
        let Some(origin) = self.origins.get(&path) else {
            self.keys = slice::Iter::default();
            return;
        };
        self.keys = origin.keys.iter();
        self.boundaries.extend(statement.map(|start| (start, Some(origin.name))));
        self.boundaries.insert(id.span.start, Some(origin.name));
    }
}

impl<'a> Visit<'a> for Tokens<'a> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        self.statement = Some(it.span().start);
        walk::walk_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.declare(id);
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_ts_type_alias_declaration(&mut self, it: &TSTypeAliasDeclaration<'a>) {
        self.declare(&it.id);
        walk::walk_ts_type_alias_declaration(self, it);
    }

    fn visit_ts_property_signature(&mut self, it: &TSPropertySignature<'a>) {
        if let Some(&at) = self.keys.next() {
            self.boundaries.insert(it.key.span().start, Some(at));
        }
        walk::walk_ts_property_signature(self, it);
    }

    fn visit_ts_namespace_declaration(&mut self, it: &TSNamespaceDeclaration<'a>) {
        self.declare(&it.id);
        self.path.push(it.id.name.as_str());
        walk::walk_ts_namespace_declaration(self, it);
        self.path.pop();
    }

    fn visit_span(&mut self, it: &Span) {
        for offset in [it.start, it.end] {
            self.boundaries.entry(offset).or_insert(None);
        }
    }
}

/// Each of `offsets`, ascending, as the 0-based line and UTF-16 column TypeScript counts.
fn positions(text: &str, offsets: impl Iterator<Item = u32>) -> impl Iterator<Item = (u32, u32)> {
    let mut chars = text.char_indices().peekable();
    let (mut line, mut column) = (0, 0);
    offsets.map(move |offset| {
        while let Some((index, char)) = chars.next_if(|&(index, _)| index < offset as usize) {
            match char {
                '\r' if text[index + 1..].starts_with('\n') => {}
                char if is_line_terminator(char) => (line, column) = (line + 1, 0),
                char => column += char.len_utf16() as u32,
            }
        }
        (line, column)
    })
}

#[cfg(test)]
mod tests {
    use std::panic::Location;

    use taba_rolldown::rolldown_sourcemap::OwnedSourceMap;

    use super::map;
    use crate::typescript::{Origin, Origins};

    #[test]
    fn each_declared_name_maps_to_its_origin_however_it_prints() {
        let cart = Location::caller();
        let buy = Location::caller();
        let items = Location::caller();
        let load = Location::caller();
        let by = Location::caller();
        let who = Location::caller();
        let origins = [
            ("crate.Cart", Origin { name: cart, keys: vec![items] }),
            ("crate.Buy", Origin { name: buy, keys: vec![] }),
            ("crate.Buy.Input", Origin { name: buy, keys: vec![load, by, who] }),
            ("buy", Origin { name: buy, keys: vec![] }),
        ];
        let origins: Origins = origins
            .into_iter()
            .map(|(path, origin)| (path.split('.').map(str::to_owned).collect(), origin))
            .collect();
        // formatted by hand, with a line separator in a doc comment
        let text = "import type * as $ from \"./index.js\";\n\
            export namespace crate {\n\
            \x20 /** A cart,\u{2028}to fill. */\n\
            \x20 export type Cart<T> = { items: T[] };\n\
            \x20 export namespace Buy {\n\
            \x20   export type Input = { load: Cart<string>; by: { who: string } };\n\
            \x20 }\n\
            \x20 export type Value = null;\n\
            }\n\
            export declare function buy(\n\
            \x20 request: crate.Buy.Input,\n\
            \x20 options?: { signal?: AbortSignal },\n\
            ): Promise<crate.Cart<string>>;\n";
        let json = map("index.d.ts", text, &origins, |file| format!("../{file}"));
        let map = OwnedSourceMap::from_json_string(&json).unwrap();
        let mut lines: Vec<_> = text.split(['\n', '\u{2028}']).map(str::to_owned).collect();
        let tokens: Vec<_> = map.get_tokens().collect();
        for token in tokens.iter().rev() {
            let at = (token.get_src_line() + 1, token.get_src_col() + 1);
            let label = token.get_source_id().map_or("|", |_| {
                let locations = [
                    ("[CART]", cart),
                    ("[BUY]", buy),
                    ("[items]", items),
                    ("[load]", load),
                    ("[by]", by),
                    ("[who]", who),
                ];
                let found = locations
                    .into_iter()
                    .find(|(_, location)| (location.line(), location.column()) == at);
                found.map_or("[elsewhere]", |(name, _)| name)
            });
            let line = &mut lines[token.get_dst_line() as usize];
            line.insert_str(token.get_dst_col() as usize, label);
        }
        insta::assert_snapshot!(lines.join("\n"), @r#"
        |import type |* as |$| from |"./index.js"|;|
        |export |namespace |crate| |{
          /** A cart,
        to fill. */
          [CART]export |type [CART]Cart|<|T|>| = |{ [items]items|: |T|[]| }|;|
          [BUY]export |namespace [BUY]Buy| |{
            [BUY]export |type [BUY]Input| = |{ [load]load|: |Cart|<|string|>|;| [by]by|: |{ [who]who|: |string| }| }|;|
          }|
          |export |type |Value| = |null|;|
        }|
        [BUY]export |declare function [BUY]buy|(
          |request|: |crate|.|Buy|.|Input|,
          |options|?|: |{ |signal|?|: |AbortSignal| }|,
        )|: |Promise|<|crate|.|Cart|<|string|>|>|;|
        |
        "#);
    }
}
