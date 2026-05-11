// Builder requires fields (R1): omitting `.producer(...)` must
// prevent `.build()` from compiling.

use scope_core::{Confidence, Edge, EdgeKind};

fn main() {
    let _ = Edge::builder()
        .from("a")
        .to("b")
        .kind(EdgeKind::Calls)
        .confidence(Confidence::High)
        .pattern_id("calls.method")
        .build();
}
