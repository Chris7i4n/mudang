class OrderController
  def checkout
    PaymentService.build(client: nil, logger: Logger.new)
    PaymentService.new.process_payment
  end
end
