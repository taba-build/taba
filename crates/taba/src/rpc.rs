//! Taba's RPC

use core::convert::Infallible;

use serde_core::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned},
};
use specta::{Type, Types, datatype::DataType};
use specta_typescript::Number;

use self::__private::SafeInteger;

#[doc(hidden)]
pub mod __private {
    /// A [Safe JavaScript Number](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/isSafeInteger#description).
    #[derive(Debug)]
    pub struct SafeInteger(pub(super) u64);
}

/// A request from the page, with its handler's return types.
pub trait Procedure: DeserializeOwned + Type + Send + 'static {
    /// A successful handler in `Ok`.
    type Output: Serialize + Type + Send;

    /// A failed handler in `Err`.
    type Error: Serialize + Type + Send;
}

/// An error that can never happen.
#[derive(Debug)]
pub enum Never {}

impl Serialize for Never {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

impl Type for Never {
    fn definition(types: &mut Types) -> DataType {
        Infallible::definition(types)
    }
}

impl SafeInteger {
    /// [`Number.MAX_SAFE_INTEGER`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER).
    const MAX: u64 = (1 << 53) - 1;
}

impl Serialize for SafeInteger {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0) // we enforce the max at creation
    }
}

impl<'de> Deserialize<'de> for SafeInteger {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match u64::deserialize(deserializer)? {
            x if x <= Self::MAX => Ok(SafeInteger(x)),
            x => Err(de::Error::custom(format_args!("{x} > `Number.MAX_SAFE_INTEGER`"))),
        }
    }
}

impl Type for SafeInteger {
    fn definition(types: &mut Types) -> DataType {
        Number::<()>::definition(types)
    }
}

#[cfg(test)]
mod tests {
    use specta::{Type, Types};
    use specta_typescript::{Typescript, primitives};

    use super::SafeInteger;

    /// [`Number.MAX_SAFE_INTEGER`]'s example.
    ///
    /// [`Number.MAX_SAFE_INTEGER`]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER
    #[test]
    fn max_safe_integer() {
        let x = SafeInteger::MAX + 1;
        let y = SafeInteger::MAX + 2;

        insta::assert_snapshot!(SafeInteger::MAX, @"9007199254740991");
        insta::assert_snapshot!(x, @"9007199254740992");
        insta::assert_snapshot!(x as f64 == y as f64, @"true"); // Rust is also IEEE 754

        let max: SafeInteger = serde_json::from_str("9007199254740991").unwrap();
        insta::assert_snapshot!(serde_json::to_string(&max).unwrap(), @"9007199254740991");

        let error = serde_json::from_str::<SafeInteger>("9007199254740992").unwrap_err();
        insta::assert_snapshot!(error, @"9007199254740992 > `Number.MAX_SAFE_INTEGER`");

        let mut types = Types::default();
        let definition = SafeInteger::definition(&mut types);
        let exported = primitives::inline(&Typescript::default(), &types, &definition).unwrap();
        insta::assert_snapshot!(exported, @"number");
    }
}
