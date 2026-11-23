//! What [`serde_json`] writes and reads, using TypeScript's tsc to check declarations.

use std::fmt;

use serde_core::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use taba::{
    __private::typescript::{Named, Shape},
    Input, Output,
};
use taba_rolldown::oxc::{
    allocator::{Allocator, GetAllocator, Vec as ArenaVec},
    ast::{
        ast::{
            ArrayExpressionElement, BindingPattern, Declaration, Expression, NumberBase,
            ObjectPropertyKind, PropertyKey, PropertyKind, Statement, TSTypeAnnotation,
            VariableDeclarationKind, VariableDeclarator,
        },
        builder::AstBuilder,
    },
    codegen::Codegen,
    span::SPAN,
};

use super::{Declaring, Printer};

/// JSON that tsc checks against declarations, like taba crate's rustc ui tests.
///
/// Expected failures have `@ts-expect-error` to ensure that specific error happens.
#[derive(Debug, Default)]
pub struct Cases {
    cases: Vec<Case>,
}

/// A declaration attached to a JSON value to check conformance against.
#[derive(Debug)]
struct Case {
    json: Value,
    shape: &'static Shape,
    title: String,
}

impl Cases {
    /// Cases with no JSON to check.
    pub fn new() -> Self {
        Self::default()
    }

    /// The JSON [`serde_json`] writes for `value`, which `T`'s declaration must accept.
    pub fn output<T: Output + Serialize>(&mut self, value: &T) {
        let shape = const { &T::__SHAPE };
        let json = serde_json::to_value(value).expect("the case serializes");
        let title = format!("serde_json writes this for a `{}`", named(shape).name);
        self.cases.push(Case { json, shape, title });
    }

    /// `json`, which `T`'s declaration must accept exactly when [`serde_json`] reads it.
    pub fn input<T: Input + DeserializeOwned>(&mut self, json: &str) {
        let shape = const { &T::__SHAPE };
        let title = match serde_json::from_str::<T>(json) {
            Ok(_) => format!("serde_json reads this as a `{}`", named(shape).name),
            Err(error) => format!("@ts-expect-error serde_json: {error}"),
        };
        let json = serde_json::from_str(json).expect("the case is JSON");
        self.cases.push(Case { json, shape, title });
    }
}

impl fmt::Display for Cases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let allocator = Allocator::default();
        let ast = AstBuilder::new(&allocator);
        let mut printer = Printer::new(&ast);
        let cases = self.cases.iter().zip(1..);
        let cases = cases.map(|(case, number)| case.statement(&mut printer, number)).collect();
        let import = printer.import_self();
        f.write_str(&Codegen::new().build(&printer.program([import], cases)).code)
    }
}

impl Case {
    /// `export const case1: $.app.foo.Bar = ...;`, led by the case's title as a comment.
    fn statement<'a>(&self, printer: &mut Printer<'a>, number: usize) -> Statement<'a> {
        let ast = printer.ast;
        let span = printer.comments.line(&self.title);
        let declared = Declaring::new(printer, named(self.shape)).ts(self.shape);
        let declared = TSTypeAnnotation::boxed(SPAN, declared, ast);
        let name = ast.allocator().alloc_str(&format!("case{number}"));
        let id = BindingPattern::new_binding_identifier(SPAN, name, ast);
        let json = json(&self.json, ast);
        let constant = VariableDeclarator::new(SPAN, id, Some(declared), Some(json), false, ast);
        let kind = VariableDeclarationKind::Const;
        let constant = Declaration::new_variable_declaration(SPAN, kind, [constant], false, ast);
        Statement::new_export_declaration(span, constant, ast)
    }
}

/// The derived type `shape` refers to.
fn named(shape: &Shape) -> &'static Named {
    let Shape::Reference(named, _) = shape else { panic!("only a derived type has cases") };
    named()
}

/// `value` as a TypeScript literal.
fn json<'a>(value: &Value, ast: &AstBuilder<'a>) -> Expression<'a> {
    let string = |text: &str| ast.allocator().alloc_str(text);
    match value {
        Value::Null => Expression::new_null_literal(SPAN, ast),
        Value::Bool(value) => Expression::new_boolean_literal(SPAN, *value, ast),
        Value::Number(number) => {
            let value = number.as_f64().expect("serde_json's numbers fit a JavaScript number");
            Expression::new_numeric_literal(SPAN, value, None, NumberBase::Decimal, ast)
        }
        Value::String(text) => Expression::new_string_literal(SPAN, string(text), None, ast),
        Value::Array(items) => {
            let items = items.iter().map(|item| ArrayExpressionElement::from(json(item, ast)));
            Expression::new_array_expression(SPAN, ArenaVec::from_iter_in(items, ast), ast)
        }
        Value::Object(entries) => {
            let entries = entries.iter().map(|(key, value)| {
                let key = PropertyKey::new_string_literal(SPAN, string(key), None, ast);
                let value = json(value, ast);
                let kind = PropertyKind::Init;
                ObjectPropertyKind::new_object_property(
                    SPAN, kind, key, value, false, false, false, ast,
                )
            });
            Expression::new_object_expression(SPAN, ArenaVec::from_iter_in(entries, ast), ast)
        }
    }
}
