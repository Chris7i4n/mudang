// Builder requires fields (R1): omitting `.confidence(...)` must
// prevent `.build()` from compiling.

use scope_core::{EdgeKind, Producer, RawEdge};

fn main() {
    // No .confidence() call — `.build()` is not implemented for
    // EdgeBuilder<_, _, _, No, _, _>.
    let _ = RawEdge::builder()
        .from("a")
        .to("b")
        .kind(EdgeKind::Calls)
        .producer(Producer::Lang("rust_lang".into()))
        .pattern_id("calls.method")
        .build();
}
