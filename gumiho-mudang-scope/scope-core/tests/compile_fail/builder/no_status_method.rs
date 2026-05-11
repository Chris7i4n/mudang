// Builder forbids status (R1): `EdgeBuilder` exposes no `.status(...)`
// method. Status is the resolver's output (R3), never the extractor's.

use scope_core::{Confidence, Edge, EdgeKind, Producer};

fn main() {
    let _ = Edge::builder()
        .from("a")
        .to("b")
        .kind(EdgeKind::Calls)
        .confidence(Confidence::High)
        .producer(Producer::Lang("rust_lang".into()))
        .pattern_id("calls.method")
        // .status(...) does not exist on any EdgeBuilder state.
        .status("resolved")
        .build();
}
