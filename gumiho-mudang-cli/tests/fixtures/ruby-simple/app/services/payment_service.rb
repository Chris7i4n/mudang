require "json"
require_relative "../../lib/auditable"

autoload :PaymentResult, "payment_result"

class BaseGateway
end

# Handles payment workflows for checkout requests.
class PaymentService < BaseGateway
  include Auditable

  DEFAULT_CURRENCY = "USD"

  normalize_amount = ->(amount) { amount.to_i }
  log_payment = proc { |payment_id| payment_id.to_s }

  def initialize(logger = nil, _unused = nil)
    @logger = logger
  end

  # Runs payment processing and yields the normalized request when needed.
  def process_payment(request, retry_count: 0, **options, &block)
    validate_card(request)
    normalize_amount = request.to_i
    result = PaymentResult.new
    @logger&.info("processing payment") if @logger
    audit!
    send(:audit!)
    public_send("paid?")
    public_send("process_payment") if retry_count.negative?
    dynamic_target = options[:dynamic_target]
    send(dynamic_target) if dynamic_target
    block.call(normalize_amount) if block
    yield(result) if block_given?
    "#{normalize_amount} #{retry_count} #{options[:currency]}"
  end

  def paid?
    false
  end

  def settle!
    true
  end

  def status=(value)
    @status = value
  end

  def [](key)
    key
  end

  def []=(key, value)
    @values ||= {}
    @values[key] = value
  end

  def currency = DEFAULT_CURRENCY

  def self.build(client:, logger:)
    new
  end

  private

  def validate_card(card)
    card
  end

  class << self
    def default_currency
      DEFAULT_CURRENCY
    end
  end
end

class PaymentResult
end

class Payments::Gateway < BaseGateway
end
