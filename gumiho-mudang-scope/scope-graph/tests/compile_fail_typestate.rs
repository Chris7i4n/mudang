//! R3 CI gate harness — `Insertable typestate`.
//!
//! Drives `trybuild` over fixtures in `tests/compile_fail/typestate/`.
//! Each fixture must fail to compile; trybuild records the rustc
//! stderr as the test signal. See `docs/CI-GATES.md` § Insertable
//! typestate and `docs/ARCHITECTURAL-REFACTOR.md` § R3 ("Resolver
//! location") for the contract:
//!
//! - `Graph::insert_*` accepts only `InsertableEdge`; passing a
//!   `RawEdge` is a compile error (R3 acceptance bullet 1).
//! - `InsertableEdge::new` is module-private to `scope_graph::resolve`;
//!   calling it from outside is a compile error (R3 acceptance
//!   bullet 2 — "constructing an `InsertableEdge` outside the
//!   resolver module does not compile").
//! - `InsertableEdge`'s fields are private; the struct-literal form
//!   `InsertableEdge { raw, status }` is also rejected (same
//!   acceptance bullet, via a complementary failure mode).

#[test]
fn compile_fail_typestate() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/typestate/insert_raw_edge.rs");
    t.compile_fail("tests/compile_fail/typestate/insertable_new_is_private.rs");
    t.compile_fail("tests/compile_fail/typestate/insertable_fields_private.rs");
}
