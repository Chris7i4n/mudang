module Acme
  class OrderProcessor
    attr_accessor :total

    def compute_total(items)
      sum = 0
      items.each do |item|
        sum +=
