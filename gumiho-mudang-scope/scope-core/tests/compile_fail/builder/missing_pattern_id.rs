// Builder requires fields (R1): omitting `.pattern_id(...)` must
// prevent `.build()` from compiling.

use scope_core::{Confidence, EdgeKind, Producer, RawEdge};

fn main() {
    let _ = RawEdge::builder()
        .from("a")
        .to("b")
        .kind(EdgeKind::Calls)
        .confidence(Confidence::High)
        .producer(Producer::Lang("rust_lang".into()))
        .build();
}
