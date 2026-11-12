//! What serde_json writes and reads, as TypeScript sees it.
//!
//! [`Written`] is what TypeScript receives, and [`Read`] is what it may send. Each impl holds a
//! [`Shape`] as const data. A derived type is one [`Named`] static that other shapes refer to.
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
    pub use super::{Named, Object, Property, Read, Shape, Variant, Written};
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

    /// No value, such as an enum without variants.
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
    /// A `fn` so a recursive type's static can refer to itself.
    Reference(fn() -> &'static Named),
}

/// An object's keys.
#[derive(Debug)]
pub struct Object {
    /// Whether serde promises to reject keys the object doesn't declare.
    pub closed: bool,

    /// Its keys, in serde's order.
    pub properties: &'static [Property],
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

/// What serde_json writes for `Self`.
#[diagnostic::on_unimplemented(
    note = "derive `Written` alongside `Serialize`",
    note = "a type from another crate takes `#[taba(with = \"...\")]` on its field",
    note = "a 64-bit integer has no exact JavaScript number"
)]
pub trait Written {
    #[doc(hidden)]
    const __SHAPE: Shape;
}

/// What serde_json reads as `Self`.
#[diagnostic::on_unimplemented(
    note = "derive `Read` alongside `Deserialize`",
    note = "a type from another crate takes `#[taba(with = \"...\")]` on its field",
    note = "a 64-bit integer has no exact JavaScript number"
)]
pub trait Read {
    #[doc(hidden)]
    const __SHAPE: Shape;

    /// Whether a struct field of this type may be missing when read.
    #[doc(hidden)]
    const __OPTIONAL: bool = false;
}

macro_rules! both {
    ($shape:expr, $($ty:ty),+) => {$(
        impl Written for $ty {
            const __SHAPE: Shape = $shape;
        }

        impl Read for $ty {
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
        impl Written for $ty {
            const __SHAPE: Shape = Shape::Union(&[Shape::Number, Shape::Null]);
        }

        impl Read for $ty {
            const __SHAPE: Shape = Shape::Number;
        }
    )+};
}

float!(f32, f64);

impl Written for str {
    const __SHAPE: Shape = Shape::String;
}

impl<T: Written + ?Sized> Written for &T {
    const __SHAPE: Shape = T::__SHAPE;
}

impl<T: Written + ?Sized> Written for Box<T> {
    const __SHAPE: Shape = T::__SHAPE;
}

impl<T: Read> Read for Box<T> {
    const __SHAPE: Shape = T::__SHAPE;
    const __OPTIONAL: bool = T::__OPTIONAL;
}

impl<T: Written> Written for Option<T> {
    const __SHAPE: Shape = Shape::Union(&[T::__SHAPE, Shape::Null]);
}

impl<T: Read> Read for Option<T> {
    const __SHAPE: Shape = Shape::Union(&[T::__SHAPE, Shape::Null]);
    const __OPTIONAL: bool = true;
}

impl<T: Written> Written for [T] {
    const __SHAPE: Shape = Shape::Array(&T::__SHAPE);
}

macro_rules! generic {
    ($shape:expr, $($ty:ident),+) => {$(
        impl<T: Written> Written for $ty<T> {
            const __SHAPE: Shape = $shape;
        }

        impl<T: Read> Read for $ty<T> {
            const __SHAPE: Shape = $shape;
        }
    )+};
}

generic!(Shape::Array(&T::__SHAPE), Vec, BTreeSet);

impl<T: Written, const N: usize> Written for [T; N] {
    const __SHAPE: Shape = Shape::Tuple(&[T::__SHAPE; N]);
}

impl<T: Read, const N: usize> Read for [T; N] {
    const __SHAPE: Shape = Shape::Tuple(&[T::__SHAPE; N]);
}

macro_rules! tuples {
    () => {};
    ($first:ident $(, $rest:ident)*) => {
        impl<$first: Written $(, $rest: Written)*> Written for ($first, $($rest),*) {
            const __SHAPE: Shape = Shape::Tuple(&[$first::__SHAPE $(, $rest::__SHAPE)*]);
        }

        impl<$first: Read $(, $rest: Read)*> Read for ($first, $($rest),*) {
            const __SHAPE: Shape = Shape::Tuple(&[$first::__SHAPE $(, $rest::__SHAPE)*]);
        }

        tuples!($($rest),*);
    };
}

tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

/// A map key [`serde_json`] writes as a string.
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

impl<K: Key, V: Written> Written for BTreeMap<K, V> {
    const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
}

impl<K: Key, V: Read> Read for BTreeMap<K, V> {
    const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
}

/// std's collections.
#[cfg(feature = "std")]
mod std_collections {
    use std::collections::{HashMap, HashSet};

    use super::{Key, Read, Shape, Written};

    impl<T: Written, S> Written for HashSet<T, S> {
        const __SHAPE: Shape = Shape::Array(&T::__SHAPE);
    }

    impl<T: Read, S> Read for HashSet<T, S> {
        const __SHAPE: Shape = Shape::Array(&T::__SHAPE);
    }

    impl<K: Key, V: Written, S> Written for HashMap<K, V, S> {
        const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
    }

    impl<K: Key, V: Read, S> Read for HashMap<K, V, S> {
        const __SHAPE: Shape = Shape::Record(&V::__SHAPE);
    }
}
