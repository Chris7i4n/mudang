// R3 Insertable typestate: `InsertableEdge::new` is module-private to
// `scope_graph::resolve`. Any caller outside that module — including
// integration tests living in the `scope-graph` test crate — must be
// rejected by visibility.

use scope_core::{Confidence, EdgeKind, Producer, RawEdge, Status};
use scope_graph::resolve::InsertableEdge;

fn main() {
    let raw: RawEdge = RawEdge::builder()
        .from("a")
        .to("b")
        .kind(EdgeKind::Calls)
        .confidence(Confidence::High)
        .producer(Producer::Lang("rust_lang".into()))
        .pattern_id("calls.method")
        .build();

    // Module-private constructor — must not compile.
    let _ = InsertableEdge::new(raw, Status::Resolved);
}
