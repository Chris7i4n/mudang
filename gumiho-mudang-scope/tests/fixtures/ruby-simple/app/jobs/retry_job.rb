class RetryJob
  def call
    PaymentService.new.process_payment
  end
end

class WebhookJob
  def call
    PaymentService.build(client: nil, logger: Logger.new)
  end
end
