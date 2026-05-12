namespace Acme.Example;

public class PreambleProbe
{
    public string Ping() => "ok";
}

public class TemplateLoader
{
    public string Template { get; } = @"
        <html>
            <body>
                <p>Hello</p>
            </body>
        </html>

    public void Reset() { Template = null; }
}
