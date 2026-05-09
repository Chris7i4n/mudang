describe "payment" do
  it "charges" do
    PaymentService.new.process_payment
  end
end
