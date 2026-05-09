module Payments
  class Gateway
    def authorize(request)
      request
    end

    def ==(other)
      other.is_a?(self.class)
    end
  end
end

class Payments::RefundService
  def call(request)
    PaymentService.new.process_payment(request)
  end
end

class AdminController
  before_action :authenticate
  has_many :orders
end

class LiteralsExample
  TEMPLATE = <<~TEXT
    send(:ghost_call)
    require "ghost_dependency"
    include GhostMixin
  TEXT

  WORDS = %w[send require include]
  SYMBOLS = %i[foo bar]
  MESSAGE = %Q{public_send("ghost_message")}
  PATTERN = /send\(:regex_ghost\)/

  def render
    TEMPLATE
  end
end
