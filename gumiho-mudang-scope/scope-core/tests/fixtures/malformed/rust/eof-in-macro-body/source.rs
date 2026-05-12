pub fn build_router() {
    routes! {
        GET "/health" => health_handler,
        GET "/version" => version_handler,
        POST "/restart" => restart_handler,

pub fn health_handler() -> &'static str {
    "ok"
}
