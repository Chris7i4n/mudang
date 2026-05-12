pub struct Inventory {
    pub count: i64,
}

impl Inventory {
    pub fn restock(&mut self, amount: i64) {
        self.count += amount;

    pub fn drain(&mut self) {
        self.count = 0;
    }
}
