module Acme
  class TemplateLoader
    def template
      <<~HTML
        <html>
          <body>
            <p>Hello</p>
          </body>
        </html>

    def reset
      nil
    end
  end
end
