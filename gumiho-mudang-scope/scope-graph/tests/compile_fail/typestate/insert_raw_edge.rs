// R3 Insertable typestate: the storage layer accepts only
// `InsertableEdge` (the resolver's output). Passing a `RawEdge` to
// any `Graph::insert_*` method is a compile error.

use scope_core::{Confidence, EdgeKind, Producer, RawEdge};
use scope_graph::graph::Graph;

fn main() {
    let raw: RawEdge = RawEdge::builder()
        .from("a")
        .to("b")
        .kind(EdgeKind::Calls)
        .confidence(Confidence::High)
        .producer(Producer::Lang("rust_lang".into()))
        .pattern_id("calls.method")
        .build();

    // The `todo!()` lets the compiler type-check the call site
    // without opening a real DB. The type mismatch on the slice
    // argument is what should reject this code.
    let mut g: Graph = todo!();
    let _ = g.insert_edges_for_file("src/x.rs", &[raw]);
}
