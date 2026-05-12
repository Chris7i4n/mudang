use std::collections::HashMap;

pub struct Registry {
    map: HashMap<String, HashMap<String,

    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }
}
