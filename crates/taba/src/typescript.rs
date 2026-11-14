//! What serde_json writes and reads, as TypeScript sees it.
//!
//! [`Output`] is what TypeScript receives, and [`Input`] is what it may send. Each impl holds a
//! [`Shape`] as const data. A derived type is one [`Named`] that other shapes refer to.
//! Only the derives and the impls here build shapes, so they're hidden.
//!
//! 64-bit integers have no impl, because a JavaScript number can't hold one exactly. They're fine
//! as map keys, which travel as strings.

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

#[doc(hidden)]
pub mod __private {
    pub use super::{Direction, Input, Named, Object, Output, Property, Rest, Shape, Variant};
}

/// One JSON shape.
#[derive(Debug)]
pub enum Shape {
    /// `boolean`.
    Boolean,

    /// `number`.
    Number,

    /// `string`.
    String,

    /// `null`.
    Null,

    /// No value.
    Never,

    /// A string literal type.
    Literal(&'static str),

    /// A homogeneous array.
    Array(&'static Shape),

    /// A fixed-length array.
    Tuple(&'static [Shape]),

    /// An object with any string keys.
    Record(&'static Shape),

    /// An object with fixed keys.
    Object(&'static Object),

    /// Any one of these.
    Union(&'static [Shape]),

    /// The first of these that serde reads.
    Choice(&'static [Variant]),

    /// A named type, declared once.
    ///
    /// A `fn` so a recursive type can refer to itself.
    Reference(fn() -> &'static Named),
}

/// An object's keys.
#[derive(Debug)]
pub struct Object {
    /// Its keys, in serde's order.
    pub properties: &'static [Property],

    /// Its other keys.
    pub rest: Rest,
}

/// What serde does with an [`Object`]'s other keys.
#[derive(Debug)]
pub enum Rest {
    /// Ignores them.
    Ignored,

    /// Rejects them, under `deny_unknown_fields`.
    Rejected,

    /// They're the keys of these `#[serde(flatten)]` fields.
    ///
    /// A `null` adds none, which is what a flattened `None` writes.
    Flattened(&'static [Shape]),
}

/// One key of an [`Object`].
#[derive(Debug)]
pub struct Property {
    /// The field's `#[doc]` values, like [`Named::docs`].
    pub docs: &'static [&'static str],

    /// The key, as serde names it.
    pub key: &'static str,

    /// Whether the key may be missing.
    pub optional: bool,

    /// The value's shape.
    pub shape: Shape,
}

/// One variant of a [`Shape::Choice`].
#[derive(Debug)]
pub struct Variant {
    /// Its Rust name.
    pub name: &'static str,

    /// Its shape.
    pub shape: Shape,
}

/// A type the derives describe, where it's defined.
#[derive(Debug)]
pub struct Named {
    /// Its `#[doc]` values, as written.
    ///
    /// Each `///` line is one value, still led by the space after the slashes.
    pub docs: &'static [&'static str],

    /// Which derive built it.
    pub direction: Direction,

    /// Where it derives, from [`file!`].
    pub file: &'static str,

    /// Where it derives, from [`line!`].
    pub line: u32,

    /// Its module, from [`module_path!`].
    pub module_path: &'static str,

    /// Its Rust name.
    pub name: &'static str,

    /// Its shape.
    pub shape: Shape,
}

/// Which derive built a [`Named`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    /// [`Output`]'s.
    Output,

    /// [`Input`]'s.
    Input,
}

/// What serde_json writes for `Self`.
#[diagnostic::on_unimplemented(
    note = "derive `Output` alongside `Serialize`",
    note = "a type from another crate takes `#[taba(with = \"...\")]` on its field",
    note = "a 64-bit integer has no exact JavaScript number"
)]
pub trait Output {
    #[doc(hidden)]
    const __SHAPE: Shape;
}

/// What serde_json reads as `Self`.
#[diagnostic::on_unimplemented(
    note = "derive `Input` alongside `Deserialize`",
    note = "a type from another crate takes `#[taba(with = \"...\")]` on its field",
    note = "a 64-bit integer has no exact JavaScript number"
)]
pub trait Input {
    #[doc(hidden)]
    const __SHAPE: Shape;

    /// Whether a struct field of this type may be missing when read.
    #[doc(hidden)]
    const __OPTIONAL: bool = false;
}

macro_rules! both {
    ($shape:expr, $($ty:ty),+) => {$(
        impl Output for $ty {
            const __SHAPE: Shape = $shape;
        }

        impl Input for $ty {
            const __SHAPE: Shape = $shape;
        }
    )+};
}

both!(Shape::Boolean, bool);
both!(Shape::Number, u8, u16, u32, i8, i16, i32);
both!(Shape::String, String, char);
both!(Shape::Null, ());

macro_rules! float {
    ($($ty:ty),+) => {$(
        /// Nullable because JSON doesn't represent `NaN` or infinity.
        impl Output for $ty {
            const __SHAPE: Shape = Shape::Union(&[Shape::Number, Shape::Null]);
        }

        impl Input for $ty {
            const __SHAPE: Shape = Shape::Number;
        }
    )+};
}

float!(f32, f64);

impl Output for str {
    const __SHAPE: Shape = Shape::String;
}

impl<T: Output + ?Sized> Output for &T {
    const __SHAPE: Shape = T::__SHAPE;
}

impl<T: Output + ?Sized> Output for Box<T> {
    const __SHAPE: Shape = T::__SHAPE;
}

impl<T: Input> Input for Box<T> {
    const __SHAPE: Shape = T::__SHAPE;
    const __OPTIONAL: bool = T::__OPTIONAL;
}

impl<T: Output> Output for Option<T> {
    const __SHAPE: Shape = Shape::Union(&[T::__SHAPE, Shape::Null]);
}

impl<T: Input> Input for Option<T> {
    const __SHAPE: Shape = Shape::Union(&[T::__SHAPE, Shape::Null]);
    const __OPTIONAL: bool = true;
}

impl<T: Output> Output for [T] {
    const __SHAPE: Shape = Shape::Array(&T::__SHAPE);
}

macro_rules! generic {
    ($shape:expr, $($ty:ident),+) => {$(
        impl<T: Output> Output for $ty<T> {
            const __SHAPE: Shape = $shape;
        }

        impl<T: Input> Input for $ty<T> {
            const __SHAPE: Shape = $shape;
        }
    )+};
}

generic!(Shape::Array(&T::__SHAPE), Vec, BTreeSet);

impl<T: Output, const N: usize> Output for [T; N] {
    const __SHAPE: Shape = Shape::Tuple(&[T::__SHAPE; N]);
}

impl<T: Input, const N: usize> Input for [T; N] {
    const __SHAPE: Shape = Shape::Tuple(&[T::__SHAPE; N]);
}

macro_rules! tuples {
    () => {};
    ($first:ident $(, $rest:ident)*) => {
        impl<$first: Output $(, $rest: Output)*> Output for ($first, $($rest),*) {
            const __SHAPE: Shape = Shape::Tuple(&[$first::__SHAPE $(, $rest::__SHAPE)*]);
        }

        impl<$first: Input $(, $rest: Input)*> Input for ($first, $($rest),*) {
            const __SHAPE: Shape = Shape::Tuple(&[$first::__SHAPE $(, $rest::__SHAPE)*]);
        }

        tuples!($($rest),*);
    };
}

tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

/// A map key [`serde_json`](::serde_json) writes as a string.
///
/// Integer keys are exact as digit strings, including 64 and 128 bit.
#[diagnostic::on_unimplemented(note = "a key is a string, `char`, `bool` or integer")]
pub trait Key {}

macro_rules! key {
    ($($ty:ty),+) => {$(impl Key for $ty {})+};
}

key!(String, char, bool);
key!(u8, u16, u32, u64, u128, usize);
key!(i8, i16, i32, i64, i128, isize);

impl<K: Key, V: Output> Output for BTreeMap<K, V> {
    const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
}

impl<K: Key, V: Input> Input for BTreeMap<K, V> {
    const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
}

mod serde_json {
    use ::serde_json::{Map, Number, Value, value::RawValue};

    use super::*;

    both!(Shape::Number, Number);

    macro_rules! value {
        ($trait:ident) => {
            impl $trait for Value {
                const __SHAPE: Shape = Shape::Reference(|| &Named {
                    docs: &[],
                    direction: Direction::$trait,
                    file: file!(),
                    line: line!(),
                    module_path: "serde_json::value",
                    name: "Value",
                    shape: Shape::Union(&[
                        Shape::Null,
                        Shape::Boolean,
                        Shape::Number,
                        Shape::String,
                        Shape::Array(&<Value as $trait>::__SHAPE),
                        Shape::Record(&<Value as $trait>::__SHAPE),
                    ]),
                });
            }

            impl $trait for Map<String, Value> {
                const __SHAPE: Shape = Shape::Record(&<Value as $trait>::__SHAPE);
            }
        };
    }

    value!(Output);
    value!(Input);

    impl Output for RawValue {
        const __SHAPE: Shape = <Value as Output>::__SHAPE;
    }

    impl Input for Box<RawValue> {
        const __SHAPE: Shape = <Value as Input>::__SHAPE;
    }

    impl Input for &RawValue {
        const __SHAPE: Shape = <Value as Input>::__SHAPE;
    }
}

#[cfg(feature = "std")]
mod std {
    use std::collections::{HashMap, HashSet};

    use super::{Input, Key, Output, Shape};

    impl<T: Output, S> Output for HashSet<T, S> {
        const __SHAPE: Shape = Shape::Array(&T::__SHAPE);
    }

    impl<T: Input, S> Input for HashSet<T, S> {
        const __SHAPE: Shape = Shape::Array(&T::__SHAPE);
    }

    impl<K: Key, V: Output, S> Output for HashMap<K, V, S> {
        const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
    }

    impl<K: Key, V: Input, S> Input for HashMap<K, V, S> {
        const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
    }
}
