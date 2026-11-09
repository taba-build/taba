//! Procedure router, how an app attaches handlers to procedures.

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{any::type_name, fmt, future::ready};

use futures_util::{FutureExt, Stream, StreamExt, stream};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::value::{RawValue, to_raw_value};
use specta::{
    Type, Types,
    datatype::{DataType, Reference},
};

use self::sealed::Erased;
use super::{Once, Output, Procedure, Strand, wire::Kind};

/// Procedures, and the handlers an app attaches.
///
/// `S` is the state the next handlers get, `()` until [`with_state`](Self::with_state) is called.
#[derive(Debug)]
pub struct Procedures<S = ()> {
    names: BTreeMap<String, Pairing>,
    unnamed: Vec<Problem>,
    state: S,
}

/// A name's declaration and handlers.
#[derive(Debug, Default)]
struct Pairing {
    declared: bool,
    handlers: Vec<Erased>,
}

impl Procedures {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Procedures {
    fn default() -> Self {
        Self { names: BTreeMap::new(), unnamed: Vec::new(), state: () }
    }
}

impl<S> Procedures<S> {
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

    /// Gives the next handlers `state`.
    pub fn with_state<S2>(self, state: S2) -> Procedures<S2> {
        Procedures { names: self.names, unnamed: self.unnamed, state }
    }

    /// Answers `P` with `handler`.
    pub fn handle<H, Args, P>(self, handler: H) -> Self
    where
        H: Handler<Args, S, Request = P> + Send + Sync + 'static,
        P: DeserializeOwned + Procedure + Type,
        P::Output: Output,
        H::Future: Answers<<P::Output as Output>::Shape, P::Error>,
        S: Clone + Send + Sync + 'static,
    {
        let state = self.state.clone();
        self.handle_with(state, handler)
    }

    /// Answers `P` with `handler`, which gets its own `state`.
    pub fn handle_with<S2, H, Args, P>(mut self, state: S2, handler: H) -> Self
    where
        H: Handler<Args, S2, Request = P> + Send + Sync + 'static,
        P: DeserializeOwned + Procedure + Type,
        P::Output: Output,
        H::Future: Answers<<P::Output as Output>::Shape, P::Error>,
        S2: Clone + Send + Sync + 'static,
    {
        let parsed = move |request: &RawValue| match serde_json::from_str::<P>(request.get()) {
            Ok(request) => Ok(handler.call(state.clone(), request)),
            Err(error) => Err(Kind::invalid(error)),
        };
        let erased = H::Future::erase(parsed);
        match Self::name::<P>() {
            Ok(name) => self.names.entry(name).or_default().handlers.push(erased),
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
        let mut paired = BTreeMap::new();
        for (name, Pairing { declared, mut handlers }) in self.names {
            let handler = handlers.pop();
            match (declared, handler, handlers.len()) {
                (true, Some(handler), 0) => _ = paired.insert(name, handler),
                (true, None, _) => problems.push(Problem::Missing(name)),
                (true, Some(_), rest) => problems.push(Problem::Several(name, rest + 1)),
                (false, _, _) => problems.push(Problem::Undeclared(name)),
            }
        }
        if !problems.is_empty() {
            return Err(Error(problems));
        }
        Ok(Router { handlers: paired })
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

/// A function that answers a procedure.
pub trait Handler<Args, S> {
    /// The procedure it answers.
    type Request;

    /// Its answer.
    type Future;

    /// Answers `request`, given the app's state.
    fn call(&self, state: S, request: Self::Request) -> Self::Future;
}

impl<F, Fut, P, S> Handler<(P,), S> for F
where
    F: Fn(P) -> Fut,
{
    type Request = P;
    type Future = Fut;

    fn call(&self, _: S, request: P) -> Fut {
        self(request)
    }
}

impl<F, Fut, S, P> Handler<(State<S>, P), S> for F
where
    F: Fn(State<S>, P) -> Fut,
{
    type Request = P;
    type Future = Fut;

    fn call(&self, state: S, request: P) -> Fut {
        self(State(state), request)
    }
}

/// The app's state, as a handler's argument.
#[derive(Debug)]
pub struct State<S>(pub S);

/// Handlers, by procedure name.
#[derive(Debug)]
pub struct Router {
    handlers: BTreeMap<String, Erased>,
}

impl Router {
    /// Answers a call.
    pub fn call(
        &self,
        name: &str,
        request: &RawValue,
    ) -> impl Future<Output = Kind> + Send + use<> {
        match self.handlers.get(name) {
            Some(Erased::Call(call)) => call(request),
            _ => ready(Kind::unknown(name.to_string())).boxed(),
        }
    }

    /// Answers a subscription, a frame per item, until its handler's stream ends.
    pub fn subscribe(
        &self,
        name: &str,
        request: &RawValue,
    ) -> impl Stream<Item = Kind> + Send + use<> {
        match self.handlers.get(name) {
            Some(Erased::Strand(strand)) => strand(request),
            _ => stream::once(ready(Kind::unknown(name.to_string()))).boxed(),
        }
    }
}

/// What a handler returns, answering a procedure whose output has the shape `O`.
///
/// A future answers [`Once`], and a stream answers [`Strand`]. Sitting on what the handler
/// returns puts that type first in rustc's errors.
///
/// [Sealed](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed).
pub trait Answers<O, E>: Sized {
    #[doc(hidden)]
    fn erase(parsed: impl Fn(&RawValue) -> Result<Self, Kind> + Send + Sync + 'static) -> Erased;
}

impl<Fut, T, E> Answers<Once<T>, E> for Fut
where
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Serialize + 'static,
    E: Serialize + 'static,
{
    fn erase(parsed: impl Fn(&RawValue) -> Result<Fut, Kind> + Send + Sync + 'static) -> Erased {
        Erased::Call(Box::new(move |request| match parsed(request) {
            Ok(future) => future.map(answer).boxed(),
            Err(reject) => ready(reject).boxed(),
        }))
    }
}

impl<St, T, E> Answers<Strand<T>, E> for St
where
    St: Stream<Item = Result<T, E>> + Send + 'static,
    T: Serialize + 'static,
    E: Serialize + 'static,
{
    fn erase(parsed: impl Fn(&RawValue) -> Result<St, Kind> + Send + Sync + 'static) -> Erased {
        Erased::Strand(Box::new(move |request| match parsed(request) {
            Ok(stream) => stream.map(answer).boxed(),
            Err(reject) => stream::once(ready(reject)).boxed(),
        }))
    }
}

mod sealed {
    //! [Seals] [`Answers`](super::Answers) through `erase`'s return type.
    //!
    //! [Seals]: https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed

    use alloc::boxed::Box;
    use core::fmt;

    use futures_util::{future::BoxFuture, stream::BoxStream};
    use serde_json::value::RawValue;

    use crate::rpc::wire::Kind;

    /// A handler and its state, ready for the router.
    pub enum Erased {
        Call(OnRequest<BoxFuture<'static, Kind>>),
        Strand(OnRequest<BoxStream<'static, Kind>>),
    }

    /// Parses a request and starts its handler, so the answer borrows nothing.
    type OnRequest<A> = Box<dyn Fn(&RawValue) -> A + Send + Sync>;

    impl fmt::Debug for Erased {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Erased").finish_non_exhaustive()
        }
    }
}

/// A handler's result as its frame, rejected when serde can't encode it.
fn answer<T: Serialize, E: Serialize>(result: Result<T, E>) -> Kind {
    let encoded = match result {
        Ok(output) => to_raw_value(&output).map(Kind::Value),
        Err(error) => to_raw_value(&error).map(Kind::Error),
    };
    encoded.unwrap_or_else(Kind::unencodable)
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
    use alloc::vec::Vec;

    use futures_util::{FutureExt, StreamExt};
    use serde_json::value::RawValue;

    use self::idle_game::{Buy, Game, Ore, Sell, buy, hire, ore, sell};
    use super::Procedures;

    mod idle_game {
        //! Example application for testing.
        // used by specta's derives, root crate is no_std
        extern crate std;

        use alloc::string::String;
        use std::vec;

        use futures_util::{Stream, stream};
        use serde::{Deserialize, Serialize};
        use specta::Type;

        use crate::rpc::{Never, Procedure, State, Strand};

        #[derive(Clone)]
        pub(super) struct Game {
            pub(super) drills: u32,
        }

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

        pub(super) async fn buy(
            State(game): State<Game>,
            Buy { generator }: Buy,
        ) -> Result<u32, BuyError> {
            match generator.as_str() {
                "drill" => Ok(game.drills + 1),
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

        pub(super) async fn sell(State(game): State<Game>, _: Sell) -> Result<u32, Never> {
            Ok(game.drills)
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

        #[derive(Deserialize, Type)]
        pub(super) struct Ore;

        impl Procedure for Ore {
            type Output = Strand<u32>;
            type Error = Never;
        }

        pub(super) fn ore(
            State(game): State<Game>,
            _: Ore,
        ) -> impl Stream<Item = Result<u32, Never>> {
            stream::iter([Ok(game.drills), Ok(game.drills * 2)])
        }
    }

    #[test]
    fn every_answer_kind() {
        let procedures = Procedures::new().declare::<Buy>().declare::<Sell>().declare::<Ore>();
        let procedures = procedures.with_state(Game { drills: 3 }).handle(buy).handle(ore);
        let router = procedures.handle_with(Game { drills: 9 }, sell).build().unwrap();
        let calls = [
            ("taba::rpc::router::tests::idle_game::Buy", r#"{"generator":"drill"}"#),
            ("taba::rpc::router::tests::idle_game::Buy", r#"{"generator":"furnace"}"#),
            ("taba::rpc::router::tests::idle_game::Buy", r#"{"generatr":"drill"}"#),
            ("taba::rpc::router::tests::idle_game::Sell", "null"),
            ("taba::rpc::router::tests::idle_game::Hire", "null"),
            ("taba::rpc::router::tests::idle_game::Ore", "null"),
        ];
        let answers = calls.map(|(name, request)| {
            let request = RawValue::from_string(request.into()).unwrap();
            let answer = router.call(name, &request).now_or_never().unwrap();
            serde_json::to_string(&answer).unwrap()
        });
        insta::assert_snapshot!(answers.join("\n"), @r#"
        {"kind":"value","value":4}
        {"kind":"error","value":{"CantAfford":{"missing":30}}}
        {"kind":"reject","value":{"kind":"invalid","value":"missing field `generator` at line 1 column 20"}}
        {"kind":"value","value":9}
        {"kind":"reject","value":{"kind":"unknown","value":"taba::rpc::router::tests::idle_game::Hire"}}
        {"kind":"reject","value":{"kind":"unknown","value":"taba::rpc::router::tests::idle_game::Ore"}}
        "#);

        let subscriptions = [
            ("taba::rpc::router::tests::idle_game::Ore", "null"),
            ("taba::rpc::router::tests::idle_game::Ore", "7"),
            ("taba::rpc::router::tests::idle_game::Buy", r#"{"generator":"drill"}"#),
        ];
        let strands = subscriptions.map(|(name, request)| {
            let request = RawValue::from_string(request.into()).unwrap();
            let frames: Vec<_> = router.subscribe(name, &request).collect().now_or_never().unwrap();
            serde_json::to_string(&frames).unwrap()
        });
        insta::assert_snapshot!(strands.join("\n"), @r#"
        [{"kind":"value","value":3},{"kind":"value","value":6}]
        [{"kind":"reject","value":{"kind":"invalid","value":"invalid type: integer `7`, expected unit struct Ore at line 1 column 1"}}]
        [{"kind":"reject","value":{"kind":"unknown","value":"taba::rpc::router::tests::idle_game::Buy"}}]
        "#);
    }

    #[test]
    fn every_build_problem() {
        let procedures = Procedures::new().declare::<Buy>().declare::<Sell>();
        let handlers = procedures.with_state(Game { drills: 3 });
        let error = handlers.handle(buy).handle(buy).handle(hire).build().unwrap_err();
        insta::assert_snapshot!(error, @"
        can't build the router:
        \t2 handlers for `taba::rpc::router::tests::idle_game::Buy`
        \ta handler for `taba::rpc::router::tests::idle_game::Hire`, which isn't declared
        \tno handler for `taba::rpc::router::tests::idle_game::Sell`
        ");
    }
}
