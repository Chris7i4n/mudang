// R3 Insertable typestate: `InsertableEdge`'s fields are private.
// The struct-literal construction form is rejected by visibility,
// closing the loophole of bypassing the module-private `new`
// constructor.

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

    // Private fields — must not compile.
    let _ = InsertableEdge {
        raw,
        status: Status::Resolved,
    };
}
