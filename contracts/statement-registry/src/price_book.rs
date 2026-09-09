// Path is relative to this crate's Cargo.toml directory (CARGO_MANIFEST_DIR
// — verified in soroban-sdk-macros' own source, since the macro's own doc
// comment claims "relative to the workspace root", which is not what the
// implementation actually does for a multi-crate workspace like this one),
// not relative to this file. contracts/statement-registry/ -> .. ->
// contracts/ -> .. -> repo root, then into target/wasm32v1-none/release/.
//
// This creates a compile-time dependency: price-book must be built to wasm
// before this crate compiles, or contractimport! fails to find the file.
// See the Makefile and README for the enforced build order.
soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/price_book.wasm");
