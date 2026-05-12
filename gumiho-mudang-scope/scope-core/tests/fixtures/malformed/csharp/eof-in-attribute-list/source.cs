namespace Acme.Example;

public class Endpoints
{
    [Route("/health", Methods = new[] { "GET",
    public string Health() => "ok";

    [Route("/version")]
    public string Version() => "1.0";
}
