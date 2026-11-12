//! Taba's Tauri backend
#![cfg_attr(docsrs, feature(doc_cfg))]

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

#[cfg(test)]
mod tests {
    use taba::rpc::{Procedures, Router};
    use tauri::{
        App, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
        http::HeaderMap,
        ipc::{CallbackFn, Invoke, InvokeBody, InvokeResponseBody},
        test::{
            INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
        },
        webview::InvokeRequest,
    };

    use super::{BuilderExt, WithTaba};

    /// A Tauri app on Tauri's mock runtime.
    struct Mock(App<MockRuntime>);

    impl Mock {
        fn new(with_taba: WithTaba<MockRuntime>) -> Self {
            Self(with_taba.build(mock_context(noop_assets())).unwrap())
        }

        fn webview(&self, label: &str) -> Webview {
            let builder = WebviewWindowBuilder::new(&self.0, label, WebviewUrl::default());
            Webview(builder.build().unwrap())
        }
    }

    /// One webview of a [`Mock`].
    struct Webview(WebviewWindow<MockRuntime>);

    impl Webview {
        /// Invokes `command` from the webview's URL, which Tauri treats as local.
        fn invoke(&self, command: &str, body: InvokeBody) -> String {
            let request = InvokeRequest {
                cmd: command.to_owned(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: self.0.url().unwrap(),
                body,
                headers: HeaderMap::new(),
                invoke_key: INVOKE_KEY.to_owned(),
            };
            match get_ipc_response(&self.0, request) {
                Ok(InvokeResponseBody::Json(json)) => json,
                Ok(InvokeResponseBody::Raw(raw)) => String::from_utf8(raw).unwrap(),
                Err(error) => format!("rejected: {error}"),
            }
        }
    }

    fn router() -> Router {
        Procedures::new().build().unwrap()
    }

    fn raw(text: &str) -> InvokeBody {
        InvokeBody::Raw(text.as_bytes().to_vec())
    }

    #[test]
    fn a_second_connect_ends_the_first() {
        let mock = Mock::new(mock_builder().taba(router()));
        let main = mock.webview("main");
        let first = main.invoke("taba::connect", InvokeBody::default());
        let second = main.invoke("taba::connect", InvokeBody::default());
        let sends = [first, second].map(|id| main.invoke("taba::send", raw(&format!("[{id},[]]"))));
        insta::assert_snapshot!(sends.join("\n"), @r#"
        rejected: "this session has ended"
        null
        "#);
    }

    #[test]
    fn each_webview_has_its_own_sessions() {
        let mock = Mock::new(mock_builder().taba(router()));
        let main = mock.webview("main");
        let second = mock.webview("second");
        let id = main.invoke("taba::connect", InvokeBody::default());
        second.invoke("taba::connect", InvokeBody::default());
        let send = |webview: &Webview| webview.invoke("taba::send", raw(&format!("[{id},[]]")));
        insta::assert_snapshot!([send(&main), send(&second)].join("\n"), @r#"
        null
        rejected: "this session has ended"
        "#);
    }

    #[test]
    fn a_json_body_reads_as_bytes() {
        let mock = Mock::new(mock_builder().taba(router()));
        let main = mock.webview("main");
        let id = main.invoke("taba::connect", InvokeBody::default());
        let numbers = serde_json::Value::from(format!("[{id},[]]").into_bytes());
        insta::assert_snapshot!(main.invoke("taba::send", InvokeBody::Json(numbers)), @"null");
    }

    #[test]
    fn other_commands_reach_the_fallback() {
        let greet = |invoke: Invoke<MockRuntime>| match invoke.message.command() {
            "greet" => {
                invoke.resolver.resolve("hello");
                true
            }
            _ => false,
        };
        let mock = Mock::new(mock_builder().taba_with_fallback(router(), greet));
        let main = mock.webview("main");
        let answers = [
            main.invoke("greet", InvokeBody::default()),
            main.invoke("taba::connect", InvokeBody::default()),
        ];
        insta::assert_snapshot!(answers.join("\n"), @r#"
        "hello"
        0
        "#);
    }
}
