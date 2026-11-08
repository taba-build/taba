//! Rust <-> page.

use alloc::{boxed::Box, string::String};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::value::RawValue;
use specta::{Type, Types, datatype::DataType};
use specta_typescript::Number;

/// A [Safe JavaScript Number](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/isSafeInteger#description).
#[derive(Debug)]
pub struct SafeInteger(u64);

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

/// Requests from the page.
///
/// `value` comes last for [`RawValue`]'s deferred parsing.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Message<'a> {
    /// Answered once.
    Call(SafeInteger, &'a str, #[serde(borrow)] &'a RawValue),
    /// Answered once per [`Next`](Self::Next).
    Subscribe(SafeInteger, &'a str, #[serde(borrow)] &'a RawValue),
    /// Pulls one value.
    Next(SafeInteger),
    /// Abort a call or subscription's task.
    Cancel(SafeInteger),
}

/// What a poll returns.
#[derive(Debug, Serialize)]
pub struct Frame {
    /// Its message's id.
    pub id: SafeInteger,
    /// Beside `id` on the wire.
    #[serde(flatten)]
    pub kind: Kind,
}

/// What a frame tells the page.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Kind {
    /// A handler's `Ok`.
    Value(Box<RawValue>),
    /// A handler's `Err`, which no frame follows.
    Error(Box<RawValue>),
    /// After a stream's last value.
    End,
    /// [`taba`](crate)'s own failure, which the page rejects.
    Reject(String),
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use serde_json::value::RawValue;
    use specta::{Type, Types};
    use specta_typescript::{Typescript, primitives};

    use super::{Frame, Kind, Message, SafeInteger};

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

    #[test]
    fn every_message_kind() {
        let batch = r#"[
            {"kind":"call","value":[7,"buy",{"generator":"drill"}]},
            {"kind":"subscribe","value":[8,"resources",{"resource":"ore"}]},
            {"kind":"next","value":8},
            {"kind":"cancel","value":8}
        ]"#;
        let batch: Vec<Message> = serde_json::from_str(batch).unwrap();
        insta::assert_debug_snapshot!(batch, @r#"
        [
            Call(
                SafeInteger(
                    7,
                ),
                "buy",
                RawValue(
                    {"generator":"drill"},
                ),
            ),
            Subscribe(
                SafeInteger(
                    8,
                ),
                "resources",
                RawValue(
                    {"resource":"ore"},
                ),
            ),
            Next(
                SafeInteger(
                    8,
                ),
            ),
            Cancel(
                SafeInteger(
                    8,
                ),
            ),
        ]
        "#);
    }

    #[test]
    fn every_frame_kind() {
        let json = |json: &str| RawValue::from_string(json.into()).unwrap();
        let frames = [
            Frame { id: SafeInteger(0), kind: Kind::Value(json("4")) },
            Frame { id: SafeInteger(1), kind: Kind::Error(json(r#""not enough ore""#)) },
            Frame { id: SafeInteger(2), kind: Kind::End },
            Frame { id: SafeInteger(3), kind: Kind::Reject("this handler panicked".into()) },
        ];
        let frames = frames.map(|frame| serde_json::to_string(&frame).unwrap());
        insta::assert_snapshot!(frames.join("\n"), @r#"
        {"id":0,"kind":"value","value":4}
        {"id":1,"kind":"error","value":"not enough ore"}
        {"id":2,"kind":"end"}
        {"id":3,"kind":"reject","value":"this handler panicked"}
        "#);
    }
}
