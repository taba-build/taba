//! What rustc tells an app whose handler doesn't fit its procedure, or whose type can't be one.

#[cfg(feature = "rpc")]
#[rustversion::attr(not(beta), ignore = "requires beta")]
#[test]
fn mistakes_name_their_cause() {
    trybuild::TestCases::new().compile_fail("tests/ui/**/*.rs");
}
