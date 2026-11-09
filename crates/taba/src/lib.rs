//! Taba's runtime
#![no_std]

extern crate alloc;

mod error;
#[cfg(feature = "rpc")]
pub mod rpc;

use alloc::string::{String, ToString as _};
use core::fmt::{self, Display, Write};

use serde_core::Serialize;

pub use crate::error::Error;

/// Alias for a `Result` with the error type `taba::Error`.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// What generated code names through taba: a std crate has no `alloc` without [`extern crate`](https://doc.rust-lang.org/reference/names/preludes.html#extern-prelude).
#[doc(hidden)]
pub mod __private {
    pub use alloc::string::String;

    #[cfg(feature = "rpc")]
    pub use crate::rpc::__private as rpc;
}

/// Include `taba-build` output as the named module.
///
/// ```ignore
/// taba::include_scripts!(mod scripts);
/// let js = scripts::core(&input)?;
/// ```
///
/// Attributes and documentation work fine:
///
/// ```ignore
/// taba::include_scripts!(
///     /// Some explicit documentation we want to add to the binding's root.
///     #[forbid(unsafe_code)]
///     mod scripts;
/// );
/// ```
#[macro_export]
macro_rules! include_scripts {
    ($(#[$attribute:meta])* $visibility:vis mod $name:ident $(;)?) => {
        $(#[$attribute])*
        $visibility mod $name {
            ::core::include!(::core::concat!(::core::env!("OUT_DIR"), "/taba/mod.rs"));
        }
    };
}

/// The unresolved ("global") block-scoped variable scripts read input from.
const BLOCK_SCOPED_GLOBAL: &str = "_taba";

/// A script that runs the bundle with input.
pub fn script<T>(bundle: &str, input: &T) -> Result<String, Error>
where
    T: ?Sized + Serialize,
{
    let input = Json::serialize(input)?.declare_constant();
    Ok(block(format_args!("{input}\n{bundle}")).to_string())
}

/// A [block statement] to scope our input, preventing any global naming collisions.
///
/// [block statement]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/block
fn block(body: impl Display) -> impl Display {
    fmt::from_fn(move |f| write!(f, "{{\n{body}\n}}"))
}

/// The ability to safely escape JSON into a JSON.parse('{json}') relies entirely on 2 things.
///
/// 1. `serde_json`'s ability to correctly escape and format JSON into a [`String`].
/// 2. JavaScript engines not accepting anything except another unescaped, literal single quote
///    character to end a string that was opened with it.
struct Json(String);

impl Json {
    fn serialize<T: ?Sized + Serialize>(input: &T) -> Result<Self, Error> {
        serde_json::to_string(input).map(Self).map_err(Error)
    }

    fn declare_constant(self) -> impl Display {
        fmt::from_fn(move |f| write!(f, "const {BLOCK_SCOPED_GLOBAL} = {};", self.json_parse()))
    }

    fn json_parse(&self) -> impl Display {
        fmt::from_fn(move |f| write!(f, "JSON.parse({})", self.string_literal()))
    }

    /// A single-quoted JavaScript string literal, for use inside `JSON.parse()`.
    ///
    /// Single quotes chosen because double quotes are already used in JSON. With single quotes, we
    /// only need to escape strings that include backslashes or single quotes. If we used double
    /// quotes, then there would be no cases that a string doesn't need escaping.
    fn string_literal(&self) -> impl Display {
        const BACKSLASH: char = '\\';
        const SINGLE_QUOTE: char = '\'';
        fmt::from_fn(move |f| {
            f.write_char(SINGLE_QUOTE)?;

            let mut last = 0;
            for (idx, _) in self.0.match_indices([BACKSLASH, SINGLE_QUOTE]) {
                f.write_str(&self.0[last..idx])?;
                f.write_char(BACKSLASH)?;
                last = idx;
            }

            f.write_str(&self.0[last..])?;
            f.write_char(SINGLE_QUOTE)
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_json_type() {
        let input = serde_json::json!({ "input": [null, true, 42, r"it's C:\"] });
        let bundle = "run(_taba);\n//# sourceMappingURL=run.js.map";

        insta::assert_snapshot!(super::script(bundle, &input).unwrap(), @r#"
        {
        const _taba = JSON.parse('{"input":[null,true,42,"it\'s C:\\\\"]}');
        run(_taba);
        //# sourceMappingURL=run.js.map
        }
        "#);
    }
}
