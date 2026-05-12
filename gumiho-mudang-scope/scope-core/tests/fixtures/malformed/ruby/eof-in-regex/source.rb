module Acme
  class Matcher
    def find_first(text)
      text.match(/welcome to [Aa]cme
    end

    def find_all(text)
      text.scan(/acme/i)
    end
  end
end
