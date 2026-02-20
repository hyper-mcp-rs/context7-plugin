# Context7 API Tools Plugin

This plugin provides tools to interact with the Context7 API, allowing you to resolve library IDs and query up-to-date documentation for any programming library or framework.

It is a drop-in replacement for the MCP server provided by Context7, with structured output for both library resolution and documentation queries.

## Configuration

### API Key (Optional)

The plugin supports authenticated access to the Context7 API using an API key. While the plugin works with anonymous access, using an API key may provide:
- Higher rate limits
- Access to premium features
- Better performance

#### Option 1: Using System Keyring (Recommended)

For secure storage, store your API key in your system keyring and reference it in the configuration:

```json
{
  "plugins": {
    "context7": {
      "url": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:latest",
      "runtime_config": {
        "allowed_hosts": ["context7.com"],
        "allowed_secrets": [
          {
            "service": "your-service",
            "user": "your-user"
          }
        ],
        "env_vars": {
          "CONTEXT7_API_KEY": "{\"service\":\"your-service\",\"user\":\"your-user\"}"
        }
      }
    }
  }
}
```

Then store your API key in the system keyring with service name `your-service` and user `your-user`. Note that:
- The keyring reference is passed as a JSON string in `env_vars.CONTEXT7_API_KEY`
- The same keyring entry must be listed in `allowed_secrets` array
- The JSON string needs escaped quotes: `"{\"service\":\"your-service\",\"user\":\"your-user\"}"`
- The secret stored should be the plain-text API key (no JSON formatting)

#### Option 2: Plain Text (Not Recommended)

Alternatively, you can provide the API key directly in the configuration (less secure):

```json
{
  "plugins": {
    "context7": {
      "url": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:latest",
      "runtime_config": {
        "allowed_hosts": ["context7.com"],
        "env_vars": {
          "CONTEXT7_API_KEY": "your-api-key-here"
        }
      }
    }
  }
}
```

⚠️ **Warning:** Storing API keys in plain text is not recommended for production use.

#### Anonymous Access

If no API key is configured, the plugin will use anonymous access. You'll see an info-level log message:
```
Unable to resolve api key for Context7, using anonymous access
```

### Response Caching (Optional)

The plugin supports on-disk caching of API responses to reduce the number of calls made to the Context7 API. Caching is enabled by mounting a `/cache` directory via the `allowed_paths` runtime configuration.

#### Enabling the Cache

Add `/cache` to `allowed_paths` in your plugin configuration, mapping it to a directory on the host:

```json
{
  "plugins": {
    "context7": {
      "url": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:latest",
      "runtime_config": {
        "allowed_hosts": ["context7.com"],
        "allowed_paths": ["/path/on/host/context7-cache:/cache"]
      }
    }
  }
}
```

If the `/cache` directory is not mounted, the plugin will log an info-level message and operate without caching:
```
Cache directory /cache is not mounted; caching is disabled
```

#### Cache TTL

By default, cached responses expire after **1 day**. You can customize this with the `CACHE_TTL` configuration variable:

```json
{
  "plugins": {
    "context7": {
      "url": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:latest",
      "runtime_config": {
        "allowed_hosts": ["context7.com"],
        "allowed_paths": ["/path/on/host/context7-cache:/cache"],
        "env_vars": {
          "CACHE_TTL": "7"
        }
      }
    }
  }
}
```

#### How it Works

- Cache entries are stored as JSON files in `/cache`, keyed by a hash of the tool arguments.
- Staleness is determined by the file's last-modified time compared to the configured TTL.
- Only successful responses are cached; errors are never cached.
- The `clear_cache` tool can be used to manually invalidate all cached entries.

## Usage

Add the plugin to your Hyper MCP configuration:

```json
{
  "plugins": {
    "context7": {
      "url": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:latest",
      "runtime_config": {
        "allowed_hosts": ["context7.com"]
      }
    }
  }
}
```

For nightly builds:
```json
{
  "plugins": {
    "context7": {
      "url": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:nightly",
      "runtime_config": {
        "allowed_hosts": ["context7.com"]
      }
    }
  }
}
```

### Full Configuration Example

A complete configuration with API key (via keyring), caching, and custom TTL:

```json
{
  "plugins": {
    "context7": {
      "url": "oci://ghcr.io/hyper-mcp-rs/context7-plugin:latest",
      "runtime_config": {
        "allowed_hosts": ["context7.com"],
        "allowed_paths": ["/path/on/host/context7-cache:/cache"],
        "allowed_secrets": [
          {
            "service": "context7",
            "user": "api-key"
          }
        ],
        "env_vars": {
          "CONTEXT7_API_KEY": "{\"service\":\"context7\",\"user\":\"api-key\"}",
          "CACHE_TTL": "3"
        }
      }
    }
  }
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
- `score`: Search relevance score (optional)
- `vip`: VIP library flag (optional)
- `verified`: Verified library flag (optional)

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
      "versions": [],
      "score": 0.8,
      "vip": true,
      "verified": true
    }
  ]
}
```

### 2. `query_docs`

**Description:** Retrieves and queries up-to-date documentation and code examples from Context7 for any programming library or framework.

You must call `resolve_library_id` first to obtain the exact Context7-compatible library ID required to use this tool, UNLESS the user explicitly provides a library ID in the format `/org/project` or `/org/project/version` in their query.

This tool makes two parallel requests to the Context7 API:
- A **text** request (`type=txt`) for human-readable Markdown documentation
- A **JSON** request (`type=json`) for structured code snippets and documentation metadata

The Markdown is returned as the text content, and the structured JSON is returned as `structuredContent`.

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

**Text Output:**
Returns Markdown-formatted documentation and code examples relevant to your query.

**Structured Output:**
Returns a JSON object with the following structure:

- `codeSnippets`: Array of relevant code snippets, each containing:
  - `codeTitle`: Title of the code snippet
  - `codeDescription`: Description of what the code does
  - `codeLanguage`: Primary programming language
  - `codeTokens`: Token count for the snippet
  - `codeId`: URL to source location
  - `pageTitle`: Title of the documentation page
  - `codeList`: Array of code examples, each with `language` and `code`
- `infoSnippets`: Array of documentation snippets, each containing:
  - `pageId`: URL to source page
  - `breadcrumb`: Navigation breadcrumb path
  - `content`: The documentation content
  - `contentTokens`: Token count for the content
- `rules` (optional): Library-specific rules and guidelines
  - `global`: Global team rules
  - `libraryOwn`: Rules defined by the library owner
  - `libraryTeam`: Library-specific rules from the team

**Example Structured Output:**
```json
{
  "codeSnippets": [
    {
      "codeTitle": "Middleware Authentication Example",
      "codeDescription": "Shows how to implement authentication checks in Next.js middleware",
      "codeLanguage": "typescript",
      "codeTokens": 150,
      "codeId": "https://github.com/vercel/next.js/blob/canary/docs/middleware.mdx#_snippet_0",
      "pageTitle": "Middleware",
      "codeList": [
        {
          "language": "typescript",
          "code": "import { NextResponse } from 'next/server'\nimport type { NextRequest } from 'next/server'\n\nexport function middleware(request: NextRequest) {\n  const token = request.cookies.get('token')\n  if (!token) {\n    return NextResponse.redirect(new URL('/login', request.url))\n  }\n  return NextResponse.next()\n}"
        }
      ]
    }
  ],
  "infoSnippets": [
    {
      "pageId": "https://github.com/vercel/next.js/blob/canary/docs/middleware.mdx",
      "breadcrumb": "Routing > Middleware",
      "content": "Middleware allows you to run code before a request is completed...",
      "contentTokens": 200
    }
  ]
}
```

### 3. `clear_cache`

**Description:** Clears the on-disk cache of Context7 API responses. Use this if you suspect cached results are stale or incorrect.

This tool takes no arguments.

**Behavior:**
- If caching is enabled, removes all `.json` cache files from the `/cache` directory
- If caching is not enabled (directory not mounted), returns an informational message
- Non-JSON files in the cache directory are left untouched
- Returns a success message with the count of removed entries, or an error if files could not be removed

**Example Output (success):**
```
Cache cleared successfully (5 entries removed)
```

**Example Output (cache not mounted):**
```
Cache is not enabled (directory not mounted)
```

## Development

### Building

Build the WASM plugin:
```bash
cargo build --release --target wasm32-wasip1
```

The compiled plugin will be available at `target/wasm32-wasip1/release/plugin.wasm`.

### Testing

The plugin includes comprehensive tests split across two test files:

```bash
# Run all tests (requires native target, not WASM)
cargo test --target $(rustc -vV | grep host | cut -d' ' -f2)
```

Or run individual test suites:

```bash
# API integration tests (makes real HTTP calls to Context7)
cargo test --test resolve_library_id_tests --target $(rustc -vV | grep host | cut -d' ' -f2)

# Cache functionality tests (local, no network required)
cargo test --test cache_tests --target $(rustc -vV | grep host | cut -d' ' -f2)
```

Or specify your target explicitly:
```bash
# macOS ARM
cargo test --target aarch64-apple-darwin

# macOS Intel
cargo test --target x86_64-apple-darwin

# Linux
cargo test --target x86_64-unknown-linux-gnu
```

#### API Integration Tests (`resolve_library_id_tests`)

Tests verify:
- ✅ Response deserialization with actual Context7 API calls
- ✅ Handling of popular libraries (React, Next.js, MongoDB)
- ✅ Empty/no results scenarios
- ✅ Optional field handling (including `score`, `vip`, `verified`)
- ✅ All DocumentState enum variants
- ✅ Error responses

#### Cache Tests (`cache_tests`)

Tests verify:
- ✅ Hash determinism (same args → same hash, different args → different hash)
- ✅ Cache path generation format (`{tool_name}_{hex_hash}.json`)
- ✅ `CallToolResult` serialization round-trip (text, structured, error, nested content)
- ✅ Cache put/get operations (hit, miss, overwrite)
- ✅ Cache miss for different args, different tool names, empty directories
- ✅ TTL / staleness detection (fresh entries returned, stale entries rejected)
- ✅ Zero TTL always treats entries as stale
- ✅ Cache clear (removes `.json` files, leaves non-JSON files)
- ✅ Clear-then-put (cache is reusable after clearing)
- ✅ Corrupted / malformed / empty / wrong-shape cache files handled gracefully

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
- **Query Docs (text):** `GET /v2/context?libraryId={id}&query={query}&type=txt`
- **Query Docs (JSON):** `GET /v2/context?libraryId={id}&query={query}&type=json`

### Request Headers
- `X-Context7-Source: hyper-mcp/context7-plugin`
- `X-Context7-Server-Version: {version}`
- `Authorization: Bearer {api_key}` (when API key is configured)

## Privacy & Security

**Important:** Do not include sensitive or confidential information in your queries, including:
- API keys, passwords, or credentials
- Personal data
- Proprietary code
- Trade secrets

All queries are sent to the Context7 API for processing.

## License

See [LICENSE](LICENSE) for details.
