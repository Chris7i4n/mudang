module Auditable
  def audit!
    true
  end
end

module Trackable
  include Auditable

  def track!
    audit!
  end
end
