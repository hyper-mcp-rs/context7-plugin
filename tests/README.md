# Context7 Plugin Test Suite

This directory contains integration tests for the Context7 plugin, specifically testing the `ResolveLibraryIdResponse` deserialization against the actual Context7 API.

## Overview

The tests in this directory verify that the plugin can correctly deserialize responses from the Context7 API's `/v2/libs/search` endpoint. Unlike unit tests, these integration tests make real HTTP requests to the Context7 API using `reqwest`.

## Test File: `resolve_library_id_tests.rs`

### API Integration Tests

These tests make actual HTTP calls to the Context7 API:

1. **`test_resolve_library_id_response_with_results`**
   - Tests deserialization with a popular library (React)
   - Verifies that results are returned and all required fields are present
   - Validates the structure of `Library` objects

2. **`test_resolve_library_id_response_with_any_results`**
   - Tests deserialization with an unusual query
   - Verifies that the response deserializes successfully regardless of result count
   - Demonstrates that the API may return fuzzy matches

3. **`test_resolve_library_id_response_nextjs`**
   - Tests with Next.js library
   - Verifies optional fields (stars, trust_score, benchmark_score) are handled correctly
   - Demonstrates version information handling

4. **`test_resolve_library_id_response_mongodb`**
   - Tests with MongoDB library
   - Shows multiple result handling
   - Useful for debugging and exploring API responses

### Unit Tests

These tests verify deserialization without making API calls:

5. **`test_document_state_deserialization`**
   - Tests all `DocumentState` enum variants
   - Verifies: `delete`, `error`, `finalized`, `initial`

6. **`test_library_deserialization_complete`**
   - Tests deserialization of a complete `Library` object with all optional fields

7. **`test_library_deserialization_minimal`**
   - Tests deserialization of a minimal `Library` object with only required fields

8. **`test_resolve_library_id_response_with_error`**
   - Tests handling of error responses from the API

9. **`test_resolve_library_id_response_empty_results`**
   - Tests handling of responses with no results (but no error)

10. **`test_resolve_library_id_response_multiple_results`**
    - Tests handling of responses with multiple library results

## Running Tests

Because this is a WASM project (compiled for `wasm32-wasip1`), the tests must be run with an explicit native target:

```bash
cargo test --test resolve_library_id_tests --target $(rustc -vV | grep host | cut -d' ' -f2) -- --nocapture
```

Or more simply, if you want to see the output:

```bash
cargo test --test resolve_library_id_tests --target aarch64-apple-darwin -- --nocapture  # For macOS ARM
cargo test --test resolve_library_id_tests --target x86_64-apple-darwin -- --nocapture   # For macOS Intel
cargo test --test resolve_library_id_tests --target x86_64-unknown-linux-gnu -- --nocapture  # For Linux
```

The `--nocapture` flag allows you to see `println!` output from the tests, which includes:
- Full response bodies from API calls
- Deserialized result counts
- Individual library details

## Dependencies

The test suite requires the following dev dependencies (defined in `Cargo.toml`):
- `reqwest` - For making HTTP requests
- `tokio` - Async runtime for reqwest

Note: `url`, `serde`, and `serde_json` are already included in the main `[dependencies]` section, so they don't need to be listed again in `[dev-dependencies]`.

## Data Structures Tested

### `ResolveLibraryIdResponse`
```rust
struct ResolveLibraryIdResponse {
    error: Option<String>,
    results: Vec<Library>,
}
```

### `Library`
```rust
struct Library {
    id: String,
    title: String,
    description: String,
    branch: String,
    last_update_date: String,
    state: DocumentState,
    total_tokens: f64,
    total_snippets: f64,
    stars: Option<f64>,
    trust_score: Option<f64>,
    benchmark_score: Option<f64>,
    versions: Vec<String>,
}
```

### `DocumentState`
```rust
enum DocumentState {
    Initial,
    Delete,
    Error,
    Finalized,
}
```

## Notes

- The API integration tests require network access to `https://context7.com/api`
- Response data may vary over time as the Context7 API's library index is updated
- Tests verify the structure and deserialization logic, not specific API content
- All tests include descriptive output to help with debugging

## Continuous Integration

The tests are automatically run in the following GitHub Actions workflow:

- **CI Workflow** (`ci.yml`) - Runs on every push to `main` and on pull requests

The CI workflow runs the test job before the build job, ensuring that tests must pass before the build completes. This helps catch deserialization issues early and ensures the plugin works with the actual Context7 API.