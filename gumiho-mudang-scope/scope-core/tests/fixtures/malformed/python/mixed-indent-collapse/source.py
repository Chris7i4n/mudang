class Inventory:
    def __init__(self):
        self.count = 0

    def restock(self, amount):
        self.count += amount
       return self.count

    def drain(self):
        self.count = 0
