/// Integration tests for Ruby language support.
///
/// Each test copies the Ruby fixture to a temporary directory to avoid
/// modifying the committed fixture, then drives the binary via assert_cmd.
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

const RUBY_FIXTURE: &str = "tests/fixtures/ruby-simple";

fn copy_dir_all(src: &Path, dest: &Path) {
    std::fs::create_dir_all(dest).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dest_path);
        } else {
            std::fs::copy(&src_path, &dest_path).unwrap();
        }
    }
}

fn setup_ruby_fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let fixture = Path::new(RUBY_FIXTURE);
    copy_dir_all(fixture, dir.path());
    dir
}

fn setup_empty_ruby_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Gemfile"),
        "source \"https://rubygems.org\"\n",
    )
    .unwrap();
    sc_init(dir.path()).success();
    dir
}

fn sc_init(dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("scope")
        .unwrap()
        .arg("init")
        .current_dir(dir)
        .assert()
}

fn sc_index_full(dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("scope")
        .unwrap()
        .args(["index", "--full"])
        .current_dir(dir)
        .assert()
}

fn sc_sketch(dir: &Path, symbol: &str) -> assert_cmd::assert::Assert {
    Command::cargo_bin("scope")
        .unwrap()
        .args(["sketch", symbol])
        .current_dir(dir)
        .assert()
}

fn sc_find(dir: &Path, query: &str) -> assert_cmd::assert::Assert {
    Command::cargo_bin("scope")
        .unwrap()
        .args(["find", query])
        .current_dir(dir)
        .assert()
}

fn sc_refs(dir: &Path, symbol: &str) -> assert_cmd::assert::Assert {
    Command::cargo_bin("scope")
        .unwrap()
        .args(["refs", symbol])
        .current_dir(dir)
        .assert()
}

fn sc_deps(dir: &Path, symbol: &str) -> assert_cmd::assert::Assert {
    Command::cargo_bin("scope")
        .unwrap()
        .args(["deps", symbol])
        .current_dir(dir)
        .assert()
}

fn sc_impact(dir: &Path, symbol: &str) -> assert_cmd::assert::Assert {
    Command::cargo_bin("scope")
        .unwrap()
        .args(["impact", symbol])
        .current_dir(dir)
        .assert()
}

fn scope_stdout(dir: &Path, args: &[&str]) -> String {
    let assert = Command::cargo_bin("scope")
        .unwrap()
        .args(args)
        .current_dir(dir)
        .assert()
        .success();

    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

fn scope_json(dir: &Path, args: &[&str]) -> Value {
    serde_json::from_str(&scope_stdout(dir, args)).unwrap()
}

fn scope_stderr_failure(dir: &Path, args: &[&str]) -> String {
    let assert = Command::cargo_bin("scope")
        .unwrap()
        .args(args)
        .current_dir(dir)
        .assert()
        .failure();

    String::from_utf8(assert.get_output().stderr.clone()).unwrap()
}

fn indexed_ruby_fixture_db() -> (rusqlite::Connection, TempDir) {
    let dir = setup_ruby_fixture();

    sc_init(dir.path()).success();
    sc_index_full(dir.path()).success();

    let db_path = dir.path().join(".scope").join("graph.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    (conn, dir)
}

fn ruby_symbol_exists(conn: &rusqlite::Connection, name: &str, kind: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbols WHERE name = ?1 AND kind = ?2 AND language = 'ruby'",
            rusqlite::params![name, kind],
            |row| row.get(0),
        )
        .unwrap();
    count > 0
}

fn ruby_metadata(conn: &rusqlite::Connection, name: &str) -> Value {
    let metadata: String = conn
        .query_row(
            "SELECT metadata FROM symbols WHERE name = ?1 AND language = 'ruby' LIMIT 1",
            rusqlite::params![name],
            |row| row.get(0),
        )
        .unwrap();

    serde_json::from_str(&metadata).unwrap()
}

fn ruby_docstring(conn: &rusqlite::Connection, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT docstring FROM symbols WHERE name = ?1 AND language = 'ruby' LIMIT 1",
        rusqlite::params![name],
        |row| row.get(0),
    )
    .unwrap()
}

fn ruby_edge_count(conn: &rusqlite::Connection, kind: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM edges WHERE kind = ?1",
        rusqlite::params![kind],
        |row| row.get(0),
    )
    .unwrap()
}

fn ruby_edge_exists(conn: &rusqlite::Connection, kind: &str, to_id: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE kind = ?1 AND to_id = ?2",
            rusqlite::params![kind, to_id],
            |row| row.get(0),
        )
        .unwrap();
    count > 0
}

fn ruby_edge_like(conn: &rusqlite::Connection, kind: &str, to_id_like: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE kind = ?1 AND to_id LIKE ?2",
            rusqlite::params![kind, to_id_like],
            |row| row.get(0),
        )
        .unwrap();
    count > 0
}

fn ruby_file_hash_exists(conn: &rusqlite::Connection, file_path: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM file_hashes WHERE file_path = ?1",
            rusqlite::params![file_path],
            |row| row.get(0),
        )
        .unwrap();
    count > 0
}

fn ruby_symbol_count_for_file(conn: &rusqlite::Connection, file_path: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM symbols WHERE file_path = ?1",
        rusqlite::params![file_path],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn test_init_detects_ruby_from_gemfile() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Gemfile"),
        "source \"https://rubygems.org\"\n",
    )
    .unwrap();

    sc_init(dir.path()).success().stdout(contains("Ruby"));
}

#[test]
fn test_init_detects_ruby_from_rakefile() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("Rakefile"), "task default: []\n").unwrap();

    sc_init(dir.path()).success().stdout(contains("Ruby"));
}

#[test]
fn test_init_detects_ruby_from_config_ru() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("config.ru"),
        "run ->(_env) { [200, {}, []] }\n",
    )
    .unwrap();

    sc_init(dir.path()).success().stdout(contains("Ruby"));
}

#[test]
fn test_init_detects_ruby_from_ruby_version() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".ruby-version"), "3.3.0\n").unwrap();

    sc_init(dir.path()).success().stdout(contains("Ruby"));
}

#[test]
fn test_init_detects_ruby_from_gemspec() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("sample.gemspec"),
        "Gem::Specification.new do |spec|\n  spec.name = \"sample\"\nend\n",
    )
    .unwrap();

    sc_init(dir.path()).success().stdout(contains("Ruby"));
}

#[test]
fn test_index_full_on_ruby_fixture() {
    let dir = setup_ruby_fixture();

    sc_init(dir.path()).success();
    sc_index_full(dir.path())
        .success()
        .stderr(contains("files"))
        .stderr(contains("symbols"));

    let graph_db = dir.path().join(".scope").join("graph.db");
    assert!(graph_db.exists(), "graph.db should exist after indexing");
    assert!(
        graph_db.metadata().unwrap().len() > 0,
        "graph.db should not be empty"
    );
}

#[test]
fn test_index_detects_ruby_classes() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let (kind, language, metadata): (String, String, String) = conn
        .query_row(
            "SELECT kind, language, metadata FROM symbols WHERE name = ?1 LIMIT 1",
            rusqlite::params!["PaymentService"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert_eq!(kind, "class");
    assert_eq!(language, "ruby");
    assert!(metadata.contains("\"visibility\":\"public\""));
    assert!(metadata.contains("\"is_singleton\":false"));

    assert!(
        ruby_symbol_exists(&conn, "OrderController", "class"),
        "OrderController class should be indexed"
    );
    assert!(
        ruby_symbol_exists(&conn, "Logger", "class"),
        "Logger class should be indexed"
    );
    assert!(
        ruby_symbol_exists(&conn, "Payments::Gateway", "class"),
        "qualified class name should be preserved"
    );
    assert!(
        ruby_symbol_exists(&conn, "BaseGateway", "class"),
        "BaseGateway class should be indexed"
    );
    assert!(
        ruby_symbol_exists(&conn, "PaymentResult", "class"),
        "PaymentResult class should be indexed"
    );
}

#[test]
fn test_index_detects_ruby_modules() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_symbol_exists(&conn, "Auditable", "interface"),
        "Ruby modules should be indexed as interface symbols"
    );
}

#[test]
fn test_index_detects_ruby_methods() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    for name in [
        "initialize",
        "process_payment",
        "build",
        "default_currency",
        "audit!",
        "paid?",
        "settle!",
        "status=",
        "[]",
        "[]=",
        "==",
        "currency",
        "validate_card",
    ] {
        assert!(
            ruby_symbol_exists(&conn, name, "method"),
            "Ruby method should be indexed with exact name: {name}"
        );
    }
}

#[test]
fn test_ruby_methods_have_parent_id() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let parent_for = |method_name: &str| -> String {
        conn.query_row(
            "SELECT p.name
             FROM symbols m
             JOIN symbols p ON m.parent_id = p.id
             WHERE m.name = ?1 AND m.kind = 'method' AND m.language = 'ruby'
             LIMIT 1",
            rusqlite::params![method_name],
            |row| row.get(0),
        )
        .unwrap()
    };

    assert_eq!(parent_for("process_payment"), "PaymentService");
    assert_eq!(parent_for("build"), "PaymentService");
    assert_eq!(parent_for("default_currency"), "PaymentService");
    assert_eq!(parent_for("audit!"), "Auditable");
}

#[test]
fn test_index_detects_ruby_constants() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_symbol_exists(&conn, "DEFAULT_CURRENCY", "const"),
        "Ruby constant assignment should be indexed"
    );
}

#[test]
fn test_index_detects_ruby_assigned_lambdas_and_procs() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_symbol_exists(&conn, "normalize_amount", "function"),
        "assigned Ruby lambda should be indexed as function"
    );
    assert!(
        ruby_symbol_exists(&conn, "log_payment", "function"),
        "assigned Ruby proc should be indexed as function"
    );
}

#[test]
fn test_sketch_shows_ruby_class() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    sc_sketch(dir.path(), "PaymentService")
        .success()
        .stdout(contains("process_payment"))
        .stdout(contains("default_currency"));
}

#[test]
fn test_sketch_shows_ruby_module() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    sc_sketch(dir.path(), "Auditable")
        .success()
        .stdout(contains("Auditable"))
        .stdout(contains("audit!"));
}

#[test]
fn test_ruby_metadata_captures_visibility() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let process_payment = ruby_metadata(&conn, "process_payment");
    let validate_card = ruby_metadata(&conn, "validate_card");
    let initialize = ruby_metadata(&conn, "initialize");

    assert_eq!(process_payment["visibility"], "public");
    assert_eq!(validate_card["visibility"], "private");
    assert_eq!(
        initialize["visibility"], "public",
        "initialize should follow explicit region inference only"
    );
}

#[test]
fn test_ruby_metadata_captures_singleton() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let build = ruby_metadata(&conn, "build");
    let default_currency = ruby_metadata(&conn, "default_currency");
    let process_payment = ruby_metadata(&conn, "process_payment");

    assert_eq!(build["is_singleton"], true);
    assert_eq!(build["receiver"], "self");
    assert_eq!(default_currency["is_singleton"], true);
    assert_eq!(default_currency["receiver"], "self");
    assert_eq!(process_payment["is_singleton"], false);
}

#[test]
fn test_ruby_metadata_captures_parameters() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let process_payment = ruby_metadata(&conn, "process_payment");
    let params = process_payment["parameters"].as_array().unwrap();

    assert!(params
        .iter()
        .any(|p| p["name"] == "request" && p["kind"] == "positional"));
    assert!(params.iter().any(|p| p["name"] == "retry_count"
        && p["kind"] == "keyword_optional"
        && p["has_default"] == true));
    assert!(params
        .iter()
        .any(|p| p["name"] == "options" && p["kind"] == "double_splat"));
    assert!(params
        .iter()
        .any(|p| p["name"] == "block" && p["kind"] == "block"));
    assert_eq!(process_payment["has_keyword_args"], true);
    assert_eq!(process_payment["has_splat"], true);
    assert_eq!(process_payment["has_block_param"], true);

    let initialize = ruby_metadata(&conn, "initialize");
    let init_params = initialize["parameters"].as_array().unwrap();
    assert!(init_params
        .iter()
        .any(|p| p["name"] == "_unused" && p["kind"] == "optional"));
}

#[test]
fn test_ruby_metadata_captures_required_keywords() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let build = ruby_metadata(&conn, "build");
    let params = build["parameters"].as_array().unwrap();

    assert!(params
        .iter()
        .any(|p| p["name"] == "client" && p["kind"] == "keyword"));
    assert!(params
        .iter()
        .any(|p| p["name"] == "logger" && p["kind"] == "keyword"));
    assert!(!params.iter().any(|p| p["name"] == "self"));
}

#[test]
fn test_ruby_docstring_extracted() {
    let (conn, dir) = indexed_ruby_fixture_db();

    assert_eq!(
        ruby_docstring(&conn, "PaymentService").unwrap(),
        "Handles payment workflows for checkout requests."
    );
    assert_eq!(
        ruby_docstring(&conn, "process_payment").unwrap(),
        "Runs payment processing and yields the normalized request when needed."
    );

    sc_find(dir.path(), "normalized request")
        .success()
        .stdout(contains("process_payment"));
}

#[test]
fn test_ruby_endless_method_metadata() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let currency = ruby_metadata(&conn, "currency");
    let process_payment = ruby_metadata(&conn, "process_payment");

    assert_eq!(currency["is_endless"], true);
    assert_eq!(process_payment["is_endless"], false);
}

#[test]
fn test_ruby_metadata_captures_namespace() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let gateway = ruby_metadata(&conn, "Payments::Gateway");

    assert_eq!(gateway["namespace"], "Payments");
}

#[test]
fn test_ruby_vendor_patterns_are_written() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Gemfile"),
        "source \"https://rubygems.org\"\n",
    )
    .unwrap();

    sc_init(dir.path()).success();

    let config = std::fs::read_to_string(dir.path().join(".scope").join("config.toml")).unwrap();
    for expected in [
        "vendor", ".bundle", "gems", ".yardoc", "coverage", "tmp", "log",
    ] {
        assert!(
            config.contains(expected),
            "config.toml missing Ruby vendor pattern: {expected}"
        );
    }
}

#[test]
fn test_ruby_index_empty_file() {
    let dir = setup_empty_ruby_project();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/empty.rb"), "").unwrap();

    sc_index_full(dir.path()).success();

    let conn = rusqlite::Connection::open(dir.path().join(".scope").join("graph.db")).unwrap();
    assert!(ruby_file_hash_exists(&conn, "src/empty.rb"));
    assert_eq!(ruby_symbol_count_for_file(&conn, "src/empty.rb"), 0);
}

#[test]
fn test_ruby_index_comments_only_file() {
    let dir = setup_empty_ruby_project();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/comments_only.rb"),
        "# This file intentionally has no Ruby definitions.\n# Indexing must still succeed.\n",
    )
    .unwrap();

    sc_index_full(dir.path()).success();

    let conn = rusqlite::Connection::open(dir.path().join(".scope").join("graph.db")).unwrap();
    assert!(ruby_file_hash_exists(&conn, "src/comments_only.rb"));
    assert_eq!(ruby_symbol_count_for_file(&conn, "src/comments_only.rb"), 0);
}

#[test]
fn test_ruby_index_file_with_syntax_errors() {
    let dir = setup_empty_ruby_project();
    std::fs::create_dir_all(dir.path().join("app")).unwrap();
    std::fs::write(
        dir.path().join("app/valid.rb"),
        "class ValidRubyClass\n  def ok\n    true\n  end\nend\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("app/broken.rb"),
        "class BrokenRubyClass\n  def incomplete(\n",
    )
    .unwrap();

    sc_index_full(dir.path()).success();

    sc_sketch(dir.path(), "ValidRubyClass")
        .success()
        .stdout(contains("ValidRubyClass"));
}

#[test]
fn test_index_detects_ruby_import_edges() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_edge_count(&conn, "imports") >= 3,
        "Ruby fixture should have require, require_relative, and autoload import edges"
    );
    assert!(ruby_edge_exists(&conn, "imports", "json"));
    assert!(ruby_edge_exists(&conn, "imports", "../../lib/auditable"));
    assert!(ruby_edge_exists(&conn, "imports", "payment_result"));
}

#[test]
fn test_index_detects_ruby_call_edges() {
    let (conn, dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_edge_count(&conn, "calls") > 0,
        "Ruby fixture should have call edges"
    );
    assert!(ruby_edge_exists(&conn, "calls", "validate_card"));
    assert!(ruby_edge_exists(&conn, "calls", "audit!"));
    assert!(ruby_edge_exists(&conn, "calls", "PaymentService.build"));
    assert!(ruby_edge_like(&conn, "calls", "%.process_payment"));
    assert!(ruby_edge_exists(&conn, "calls", "yield"));
    assert!(ruby_edge_exists(&conn, "calls", "block.call"));

    sc_refs(dir.path(), "process_payment")
        .success()
        .stdout(contains("order_controller.rb"));
}

#[test]
fn test_index_detects_ruby_instantiates_edges() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_edge_count(&conn, "instantiates") > 0,
        "Ruby fixture should have instantiation edges"
    );
    assert!(ruby_edge_exists(&conn, "instantiates", "PaymentResult"));
    assert!(ruby_edge_exists(&conn, "instantiates", "PaymentService"));
    assert!(ruby_edge_exists(&conn, "instantiates", "Logger"));
}

#[test]
fn test_index_detects_ruby_extends_edges() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_edge_count(&conn, "extends") > 0,
        "Ruby fixture should have class inheritance edges"
    );
    assert!(ruby_edge_exists(&conn, "extends", "BaseGateway"));
}

#[test]
fn test_index_detects_ruby_implements_edges() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_edge_count(&conn, "implements") > 0,
        "Ruby fixture should have mixin edges"
    );
    assert!(ruby_edge_exists(&conn, "implements", "Auditable"));
}

#[test]
fn test_index_detects_ruby_references_type_edges() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_edge_count(&conn, "references_type") > 0,
        "Ruby fixture should have constant/type reference edges"
    );
    assert!(ruby_edge_exists(&conn, "references_type", "PaymentResult"));
}

#[test]
fn test_ruby_literal_send_edge_detected() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(ruby_edge_exists(&conn, "calls", "audit!"));
    assert!(ruby_edge_exists(&conn, "calls", "paid?"));
    assert!(ruby_edge_exists(&conn, "calls", "process_payment"));
}

#[test]
fn test_ruby_dynamic_send_not_resolved() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    assert!(
        !ruby_edge_exists(&conn, "calls", "dynamic_target"),
        "send(dynamic_target) must not create a false call edge"
    );
}

#[test]
fn test_ruby_call_edges_use_real_enclosing_scope() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    let fake_scope_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges
             WHERE from_id LIKE '%::new::function%'
                OR from_id LIKE '%::call::function%'
                OR from_id LIKE '%::build::function%'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        fake_scope_count, 0,
        "ordinary Ruby call nodes must not become synthetic from_id scopes"
    );
}

#[test]
fn test_ruby_graph_commands_use_edges() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    sc_deps(dir.path(), "PaymentService")
        .success()
        .stdout(contains("Auditable"))
        .stdout(contains("PaymentResult"));
    sc_impact(dir.path(), "PaymentService")
        .success()
        .stdout(contains("checkout"));
}

#[test]
fn test_ruby_sketch_payment_service() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["sketch", "PaymentService"]);
    assert!(stdout.contains("PaymentService"));
    assert!(stdout.contains("process_payment"));
    assert!(stdout.contains("deps:"));
    assert!(stdout.contains("extends:"));
    assert!(stdout.contains("BaseGateway"));
    assert!(stdout.contains("implements:"));
    assert!(stdout.contains("Auditable"));

    let json = scope_json(dir.path(), &["sketch", "PaymentService", "--json"]);
    assert_eq!(json["command"], "sketch");
    assert_eq!(json["data"]["symbol"]["kind"], "class");
    assert!(json["data"]["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "process_payment"));
}

#[test]
fn test_ruby_sketch_auditable_module() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["sketch", "Auditable"]);
    assert!(stdout.contains("Auditable"));
    assert!(stdout.contains("interface"));
    assert!(stdout.contains("audit!"));
}

#[test]
fn test_ruby_sketch_process_payment() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["sketch", "process_payment"]);
    assert!(stdout.contains("process_payment"));
    assert!(stdout.contains("validate_card"));
    assert!(stdout.contains("checkout"));
}

#[test]
fn test_ruby_summary_payment_service() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["summary", "PaymentService"]);
    assert!(stdout.contains("PaymentService (class)"));
    assert!(stdout.contains("app/services/payment_service.rb"));
    assert!(stdout.contains("methods"));

    let json = scope_json(dir.path(), &["summary", "PaymentService", "--json"]);
    assert_eq!(json["command"], "summary");
    assert_eq!(json["data"]["kind"], "class");
    assert!(json["data"]["methods"].as_u64().unwrap() >= 1);
}

#[test]
fn test_ruby_summary_process_payment() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["summary", "process_payment"]);
    assert!(stdout.contains("process_payment (method)"));
    assert!(stdout.contains("app/services/payment_service.rb"));

    let json = scope_json(dir.path(), &["summary", "process_payment", "--json"]);
    assert_eq!(json["data"]["kind"], "method");
    assert_eq!(json["data"]["name"], "process_payment");
}

#[test]
fn test_ruby_source_process_payment() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["source", "process_payment"]);
    assert!(stdout.contains("def process_payment"));
    assert!(stdout.contains("PaymentResult.new"));
    assert!(stdout.contains("yield(result)"));
    assert!(!stdout.contains("class PaymentService < BaseGateway"));

    let json = scope_json(dir.path(), &["source", "process_payment", "--json"]);
    assert_eq!(json["command"], "source");
    assert_eq!(json["data"]["kind"], "method");
    assert!(json["data"]["source"]
        .as_str()
        .unwrap()
        .contains("public_send(\"process_payment\")"));
}

#[test]
fn test_ruby_refs_process_payment() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["refs", "process_payment"]);
    assert!(stdout.contains("order_controller.rb"));
    assert!(stdout.contains("retry_job.rb"));
    assert!(stdout.contains("public_send(\"process_payment\")"));
    assert_eq!(
        stdout.matches("public_send(\"process_payment\")").count(),
        1,
        "same-line literal metaprogramming refs should not be duplicated in human output"
    );

    let json = scope_json(dir.path(), &["refs", "process_payment", "--json"]);
    assert_eq!(json["command"], "refs");
    let refs = json["data"].as_array().unwrap();
    assert!(refs.iter().any(|r| r["file_path"]
        .as_str()
        .unwrap()
        .ends_with("order_controller.rb")));
    assert!(refs
        .iter()
        .any(|r| r["file_path"].as_str().unwrap().ends_with("retry_job.rb")));
}

#[test]
fn test_ruby_refs_mixin_and_type_targets() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let auditable = scope_stdout(dir.path(), &["refs", "Auditable"]);
    assert!(auditable.contains("implemented"));
    assert!(auditable.contains("include Auditable"));

    let result = scope_stdout(dir.path(), &["refs", "PaymentResult"]);
    assert!(result.contains("instantiated"));
    assert!(result.contains("PaymentResult.new"));
}

#[test]
fn test_ruby_deps_payment_service() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["deps", "PaymentService"]);
    assert!(stdout.contains("imports"));
    assert!(stdout.contains("json"));
    assert!(stdout.contains("../../lib/auditable"));
    assert!(stdout.contains("calls:"));
    assert!(stdout.contains("validate_card"));
    assert!(stdout.contains("instantiates:"));
    assert!(stdout.contains("PaymentResult"));
    assert!(stdout.contains("implements:"));
    assert!(stdout.contains("Auditable"));
    assert!(
        !stdout.contains("imports:\n  paid?"),
        "literal public_send targets must not be reported as imports"
    );

    let json = scope_json(dir.path(), &["deps", "PaymentService", "--json"]);
    let deps = json["data"].as_array().unwrap();
    assert!(deps
        .iter()
        .any(|dep| dep["kind"] == "imports" && dep["name"] == "json"));
    assert!(deps
        .iter()
        .any(|dep| dep["kind"] == "instantiates" && dep["name"] == "PaymentResult"));
    assert!(deps
        .iter()
        .any(|dep| dep["kind"] == "implements" && dep["name"] == "Auditable"));
}

#[test]
fn test_ruby_deps_process_payment() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["deps", "process_payment"]);
    assert!(stdout.contains("validate_card"));
    assert!(stdout.contains("PaymentResult"));
    assert!(stdout.contains("audit!"));
}

#[test]
fn test_ruby_rdeps_payment_service() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["rdeps", "PaymentService"]);
    assert!(stdout.contains("checkout"));
    assert!(stdout.contains("call"));

    let json = scope_json(dir.path(), &["rdeps", "PaymentService", "--json"]);
    assert_eq!(json["command"], "rdeps");
    assert!(json["data"]["total_affected"].as_u64().unwrap() >= 2);
}

#[test]
fn test_ruby_rdeps_auditable() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["rdeps", "Auditable"]);
    assert!(stdout.contains("PaymentService"));
}

#[test]
fn test_ruby_impact_process_payment() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["impact", "process_payment"]);
    assert!(stdout.contains("checkout"));
    assert!(stdout.contains("call"));

    let json = scope_json(dir.path(), &["impact", "process_payment", "--json"]);
    assert_eq!(json["command"], "impact");
    assert!(json["data"]["total_affected"].as_u64().unwrap() >= 2);
}

#[test]
fn test_ruby_trace_process_payment() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["trace", "process_payment"]);
    assert!(stdout.contains("entry path"));
    assert!(stdout.contains("process_payment"));
    assert!(stdout.contains("checkout") || stdout.contains("call"));

    let json = scope_json(dir.path(), &["trace", "process_payment", "--json"]);
    assert_eq!(json["command"], "trace");
    assert!(!json["data"]["paths"].as_array().unwrap().is_empty());
}

#[test]
fn test_ruby_map_includes_ruby() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["map"]);
    assert!(stdout.contains("ruby"));

    let json = scope_json(dir.path(), &["map", "--json"]);
    let languages = json["data"]["stats"]["languages"].as_array().unwrap();
    assert!(languages.iter().any(|lang| lang == "ruby"));
}

#[test]
fn test_ruby_find_payment_retry() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stdout = scope_stdout(dir.path(), &["find", "payment retry"]);
    assert!(stdout.contains("process_payment") || stdout.contains("PaymentService"));
    assert!(
        !stdout.contains("initialize"),
        "Ruby stopwords and ranking should keep generic lifecycle methods out of this result"
    );

    let json = scope_json(dir.path(), &["find", "payment retry", "--json"]);
    assert!(!json["data"].as_array().unwrap().is_empty());
}

#[test]
fn test_ruby_ambiguous_common_method_disambiguates() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    let stderr = scope_stderr_failure(dir.path(), &["summary", "call"]);
    assert!(stderr.contains("Ambiguous symbol 'call'"));
    assert!(stderr.contains("RetryJob.call"));
    assert!(stderr.contains("WebhookJob.call"));

    let stdout = scope_stdout(dir.path(), &["summary", "RetryJob.call"]);
    assert!(stdout.contains("call (method)"));
    assert!(stdout.contains("retry_job.rb"));

    let source = scope_stdout(dir.path(), &["source", "WebhookJob.call"]);
    assert!(source.contains("PaymentService.build"));
}

#[test]
fn test_ruby_vendor_paths_deranked() {
    let dir = setup_ruby_fixture();
    std::fs::create_dir_all(dir.path().join("vendor")).unwrap();
    std::fs::write(
        dir.path().join("vendor/payment_service.rb"),
        "class PaymentService\n  def process_payment\n    :vendor\n  end\nend\n",
    )
    .unwrap();

    sc_init(dir.path()).success();
    sc_index_full(dir.path()).success();

    let json = scope_json(
        dir.path(),
        &["find", "PaymentService", "--json", "--limit", "5"],
    );
    let results = json["data"].as_array().unwrap();
    assert!(!results.is_empty());
    assert!(
        !results[0]["file_path"]
            .as_str()
            .unwrap()
            .starts_with("vendor/"),
        "first-party Ruby result should rank ahead of vendor result"
    );
    assert!(
        results
            .iter()
            .any(|result| result["file_path"].as_str().unwrap().starts_with("vendor/")),
        "control check: search should include the vendor duplicate before de-ranking is asserted"
    );
}

#[test]
fn test_ruby_nested_namespaces() {
    let (conn, dir) = indexed_ruby_fixture_db();

    assert!(
        ruby_symbol_exists(&conn, "Payments::Gateway", "class"),
        "lexically nested module/class names should retain their Ruby namespace"
    );
    assert!(ruby_symbol_exists(
        &conn,
        "Payments::RefundService",
        "class"
    ));

    let gateway = ruby_metadata(&conn, "Payments::Gateway");
    assert_eq!(gateway["namespace"], "Payments");

    let authorize_parent: String = conn
        .query_row(
            "SELECT p.name
             FROM symbols m
             JOIN symbols p ON m.parent_id = p.id
             WHERE m.name = 'authorize' AND m.kind = 'method' AND m.language = 'ruby'
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(authorize_parent, "Payments::Gateway");

    let stdout = scope_stdout(dir.path(), &["sketch", "Payments::RefundService"]);
    assert!(stdout.contains("Payments::RefundService"));
    assert!(stdout.contains("call"));
}

#[test]
fn test_ruby_heredocs_and_literals_do_not_create_false_edges() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    for false_target in [
        "ghost_call",
        "ghost_dependency",
        "GhostMixin",
        "ghost_message",
        "regex_ghost",
    ] {
        assert!(
            !ruby_edge_exists(&conn, "calls", false_target)
                && !ruby_edge_exists(&conn, "imports", false_target)
                && !ruby_edge_exists(&conn, "implements", false_target)
                && !ruby_edge_exists(&conn, "references_type", false_target),
            "Ruby heredoc/percent/regex literal text must not create edge to {false_target}"
        );
    }
}

#[test]
fn test_ruby_dsl_calls_do_not_create_symbols() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    for name in [
        "before_action",
        "has_many",
        "describe",
        "it",
        "authenticate",
    ] {
        assert!(
            !ruby_symbol_exists(&conn, name, "function")
                && !ruby_symbol_exists(&conn, name, "method")
                && !ruby_symbol_exists(&conn, name, "class"),
            "Ruby DSL call should not be indexed as a symbol: {name}"
        );
    }

    assert!(ruby_edge_exists(&conn, "calls", "before_action"));
    assert!(ruby_edge_exists(&conn, "calls", "has_many"));
    assert!(ruby_edge_exists(&conn, "calls", "describe"));
    assert!(ruby_edge_exists(&conn, "calls", "it"));

    let false_scope_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges
             WHERE from_id LIKE '%::before_action::function%'
                OR from_id LIKE '%::has_many::function%'
                OR from_id LIKE '%::describe::function%'
                OR from_id LIKE '%::it::function%'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        false_scope_count, 0,
        "Ruby DSL blocks must not become synthetic from_id scopes"
    );
}

#[test]
fn test_ruby_operator_methods() {
    let (conn, dir) = indexed_ruby_fixture_db();

    for name in ["[]", "[]=", "=="] {
        assert!(
            ruby_symbol_exists(&conn, name, "method"),
            "Ruby operator method should be indexed with exact name: {name}"
        );
    }

    let stdout = scope_stdout(dir.path(), &["source", "=="]);
    assert!(stdout.contains("def =="));
}

fn ruby_extends_from_id(conn: &rusqlite::Connection, from_like: &str, to_id: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE kind = 'extends' AND from_id LIKE ?1 AND to_id = ?2",
            rusqlite::params![from_like, to_id],
            |row| row.get(0),
        )
        .unwrap();
    count > 0
}

#[test]
fn test_ruby_extends_attaches_to_subclass_not_outer_scope() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    // F-GAP-007 + F-FP-002: nested classes inside the same module had
    // their `class X < Y` extends edge attributed to the outer module
    // instead of to X itself. Verify each subclass owns its own edge.
    assert!(
        ruby_extends_from_id(&conn, "%Routing::Redirect::class%", "Endpoint"),
        "Routing::Redirect should own its extends edge to Endpoint"
    );
    assert!(
        ruby_extends_from_id(&conn, "%Routing::PathRedirect::class%", "Redirect"),
        "Routing::PathRedirect should own its extends edge to Redirect"
    );
    assert!(
        ruby_extends_from_id(&conn, "%Routing::OptionRedirect::class%", "Redirect"),
        "Routing::OptionRedirect should own its extends edge to Redirect"
    );
}

#[test]
fn test_ruby_extends_for_class_inside_class() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    // Class declared inside another class must keep its extends edge.
    assert!(
        ruby_extends_from_id(
            &conn,
            "%Wrapper::Logger2::SimpleFormatter::class%",
            "Formatter"
        ),
        "SimpleFormatter (nested in Logger2) should extend Formatter"
    );
}

#[test]
fn test_ruby_extends_for_application_engine() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    // F-GAP-001: `class Application < Engine` style — the engine pattern.
    assert!(
        ruby_extends_from_id(&conn, "%Wrapper::Application::class%", "Engine"),
        "Wrapper::Application must extend Engine"
    );
}

#[test]
fn test_ruby_symbol_resolution_prefers_lib_over_spec_dummy() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    // F-A-004 / F-FP-001: when the same class exists in both lib/ and a
    // spec/dummy/ tree, sketch must resolve to the lib/ version.
    let stdout = scope_stdout(dir.path(), &["sketch", "CanonicalRecord"]);
    assert!(
        stdout.contains("lib/canonical_record.rb"),
        "CanonicalRecord must resolve to lib/, not spec/dummy/. Got:\n{stdout}"
    );
    assert!(
        !stdout.contains("spec/dummy/canonical_record.rb"),
        "CanonicalRecord must not resolve to the spec/dummy/ duplicate. Got:\n{stdout}"
    );
    assert!(
        stdout.contains("lib_method"),
        "Resolved sketch should expose the lib version's method"
    );
}

#[test]
fn test_ruby_sketch_dedups_implements() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    // F-A-003: same module mixed in multiple times must show only once.
    let stdout = scope_stdout(dir.path(), &["sketch", "DupConsumer"]);
    let implements_line = stdout
        .lines()
        .find(|l| l.starts_with("implements:"))
        .unwrap_or("");
    assert_eq!(
        implements_line.matches("DupHelper").count(),
        1,
        "DupHelper should appear exactly once in implements line; got: {implements_line}"
    );
}

#[test]
fn test_ruby_module_sketch_shows_implements() {
    let (_conn, dir) = indexed_ruby_fixture_db();

    // F-GAP-004: a Ruby module that does `include X` should expose the
    // mixin in its sketch via an `implements:` line.
    let stdout = scope_stdout(dir.path(), &["sketch", "Trackable"]);
    assert!(
        stdout.contains("implements:") && stdout.contains("Auditable"),
        "Trackable module should show `implements: Auditable`. Got:\n{stdout}"
    );
}

#[test]
fn test_ruby_references_type_subscript_is_a_when() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    // F-GAP-008: `Klass[..]`, `is_a?(Klass)`, `case ... when Klass`
    // must each emit a `references_type` edge.
    assert!(
        ruby_edge_exists(&conn, "references_type", "TaggedValue"),
        "TaggedValue used as subscript receiver / is_a? arg / when pattern must produce references_type"
    );
    assert!(
        ruby_edge_exists(&conn, "references_type", "Tag"),
        "Tag used in `when Tag` must produce references_type"
    );

    // `Klass[..]` (element_reference) — must not be miscategorised as instantiates/calls.
    assert!(
        !ruby_edge_exists(&conn, "instantiates", "TaggedValue"),
        "Klass[..] must not create an instantiates edge"
    );
}

#[test]
fn test_ruby_extends_does_not_attribute_to_outer_module() {
    let (conn, _dir) = indexed_ruby_fixture_db();

    // The outer `module Routing` itself must not own any extends edge —
    // those belong to the nested subclasses.
    let outer_extends: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges
             WHERE kind = 'extends'
               AND file_path LIKE '%inheritance.rb'
               AND from_id LIKE '%Routing::interface%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        outer_extends, 0,
        "outer module Routing must not aggregate subclass extends edges"
    );
}
