//! Taba's Tauri backend

use std::{borrow::Cow, fmt, sync::Arc};

use serde_core::Deserialize as _;
use taba::rpc::{Router, Sessions};
use tauri::{
    App, Builder, Context, Runtime, WindowEvent, async_runtime,
    ipc::{Invoke, InvokeBody, InvokeError},
};

/// A [`tauri::Builder`] answering Taba, wrapped to protect the invoke handler.
///
/// This overwrites any previous `invoke_handler`, so all previous commands reject as not found.
/// If you want those commands handled, pass them to [`WithTaba::with_fallback`] instead.
pub struct WithTaba<R: Runtime>(Builder<R>);

impl<R: Runtime> WithTaba<R> {
    /// Answers only Taba commands, with `router`.
    pub fn new(builder: Builder<R>, router: Router) -> Self {
        Self::with_fallback(builder, router, |_| false)
    }

    /// Answers Taba commands with `router`, otherwise passing through to `fallback`.
    pub fn with_fallback<F>(builder: Builder<R>, router: Router, fallback: F) -> Self
    where
        F: Fn(Invoke<R>) -> bool + Send + Sync + 'static,
    {
        let runtime = async_runtime::handle().inner().clone();
        let sessions = Arc::new(Sessions::new(router, runtime));
        let destroyed = Arc::clone(&sessions);
        let builder = builder
            .on_window_event(move |window, event| {
                if let WindowEvent::Destroyed = event {
                    destroyed.end(window.label());
                }
            })
            .invoke_handler(handler(sessions, fallback));
        Self(builder)
    }

    /// [`tauri::Builder::run`].
    ///
    /// # Errors
    ///
    /// The app failed to build or run.
    pub fn run(self, context: Context<R>) -> tauri::Result<()> {
        self.0.run(context)
    }

    /// [`tauri::Builder::build`].
    ///
    /// # Errors
    ///
    /// The app failed to build.
    pub fn build(self, context: Context<R>) -> tauri::Result<App<R>> {
        self.0.build(context)
    }
}

impl<R: Runtime> fmt::Debug for WithTaba<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WithTaba").finish_non_exhaustive()
    }
}

/// Helpers for [`WithTaba`].
pub trait BuilderExt<R: Runtime> {
    /// [`WithTaba::new`].
    fn taba(self, router: Router) -> WithTaba<R>;

    /// [`WithTaba::with_fallback`].
    fn taba_with_fallback<F>(self, router: Router, fallback: F) -> WithTaba<R>
    where
        F: Fn(Invoke<R>) -> bool + Send + Sync + 'static;
}

impl<R: Runtime> BuilderExt<R> for Builder<R> {
    fn taba(self, router: Router) -> WithTaba<R> {
        WithTaba::new(self, router)
    }

    fn taba_with_fallback<F>(self, router: Router, fallback: F) -> WithTaba<R>
    where
        F: Fn(Invoke<R>) -> bool + Send + Sync + 'static,
    {
        WithTaba::with_fallback(self, router, fallback)
    }
}

/// Answers Taba commands from `sessions`, using webview labels for the session address.
fn handler<R: Runtime, F>(sessions: Arc<Sessions>, fallback: F) -> impl Fn(Invoke<R>) -> bool
where
    F: Fn(Invoke<R>) -> bool,
{
    move |invoke| {
        let label = invoke.message.webview_ref().label();
        match invoke.message.command() {
            "taba::connect" => {
                // a reload keeps the label
                sessions.end(label);
                invoke.resolver.resolve(sessions.connect(label));
            }
            "taba::send" => {
                let sent = bytes(invoke.message.payload())
                    .and_then(|body| sessions.send(label, &body).map_err(InvokeError::from_error));
                invoke.resolver.respond(sent);
            }
            "taba::poll" => {
                let poll = bytes(invoke.message.payload()).map(|body| sessions.poll(label, &body));
                let frame = async move { poll?.await.map_err(InvokeError::from_error) };
                invoke.resolver.respond_async(frame);
            }
            _ => return fallback(invoke),
        }
        true
    }
}

/// A `Uint8Array` body's bytes.
///
/// It arrives raw, or as a JSON array of numbers when Tauri falls back to `postMessage`.
fn bytes(body: &InvokeBody) -> Result<Cow<'_, [u8]>, InvokeError> {
    match body {
        InvokeBody::Raw(bytes) => Ok(Cow::Borrowed(bytes)),
        InvokeBody::Json(numbers) => {
            Vec::<u8>::deserialize(numbers).map(Cow::Owned).map_err(InvokeError::from_error)
        }
    }
}
