// Builder requires fields (R1): omitting `.producer(...)` must
// prevent `.build()` from compiling.

use scope_core::{Confidence, EdgeKind, RawEdge};

fn main() {
    let _ = RawEdge::builder()
        .from("a")
        .to("b")
        .kind(EdgeKind::Calls)
        .confidence(Confidence::High)
        .pattern_id("calls.method")
        .build();
}
