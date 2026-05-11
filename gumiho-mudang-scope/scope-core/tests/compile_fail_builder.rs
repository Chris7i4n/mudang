//! R1 CI gate harness — `Builder requires fields` and `Builder forbids status`.
//!
//! Drives `trybuild` over the fixtures in `tests/compile_fail/builder/`.
//! Each fixture must fail to compile; trybuild records the rustc stderr
//! as the test signal. See `CI-GATES.md` § Builder requires fields and
//! § Builder forbids status.

#[test]
fn compile_fail_builder() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/builder/missing_confidence.rs");
    t.compile_fail("tests/compile_fail/builder/missing_producer.rs");
    t.compile_fail("tests/compile_fail/builder/missing_pattern_id.rs");
    t.compile_fail("tests/compile_fail/builder/no_status_method.rs");
}
