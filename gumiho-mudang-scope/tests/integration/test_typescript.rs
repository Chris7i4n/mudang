/// Integration tests for TypeScript language support.
///
/// Each test copies the TypeScript fixture to a temporary directory, runs
/// `scope init` + `scope index --full`, and verifies symbols and edges.
use assert_cmd::Command;
use predicates::str::contains;
use std::path::Path;
use tempfile::TempDir;

const TYPESCRIPT_FIXTURE: &str = "tests/fixtures/typescript-simple";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn copy_dir_all(src: &Path, dest: &Path) {
    std::fs::create_dir_all(dest).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            // Skip pollution from a prior local `scope init`.
            if entry.file_name() == ".scope" {
                continue;
            }
            copy_dir_all(&src_path, &dest_path);
        } else {
            std::fs::copy(&src_path, &dest_path).unwrap();
        }
    }
}

fn setup_typescript_fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let fixture = Path::new(TYPESCRIPT_FIXTURE);
    copy_dir_all(fixture, dir.path());
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

fn indexed_typescript_db() -> (rusqlite::Connection, TempDir) {
    let dir = setup_typescript_fixture();
    sc_init(dir.path()).success();
    sc_index_full(dir.path()).success();

    let db_path = dir.path().join(".scope").join("graph.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    (conn, dir)
}

fn symbol_exists(conn: &rusqlite::Connection, name: &str, kind: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbols WHERE name = ?1 AND kind = ?2",
            rusqlite::params![name, kind],
            |row| row.get(0),
        )
        .unwrap();
    count > 0
}

// ---------------------------------------------------------------------------
// Tests — scope init detects TypeScript
// ---------------------------------------------------------------------------

#[test]
fn test_init_detects_typescript_from_tsconfig() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();

    sc_init(dir.path()).success().stdout(contains("TypeScript"));
}

#[test]
fn test_init_detects_typescript_from_package_json() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        "{\"name\":\"x\",\"version\":\"0.0.0\"}",
    )
    .unwrap();

    sc_init(dir.path()).success().stdout(contains("TypeScript"));
}

// ---------------------------------------------------------------------------
// Tests — scope index on TypeScript fixture
// ---------------------------------------------------------------------------

#[test]
fn test_index_full_on_typescript_fixture() {
    let dir = setup_typescript_fixture();

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

// ---------------------------------------------------------------------------
// Tests — symbol detection
// ---------------------------------------------------------------------------

#[test]
fn test_index_detects_typescript_classes() {
    let (conn, _dir) = indexed_typescript_db();

    assert!(
        symbol_exists(&conn, "PaymentService", "class"),
        "PaymentService class should be indexed"
    );
    assert!(
        symbol_exists(&conn, "Logger", "class"),
        "Logger class should be indexed"
    );
    assert!(
        symbol_exists(&conn, "OrderController", "class"),
        "OrderController class should be indexed"
    );
    assert!(
        symbol_exists(&conn, "RefundController", "class"),
        "RefundController class should be indexed"
    );
}

#[test]
fn test_index_detects_typescript_interfaces() {
    let (conn, _dir) = indexed_typescript_db();

    assert!(
        symbol_exists(&conn, "PaymentRequest", "interface"),
        "PaymentRequest interface should be indexed"
    );
    assert!(
        symbol_exists(&conn, "PaymentResult", "interface"),
        "PaymentResult interface should be indexed"
    );
}

#[test]
fn test_index_detects_typescript_type_alias() {
    let (conn, _dir) = indexed_typescript_db();

    assert!(
        symbol_exists(&conn, "PaymentStatus", "type"),
        "PaymentStatus type alias should be indexed"
    );
}

#[test]
fn test_index_detects_typescript_enums() {
    let (conn, _dir) = indexed_typescript_db();

    assert!(
        symbol_exists(&conn, "PaymentMethod", "enum"),
        "PaymentMethod enum should be indexed"
    );
}

#[test]
fn test_index_detects_typescript_methods() {
    let (conn, _dir) = indexed_typescript_db();

    for name in [
        "processPayment",
        "refundPayment",
        "validateAmount",
        "describeMethod",
        "info",
        "error",
        "checkout",
        "retryPayment",
        "processRefund",
    ] {
        assert!(
            symbol_exists(&conn, name, "method"),
            "method {name} should be indexed"
        );
    }
}

#[test]
fn test_index_detects_typescript_enum_variants() {
    let (conn, _dir) = indexed_typescript_db();

    let variants: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM symbols WHERE kind = 'variant' ORDER BY name")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };

    for variant in ["CreditCard", "BankTransfer", "Wallet"] {
        assert!(
            variants.contains(&variant.to_string()),
            "{variant} variant should be indexed; found: {variants:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Tests — edge patterns
// ---------------------------------------------------------------------------

/// `this.validateAmount(amount)` inside `processPayment` should produce a
/// 'calls' edge whose to_id matches the bare method name.
#[test]
fn test_typescript_this_method_call_edge_detected() {
    let (conn, _dir) = indexed_typescript_db();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges
             WHERE (to_id = 'validateAmount' OR to_id LIKE '%::validateAmount') AND kind = 'calls'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        count > 0,
        "this.validateAmount() should generate a 'calls' edge to validateAmount; got {count}"
    );
}

/// `this.paymentService.processPayment(...)` inside `OrderController.checkout`
/// should produce a cross-file 'calls' edge.
#[test]
fn test_typescript_cross_file_method_call_edge_detected() {
    let (conn, _dir) = indexed_typescript_db();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges
             WHERE kind = 'calls' AND (
                to_id = 'processPayment'
                OR to_id LIKE '%::processPayment'
                OR to_id LIKE '%.processPayment'
             )",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        count > 0,
        "OrderController.checkout should call processPayment; got {count}"
    );
}

/// Enum variant references (e.g. `PaymentMethod.CreditCard`) should produce
/// 'references_type' or 'calls' edges into the enum/variant.
#[test]
fn test_typescript_enum_variant_reference_edge_detected() {
    let (conn, _dir) = indexed_typescript_db();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE kind = 'references_type' AND \
             (to_id = 'PaymentMethod' OR to_id LIKE '%::PaymentMethod')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        count > 0,
        "describeMethod should reference PaymentMethod; got {count}"
    );
}
