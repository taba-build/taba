//! What rustc tells an app whose handler doesn't fit its procedure.

#[test]
#[cfg(feature = "rpc")]
fn handler_mistakes_name_their_cause() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
