//! Sessions, as a backend reaches them.

use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, sync::Arc};
use core::{any::Any, panic::AssertUnwindSafe, pin::pin};
use std::sync::{Mutex, MutexGuard};

use futures_util::{FutureExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Handle,
    sync::{
        Semaphore,
        mpsc::{self, Receiver, Sender},
    },
};
use tokio_util::task::JoinMap;

use super::{
    Error, Router,
    wire::{Batch, Frame, Kind, Message, Rejection, SafeInteger},
};

/// A session's id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SessionId(SafeInteger);

/// Sessions, each at an address, answered by one [`Router`].
///
/// A backend hands it each connect, send and poll, and decides when an address's sessions end.
///
/// Session ids count up to JavaScript's <code>2<sup>53</sup> - 1</code> (9 quadrillion). After
/// that many connects, ids stop changing, and each new session replaces the last. At a million
/// connects a second, that's about 285 years.
#[derive(Debug)]
pub struct Sessions {
    inner: Mutex<Inner>,
    router: Arc<Router>,
    runtime: Handle,
}

impl Sessions {
    /// Sessions whose handlers run on `runtime`.
    pub fn new(router: Router, runtime: Handle) -> Self {
        Self { inner: Mutex::default(), router: Arc::new(router), runtime }
    }

    /// Starts a session at `address`.
    pub fn connect(&self, address: impl Into<String>) -> SessionId {
        let address = address.into();
        self.lock().connect(address)
    }

    /// Applies a batch of messages, in the order sent.
    ///
    /// # Errors
    ///
    /// The session has ended, or `body` is malformed for send.
    pub fn send(&self, address: &str, body: &[u8]) -> Result<(), Error> {
        let Batch(id, messages) = serde_json::from_slice(body).map_err(Error::malformed)?;
        let mut inner = self.lock();
        let session = inner.get(address, id)?;
        session.reap();
        for message in messages {
            session.apply(&self.router, &self.runtime, message);
        }
        Ok(())
    }

    /// The session's next frame, once there is one.
    ///
    /// # Errors
    ///
    /// The session has ended, or `body` isn't a poll.
    pub fn poll(
        &self,
        address: &str,
        body: &[u8],
    ) -> impl Future<Output = Result<Frame, Error>> + Send + use<> {
        let poll = serde_json::from_slice(body).map_err(Error::malformed).and_then(|id| {
            let mut inner = self.lock();
            let session = inner.get(address, id)?;
            Ok(Arc::clone(&session.poll))
        });

        async move { poll?.lock().await.recv().await.ok_or_else(Error::ended) }
    }

    /// Ends every session at `address`, aborting their calls and subscriptions.
    pub fn end(&self, address: &str) {
        self.lock().end(address);
    }

    /// This lock is internal to taba, handlers run code in their own tasks.
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap()
    }
}

/// Sessions by id.
#[derive(Debug, Default)]
struct Inner {
    by_id: BTreeMap<SessionId, Session>,

    /// The id the next connect gets.
    next: SafeInteger,
}

impl Inner {
    /// Starts a session at `address`.
    fn connect(&mut self, address: String) -> SessionId {
        let id = SessionId(self.next);
        self.next = self.next.saturating_next();
        self.by_id.insert(id, Session::new(address));
        id
    }

    /// The session `id` names at `address`, or [`Error::ended`].
    ///
    /// This prevents a request from being able to look at other sessions.
    fn get(&mut self, address: &str, id: SessionId) -> Result<&mut Session, Error> {
        let session = self.by_id.get_mut(&id).filter(|session| session.address == address);
        session.ok_or_else(Error::ended)
    }

    fn end(&mut self, address: &str) {
        self.by_id.retain(|_, session| session.address != address);
    }
}

/// What one session owns.
///
/// Its own mailbox gives an old session's frames no route to a new one.
#[derive(Debug)]
struct Session {
    /// The only address whose requests reach it.
    address: String,

    /// Tasks post frames to it, and a poll takes them from `poll`.
    mailbox: Sender<Frame>,

    /// How many `next`s each running strand hasn't answered yet.
    nexts: BTreeMap<SafeInteger, Arc<Semaphore>>,

    /// Held across a poll's wait, so a second poll waits its turn.
    poll: Arc<tokio::sync::Mutex<Receiver<Frame>>>,

    /// Dropping it aborts every call and subscription still running.
    tasks: JoinMap<SafeInteger, ()>,
}

impl Session {
    /// A session at `address`.
    ///
    /// Its mailbox holds one frame, so a task posting waits until a poll takes the last one.
    fn new(address: String) -> Self {
        let (mailbox, poll) = mpsc::channel(1);
        let poll = Arc::new(tokio::sync::Mutex::new(poll));
        Self { address, mailbox, poll, tasks: JoinMap::new(), nexts: BTreeMap::new() }
    }

    /// Forgets each finished task and its `next`s.
    fn reap(&mut self) {
        while let Some((id, _)) = self.tasks.try_join_next() {
            self.nexts.remove(&id);
        }
    }

    /// Starts, pulls or cancels one request.
    fn apply(&mut self, router: &Arc<Router>, runtime: &Handle, message: Message<'_>) {
        match message {
            Message::Call(id, name, request) => {
                let router = Arc::clone(router);
                let name = name.to_owned();
                let request = request.to_owned();
                let reply = self.reply(id);
                let task = guarded(reply.clone(), async move {
                    reply.post(router.call(&name, &request).await).await;
                });
                self.tasks.spawn_on(id, task, runtime);
            }
            Message::Subscribe(id, name, request) => {
                let router = Arc::clone(router);
                let name = name.to_owned();
                let request = request.to_owned();

                // handler lazily starts at first `next`
                let frames = async move { router.subscribe(&name, &request) }.flatten_stream();

                let nexts = Arc::new(Semaphore::new(0));
                let reply = self.reply(id);
                let task = guarded(reply.clone(), deliver(reply, frames, Arc::clone(&nexts)));
                self.tasks.spawn_on(id, task, runtime);
                self.nexts.insert(id, nexts);
            }
            Message::Next(id) => {
                if let Some(nexts) = self.nexts.get(&id) {
                    nexts.add_permits(1);
                }
            }
            Message::Cancel(id) => {
                self.tasks.abort(&id);
                self.nexts.remove(&id);
            }
        }
    }

    fn reply(&self, id: SafeInteger) -> Reply {
        Reply { id, mailbox: self.mailbox.clone() }
    }
}

/// Where one request's frames go.
#[derive(Clone)]
struct Reply {
    id: SafeInteger,
    mailbox: Sender<Frame>,
}

impl Reply {
    /// Waits for room in the mailbox, then posts `kind` under this request's id.
    async fn post(&self, kind: Kind) {
        // send fails only once the session ends and drops the receiver
        let _ = self.mailbox.send(Frame { id: self.id, kind }).await;
    }
}

/// Runs one request's task, rejecting the request if the app's code panics.
async fn guarded(reply: Reply, task: impl Future<Output = ()>) {
    if let Err(payload) = AssertUnwindSafe(task).catch_unwind().await {
        let message = panic_message(&*payload).to_owned();
        reply.post(Kind::Reject(Rejection::Panic(message))).await;
    }
}

/// Taken from `std::panicking::payload_as_str`.
fn panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else {
        "Box<dyn Any>"
    }
}

/// Answers each `next` with one frame, until the strand's first frame that isn't a value.
async fn deliver(reply: Reply, frames: impl Stream<Item = Kind>, nexts: Arc<Semaphore>) {
    let mut frames = pin!(frames);
    while let Ok(owed) = nexts.acquire().await {
        owed.forget();
        let kind = frames.next().await.unwrap_or(Kind::End);
        let is_last = !matches!(kind, Kind::Value(_));
        reply.post(kind).await;
        if is_last {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::{String, ToString};
    use core::future::ready;

    use futures_util::stream::{self, Empty};
    use serde_json::{Value, json};
    use tokio::runtime::{Builder, Runtime};

    use super::{SessionId, Sessions};
    use crate::rpc::{
        Error, Never, Procedures, Router,
        tests::{Buy, Ore},
    };

    /// A test backend over [`Sessions`], on a runtime the test drives.
    struct Backend {
        runtime: Runtime,
        sessions: Sessions,
    }

    impl Backend {
        fn new(router: Router) -> Self {
            let runtime = Builder::new_current_thread().build().unwrap();
            let sessions = Sessions::new(router, runtime.handle().clone());
            Self { runtime, sessions }
        }

        /// Sends `batch` as `id`'s session at `address`.
        fn send(&self, address: &str, id: SessionId, batch: Value) -> Result<(), Error> {
            self.sessions.send(address, &serde_json::to_vec(&json!([id, batch])).unwrap())
        }

        /// The next frame for `id`'s session at `address` as JSON, or why there's none.
        fn poll(&self, address: &str, id: SessionId) -> String {
            let poll = self.sessions.poll(address, &serde_json::to_vec(&id).unwrap());
            match self.runtime.block_on(poll) {
                Ok(frame) => serde_json::to_string(&frame).unwrap(),
                Err(error) => error.to_string(),
            }
        }
    }

    #[test]
    fn a_call_answers_under_its_id() {
        let buy = |_: Buy| ready(Ok(3));
        let backend = Backend::new(Procedures::new().declare::<Buy>().handle(buy).build().unwrap());
        let id = backend.sessions.connect("main");
        let call = json!([{ "kind": "call", "value": [42, "taba::rpc::tests::Buy", { "generator": "Drill" }] }]);
        backend.send("main", id, call).unwrap();
        insta::assert_snapshot!(backend.poll("main", id), @r#"{"id":42,"kind":"value","value":3}"#);
    }

    #[test]
    fn a_strand_waits_for_each_next_until_it_ends() {
        let ore = |_: Ore| stream::iter([Ok(3), Ok(6)]);
        let buy = |_: Buy| ready(Ok(10));
        let procedures = Procedures::new().declare::<Ore>().declare::<Buy>();
        let backend = Backend::new(procedures.handle(ore).handle(buy).build().unwrap());
        let id = backend.sessions.connect("main");
        let batches = [
            json!([
                { "kind": "subscribe", "value": [0, "taba::rpc::tests::Ore", null] },
                { "kind": "call", "value": [1, "taba::rpc::tests::Buy", { "generator": "Drill" }] },
            ]),
            json!([
                { "kind": "next", "value": 0 },
                { "kind": "call", "value": [2, "taba::rpc::tests::Buy", { "generator": "Drill" }] },
            ]),
            json!([]),
            json!([{ "kind": "next", "value": 0 }]),
            json!([{ "kind": "next", "value": 0 }]),
        ];
        let frames = batches.map(|batch| {
            backend.send("main", id, batch).unwrap();
            backend.poll("main", id)
        });
        insta::assert_snapshot!(frames.join("\n"), @r#"
        {"id":1,"kind":"value","value":10}
        {"id":0,"kind":"value","value":3}
        {"id":2,"kind":"value","value":10}
        {"id":0,"kind":"value","value":6}
        {"id":0,"kind":"end"}
        "#);
    }

    #[test]
    fn a_strand_answers_every_next() {
        let ore = |_: Ore| stream::iter([Ok(3), Ok(6)]);
        let buy = |_: Buy| ready(Ok(10));
        let procedures = Procedures::new().declare::<Ore>().declare::<Buy>();
        let backend = Backend::new(procedures.handle(ore).handle(buy).build().unwrap());
        let id = backend.sessions.connect("main");
        let batch = json!([
            { "kind": "subscribe", "value": [0, "taba::rpc::tests::Ore", null] },
            { "kind": "next", "value": 0 },
            { "kind": "next", "value": 0 },
            { "kind": "call", "value": [1, "taba::rpc::tests::Buy", { "generator": "Drill" }] },
        ]);
        backend.send("main", id, batch).unwrap();
        let frames = [backend.poll("main", id), backend.poll("main", id)];
        insta::assert_snapshot!(frames.join("\n"), @r#"
        {"id":0,"kind":"value","value":3}
        {"id":0,"kind":"value","value":6}
        "#);
    }

    #[test]
    fn a_strand_starts_at_its_first_next() {
        let ore = |_: Ore| -> Empty<Result<u32, Never>> { panic!("the mine flooded") };
        let buy = |_: Buy| ready(Ok(10));
        let procedures = Procedures::new().declare::<Ore>().declare::<Buy>();
        let backend = Backend::new(procedures.handle(ore).handle(buy).build().unwrap());
        let id = backend.sessions.connect("main");
        let batches = [
            json!([
                { "kind": "subscribe", "value": [0, "taba::rpc::tests::Ore", null] },
                { "kind": "call", "value": [1, "taba::rpc::tests::Buy", { "generator": "Drill" }] },
            ]),
            json!([{ "kind": "next", "value": 0 }]),
        ];
        let frames = batches.map(|batch| {
            backend.send("main", id, batch).unwrap();
            backend.poll("main", id)
        });
        insta::assert_snapshot!(frames.join("\n"), @r#"
        {"id":1,"kind":"value","value":10}
        {"id":0,"kind":"reject","value":{"kind":"panic","value":"the mine flooded"}}
        "#);
    }

    #[test]
    fn a_cancel_stops_its_strand() {
        let ore = |_: Ore| stream::iter([Ok(3), Ok(6)]);
        let buy = |_: Buy| ready(Ok(10));
        let procedures = Procedures::new().declare::<Ore>().declare::<Buy>();
        let backend = Backend::new(procedures.handle(ore).handle(buy).build().unwrap());
        let id = backend.sessions.connect("main");
        let batch = json!([
            { "kind": "subscribe", "value": [0, "taba::rpc::tests::Ore", null] },
            { "kind": "cancel", "value": 0 },
            { "kind": "next", "value": 0 },
            { "kind": "call", "value": [1, "taba::rpc::tests::Buy", { "generator": "Drill" }] },
        ]);
        backend.send("main", id, batch).unwrap();
        insta::assert_snapshot!(backend.poll("main", id), @r#"{"id":1,"kind":"value","value":10}"#);
    }

    #[test]
    fn a_panic_rejects_only_its_request() {
        let depth = 3;
        let ore = move |_: Ore| -> Empty<Result<u32, Never>> {
            panic!("the mine flooded at depth {depth}")
        };
        let buy = |_: Buy| ready(Ok(10));
        let procedures = Procedures::new().declare::<Ore>().declare::<Buy>();
        let backend = Backend::new(procedures.handle(ore).handle(buy).build().unwrap());
        let id = backend.sessions.connect("main");
        let batches = [
            json!([
                { "kind": "subscribe", "value": [0, "taba::rpc::tests::Ore", null] },
                { "kind": "next", "value": 0 },
            ]),
            json!([{ "kind": "call", "value": [1, "taba::rpc::tests::Buy", { "generator": "Drill" }] }]),
        ];
        let frames = batches.map(|batch| {
            backend.send("main", id, batch).unwrap();
            backend.poll("main", id)
        });
        insta::assert_snapshot!(frames.join("\n"), @r#"
        {"id":0,"kind":"reject","value":{"kind":"panic","value":"the mine flooded at depth 3"}}
        {"id":1,"kind":"value","value":10}
        "#);
    }

    #[test]
    fn an_ended_session_refuses_its_requests() {
        let backend = Backend::new(Procedures::new().build().unwrap());
        let old = backend.sessions.connect("main");
        let held = backend.sessions.poll("main", &serde_json::to_vec(&old).unwrap());
        backend.sessions.end("main");
        backend.sessions.connect("main");
        let refused = [
            backend.send("main", old, json!([])).unwrap_err().to_string(),
            backend.runtime.block_on(held).unwrap_err().to_string(),
            backend.poll("main", old),
        ];
        insta::assert_snapshot!(refused.join("\n"), @"
        this session has ended
        this session has ended
        this session has ended
        ");
    }

    #[test]
    fn a_session_answers_only_its_address() {
        let buy = |_: Buy| ready(Ok(10));
        let backend = Backend::new(Procedures::new().declare::<Buy>().handle(buy).build().unwrap());
        let id = backend.sessions.connect("main");
        let call = json!([{ "kind": "call", "value": [0, "taba::rpc::tests::Buy", { "generator": "Drill" }] }]);
        let elsewhere = backend.send("other", id, call.clone()).unwrap_err().to_string();
        backend.send("main", id, call).unwrap();
        let answered = backend.poll("main", id);
        insta::assert_snapshot!([elsewhere, answered].join("\n"), @r#"
        this session has ended
        {"id":0,"kind":"value","value":10}
        "#);
    }

    #[test]
    fn a_malformed_body_says_why() {
        let backend = Backend::new(Procedures::new().build().unwrap());
        let error = backend.sessions.send("main", br#"{"session":0,"batch":[]}"#).unwrap_err();
        insta::assert_snapshot!(error, @"can't read what was sent: invalid type: map, expected tuple struct Batch at line 1 column 0");
    }
}
