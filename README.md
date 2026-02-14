# Context7 API Tools Plugin

This plugin provides tools to interact with the Context7 API, allowing you to resolve library IDs and query up-to-date documentation for any programming library or framework.

It is a drop-in replacement for the MCP server provided by Context7, with structured output when resolving library ids.

## Usage

Add the plugin to your Hyper MCP configuration:

```json
{
  "plugins": [
    {
      "name": "context7",
      "path": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:latest",
      "runtime_config": {
        "allowed_hosts": ["context7.com"]
      }
    }
  ]
}
```

For nightly builds:
```json
{
  "plugins": [
    {
      "name": "context7",
      "path": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:nightly",
      "runtime_config": {
        "allowed_hosts": ["context7.com"]
      }
    }
  ]
}
```

## Tools

### 1. `resolve_library_id`

**Description:** Resolves a package/product name to a Context7-compatible library ID and returns matching libraries.

You MUST call this function before `query_docs` to obtain a valid Context7-compatible library ID UNLESS the user explicitly provides a library ID in the format `/org/project` or `/org/project/version` in their query.

**Selection Process:**
1. Analyze the query to understand what library/package the user is looking for
2. Return the most relevant match based on:
   - Name similarity to the query (exact matches prioritized)
   - Description relevance to the query's intent
   - Documentation coverage (prioritize libraries with higher Code Snippet counts)
   - Source reputation (consider libraries with High or Medium reputation more authoritative)
   - Benchmark Score: Quality indicator (100 is the highest score)

**Response Format:**
- Return the selected library ID in a clearly marked section
- Provide a brief explanation for why this library was chosen
- If multiple good matches exist, acknowledge this but proceed with the most relevant one
- If no good matches exist, clearly state this and suggest query refinements

**IMPORTANT:** Do not call this tool more than 3 times per question. If you cannot find what you need after 3 calls, use the best result you have.

**Input Schema:**
```json
{
  "libraryName": "string (required) - Library name to search for",
  "query": "string (required) - The question or task you need help with"
}
```

**Example Input:**
```json
{
  "libraryName": "react",
  "query": "How to use hooks in React"
}
```

**Output:**
Returns a structured response with matching libraries including:
- `id`: Context7-compatible library ID (e.g., `/facebook/react`, `/vercel/next.js`)
- `title`: Library display name
- `description`: Library description
- `branch`: Git branch (e.g., `main`, `master`)
- `lastUpdateDate`: When the library documentation was last updated
- `state`: Document state (`finalized`, `initial`, `error`, `delete`)
- `totalTokens`: Number of tokens in the library documentation
- `totalSnippets`: Number of code snippets available
- `stars`: GitHub stars (optional)
- `trustScore`: Trust score 0-10 (optional)
- `benchmarkScore`: Quality benchmark 0-100 (optional)
- `versions`: Available versions (optional)

**Example Output:**
```json
{
  "results": [
    {
      "id": "/websites/react_dev",
      "title": "React",
      "description": "React is a JavaScript library for building user interfaces.",
      "branch": "main",
      "lastUpdateDate": "2024-02-05T09:48:38.174Z",
      "state": "finalized",
      "totalTokens": 861433,
      "totalSnippets": 5574,
      "stars": -1,
      "trustScore": 10,
      "benchmarkScore": 89.2,
      "versions": []
    }
  ]
}
```

### 2. `query_docs`

**Description:** Retrieves and queries up-to-date documentation and code examples from Context7 for any programming library or framework.

You must call `resolve_library_id` first to obtain the exact Context7-compatible library ID required to use this tool, UNLESS the user explicitly provides a library ID in the format `/org/project` or `/org/project/version` in their query.

**IMPORTANT:** Do not call this tool more than 3 times per question. If you cannot find what you need after 3 calls, use the best information you have.

**Input Schema:**
```json
{
  "libraryId": "string (required) - Context7-compatible library ID (e.g., '/mongodb/docs', '/vercel/next.js')",
  "query": "string (required) - Your specific question or task"
}
```

**Example Input:**
```json
{
  "libraryId": "/vercel/next.js",
  "query": "How to implement server-side rendering in Next.js 14"
}
```

**Output:**
Returns documentation and code examples relevant to your query in text format.

## Development

### Building

Build the WASM plugin:
```bash
cargo build --release --target wasm32-wasip1
```

The compiled plugin will be available at `target/wasm32-wasip1/release/plugin.wasm`.

### Testing

The plugin includes comprehensive integration tests that make real API calls to Context7:

```bash
# Run tests (requires native target, not WASM)
cargo test --test resolve_library_id_tests --target $(rustc -vV | grep host | cut -d' ' -f2)
```

Or specify your target explicitly:
```bash
# macOS ARM
cargo test --test resolve_library_id_tests --target aarch64-apple-darwin

# macOS Intel
cargo test --test resolve_library_id_tests --target x86_64-apple-darwin

# Linux
cargo test --test resolve_library_id_tests --target x86_64-unknown-linux-gnu
```

Tests verify:
- ✅ Response deserialization with actual Context7 API calls
- ✅ Handling of popular libraries (React, Next.js, MongoDB)
- ✅ Empty/no results scenarios
- ✅ Optional field handling
- ✅ All DocumentState enum variants
- ✅ Error responses

See [tests/README.md](tests/README.md) for detailed test documentation.

### Continuous Integration

Tests run automatically in the CI workflow:
- **CI Workflow** - On every push to `main` and on pull requests

Tests must pass before the build job completes. This helps catch deserialization issues early and ensures the plugin works with the actual Context7 API.

### Code Quality

```bash
# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings
```

## API Endpoints

The plugin uses the Context7 API:
- **Base URL:** `https://context7.com/api`
- **Library Search:** `GET /v2/libs/search?libraryName={name}&query={query}`
- **Query Docs:** `GET /v2/context?libraryId={id}&query={query}`

### Request Headers
- `X-Context7-Source: hyper-mcp/context7-plugin`
- `X-Context7-Server-Version: {version}`

## Privacy & Security

**Important:** Do not include sensitive or confidential information in your queries, including:
- API keys, passwords, or credentials
- Personal data
- Proprietary code
- Trade secrets

All queries are sent to the Context7 API for processing.

## License

See [LICENSE](LICENSE) for details.
