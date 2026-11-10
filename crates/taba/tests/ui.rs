//! What rustc tells an app whose handler doesn't fit its procedure.

#[cfg(feature = "rpc")]
#[rustversion::attr(not(beta), ignore = "requires beta")]
#[test]
fn handler_mistakes_name_their_cause() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
