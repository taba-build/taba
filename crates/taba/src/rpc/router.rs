//! Procedure router, how an app attaches handlers to procedures.

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{any::type_name, fmt, future::ready, pin::Pin};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::value::{RawValue, to_raw_value};
use specta::{
    Type, Types,
    datatype::{DataType, Reference},
};

use super::{Procedure, wire::Kind};

/// Procedures and their handlers.
#[derive(Debug, Default)]
pub struct Procedures {
    names: BTreeMap<String, Pairing>,
    unnamed: Vec<Problem>,
}

#[derive(Debug, Default)]
struct Pairing {
    declared: bool,
    handlers: Vec<Call>,
}

impl Procedures {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `P`.
    pub fn declare<P>(mut self) -> Self
    where
        P: Procedure + Type,
    {
        match Self::name::<P>() {
            Ok(name) => self.names.entry(name).or_default().declared = true,
            Err(problem) => self.unnamed.push(problem),
        }
        self
    }

    /// Answers `P`'s calls with `handler`.
    pub fn handle<P, H>(mut self, handler: H) -> Self
    where
        P: DeserializeOwned + Procedure + Type,
        P::Output: Serialize + 'static, // calls only, Strand isn't Serialize
        P::Error: Serialize + 'static,
        H: Handler<P> + Send + Sync + 'static,
        H::Future: Future<Output = Result<P::Output, P::Error>> + Send + 'static,
    {
        match Self::name::<P>() {
            Ok(name) => self.names.entry(name).or_default().handlers.push(Call::new(handler)),
            Err(problem) => self.unnamed.push(problem),
        }
        self
    }

    /// A router with one handler for each procedure.
    ///
    /// # Errors
    ///
    /// Every procedure and handler that don't pair up.
    pub fn build(self) -> Result<Router, Error> {
        let mut problems = self.unnamed;
        let mut calls = BTreeMap::new();
        for (name, Pairing { declared, mut handlers }) in self.names {
            let call = handlers.pop();
            match (declared, call, handlers.len()) {
                (true, Some(call), 0) => _ = calls.insert(name, call),
                (true, None, _) => problems.push(Problem::Missing(name)),
                (true, Some(_), rest) => problems.push(Problem::Several(name, rest + 1)),
                (false, _, _) => problems.push(Problem::Undeclared(name)),
            }
        }
        if !problems.is_empty() {
            return Err(Error(problems));
        }
        Ok(Router { calls })
    }

    /// `P`'s path, crate first, so two modules can share a type name.
    fn name<P: Type>() -> Result<String, Problem> {
        let mut types = Types::default();
        let DataType::Reference(Reference::Named(reference)) = P::definition(&mut types) else {
            return Err(Problem::Unnamed(type_name::<P>()));
        };
        let named = types.get(&reference).ok_or(Problem::Unnamed(type_name::<P>()))?;
        Ok(format!("{}::{}", named.module_path, named.name))
    }
}

/// A function that answers `P`'s calls.
pub trait Handler<P> {
    /// Its answer.
    type Future;

    /// Answers `request`.
    fn call(&self, request: P) -> Self::Future;
}

impl<P, F, Fut> Handler<P> for F
where
    F: Fn(P) -> Fut,
{
    type Future = Fut;

    fn call(&self, request: P) -> Fut {
        self(request)
    }
}

/// Handlers, by procedure name.
#[derive(Debug)]
pub struct Router {
    calls: BTreeMap<String, Call>,
}

impl Router {
    /// Answers a call.
    pub fn call(
        &self,
        name: &str,
        request: &RawValue,
    ) -> impl Future<Output = Kind> + Send + use<> {
        match self.calls.get(name) {
            Some(call) => (call.0)(request),
            None => Box::pin(ready(Kind::Reject(format!("no procedure `{name}`")))),
        }
    }
}

/// A handler that parses its request first, so its answer borrows nothing.
struct Call(Box<dyn Fn(&RawValue) -> Answer + Send + Sync>);

type Answer = Pin<Box<dyn Future<Output = Kind> + Send>>;

impl Call {
    fn new<P, H>(handler: H) -> Self
    where
        P: DeserializeOwned + Procedure,
        P::Output: Serialize + 'static,
        P::Error: Serialize + 'static,
        H: Handler<P> + Send + Sync + 'static,
        H::Future: Future<Output = Result<P::Output, P::Error>> + Send + 'static,
    {
        Self(Box::new(move |request| match serde_json::from_str::<P>(request.get()) {
            Ok(request) => Box::pin(Self::answer(handler.call(request))),
            Err(error) => Box::pin(ready(Kind::Reject(error.to_string()))),
        }))
    }

    async fn answer<T, E, Fut>(future: Fut) -> Kind
    where
        T: Serialize,
        E: Serialize,
        Fut: Future<Output = Result<T, E>>,
    {
        let encoded = match future.await {
            Ok(output) => to_raw_value(&output).map(Kind::Value),
            Err(error) => to_raw_value(&error).map(Kind::Error),
        };
        encoded.unwrap_or_else(|error| Kind::Reject(error.to_string()))
    }
}

impl fmt::Debug for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Call").finish_non_exhaustive()
    }
}

/// Failed to build a [`Router`].
#[derive(Debug)]
pub struct Error(Vec<Problem>);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("can't build the router:")?;
        self.0.iter().try_for_each(|problem| write!(f, "\n\t{problem}"))
    }
}

impl core::error::Error for Error {}

#[derive(Debug)]
enum Problem {
    Missing(String),
    Several(String, usize),
    Undeclared(String),
    Unnamed(&'static str),
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unnamed(procedure) => write!(f, "`{procedure}`'s `Type` isn't a named type"),
            Self::Missing(name) => write!(f, "no handler for `{name}`"),
            Self::Several(name, count) => write!(f, "{count} handlers for `{name}`"),
            Self::Undeclared(name) => write!(f, "a handler for `{name}`, which isn't declared"),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::{
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use serde_json::value::RawValue;

    use self::idle_game::{Buy, Sell, buy, hire};
    use super::Procedures;

    mod idle_game {
        //! Example application for testing.
        // used by specta's derives, root crate is no_std
        extern crate std;

        use alloc::string::String;
        use std::vec;

        use serde::{Deserialize, Serialize};
        use specta::Type;

        use crate::rpc::{Never, Procedure};

        #[derive(Deserialize, Type)]
        pub(super) struct Buy {
            generator: String,
        }

        #[derive(Serialize, Type)]
        pub(super) enum BuyError {
            CantAfford { missing: u32 },
            Locked,
        }

        impl Procedure for Buy {
            type Output = u32;
            type Error = BuyError;
        }

        pub(super) async fn buy(Buy { generator }: Buy) -> Result<u32, BuyError> {
            match generator.as_str() {
                "drill" => Ok(4),
                "furnace" => Err(BuyError::CantAfford { missing: 30 }),
                _ => Err(BuyError::Locked),
            }
        }

        #[derive(Deserialize, Type)]
        pub(super) struct Sell;

        impl Procedure for Sell {
            type Output = u32;
            type Error = Never;
        }

        #[derive(Deserialize, Type)]
        pub(super) struct Hire;

        impl Procedure for Hire {
            type Output = u32;
            type Error = Never;
        }

        pub(super) async fn hire(_: Hire) -> Result<u32, Never> {
            Ok(1)
        }
    }

    #[test]
    fn every_answer_kind() {
        let router = Procedures::new().declare::<Buy>().handle(buy).build().unwrap();
        let calls = [
            ("taba::rpc::router::tests::idle_game::Buy", r#"{"generator":"drill"}"#),
            ("taba::rpc::router::tests::idle_game::Buy", r#"{"generator":"furnace"}"#),
            ("taba::rpc::router::tests::idle_game::Buy", r#"{"generatr":"drill"}"#),
            ("taba::rpc::router::tests::idle_game::Sell", "null"),
        ];
        let answers = calls.map(|(name, request)| {
            let request = RawValue::from_string(request.into()).unwrap();
            let answer = now(router.call(name, &request));
            serde_json::to_string(&answer).unwrap()
        });
        insta::assert_snapshot!(answers.join("\n"), @r#"
        {"kind":"value","value":4}
        {"kind":"error","value":{"CantAfford":{"missing":30}}}
        {"kind":"reject","value":"missing field `generator` at line 1 column 20"}
        {"kind":"reject","value":"no procedure `taba::rpc::router::tests::idle_game::Sell`"}
        "#);
    }

    #[test]
    fn every_build_problem() {
        let procedures = Procedures::new().declare::<Buy>().declare::<Sell>();
        let error = procedures.handle(buy).handle(buy).handle(hire).build().unwrap_err();
        insta::assert_snapshot!(error, @"
        can't build the router:
        \t2 handlers for `taba::rpc::router::tests::idle_game::Buy`
        \ta handler for `taba::rpc::router::tests::idle_game::Hire`, which isn't declared
        \tno handler for `taba::rpc::router::tests::idle_game::Sell`
        ");
    }

    /// Polls once: these handlers never wait.
    fn now<T>(future: impl Future<Output = T>) -> T {
        let Poll::Ready(output) = pin!(future).poll(&mut Context::from_waker(Waker::noop())) else {
            panic!("a handler waited");
        };
        output
    }
}
