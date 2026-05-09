module DupHelper
end

class DupConsumer
  include DupHelper
  include DupHelper
  prepend DupHelper
end
