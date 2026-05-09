module Routing
  class Endpoint
  end

  class Redirect < Endpoint
  end

  class PathRedirect < Redirect
  end

  class OptionRedirect < Redirect
  end
end

class OuterA < Routing::Endpoint
end

class OuterB < Routing::Redirect
end

module Wrapper
  class Logger2
    class Formatter
    end

    class SimpleFormatter < Formatter
    end
  end

  class Application < Engine
  end

  class Engine
  end
end
