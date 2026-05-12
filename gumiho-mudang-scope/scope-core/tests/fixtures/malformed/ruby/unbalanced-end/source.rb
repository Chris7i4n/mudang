module Acme
  class Inventory
    attr_accessor :count

    def restock(amount)
      @count += amount

    def drain
      @count = 0
    end
  end
end
