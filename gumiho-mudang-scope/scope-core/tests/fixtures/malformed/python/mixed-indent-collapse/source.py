def preamble_probe():
    return "ok"


class Inventory:
    def __init__(self):
        self.count = 0

    def restock(self, amount):
        if amount > 0:
            self.count += amount
        return self.count
        ?? broken_indent_token ??

    def drain(self):
        self.count = 0
