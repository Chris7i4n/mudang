package com.acme.example;

import com.acme.routes.Route;

class PreambleProbe {
    String ping() { return "ok"; }
}

public class Endpoints {
    @Route(path = "/health", methods = {"GET",
    public String health() { return "ok"; }

    @Route(path = "/version")
    public String version() { return "1.0"; }
}
