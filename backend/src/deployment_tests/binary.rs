//! What the compiled backend carries, checked against the running binary.

use duckdb::Connection;

// @spec DEPLOY-IMAGE-011
#[test]
fn the_parquet_reader_is_compiled_in() {
    let conn = Connection::open_in_memory().unwrap();
    let install_mode: String = conn
        .query_row(
            "SELECT install_mode FROM duckdb_extensions() WHERE extension_name = 'parquet'",
            [],
            |row| row.get(0),
        )
        .expect("DuckDB knows the parquet extension");
    assert_eq!(
        install_mode, "STATICALLY_LINKED",
        "a lake read must install or download nothing"
    );
}
