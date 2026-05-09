class TypeRefHolder
  TARGET = "x"

  class TaggedValue
  end

  class Tag
  end

  def lookup(header)
    TaggedValue[header.to_s]
  end

  def detect(value)
    if value.is_a?(TaggedValue)
      :tagged
    end
  end

  def classify(value)
    case value
    when Tag
      :tag
    when TaggedValue
      :tagged
    else
      :other
    end
  end
end
