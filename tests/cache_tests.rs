use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Duplicated types from pdk::types that we need for native tests.
// We only model the subset actually used by the cache (Text content blocks).
// ---------------------------------------------------------------------------

type Meta = Map<String, Value>;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Annotations {
    pub audience: Vec<String>,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
    pub priority: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum ContentBlock {
    Text(TextContent),
    Empty(Empty),
}

impl Default for ContentBlock {
    fn default() -> Self {
        ContentBlock::Empty(Empty {})
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Empty {}

// TextContent uses the same tagged serialization as the real pdk type.
// The pdk implementation manually injects `"type": "text"` during
// serialization, so we replicate that here so round-trip JSON matches.
#[derive(Default, Debug, Clone, PartialEq)]
struct TextContent {
    pub meta: Option<Meta>,
    pub annotations: Option<Annotations>,
    pub text: String,
}

impl Serialize for TextContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a> {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: &'a Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            annotations: &'a Option<Annotations>,
            text: &'a String,
            r#type: &'static str,
        }

        let helper = Helper {
            meta: &self.meta,
            annotations: &self.annotations,
            text: &self.text,
            r#type: "text",
        };
        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TextContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(rename = "_meta")]
            #[serde(default)]
            meta: Option<Meta>,
            #[serde(default)]
            annotations: Option<Annotations>,
            text: String,
            #[allow(dead_code)]
            r#type: Option<String>,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(TextContent {
            meta: helper.meta,
            annotations: helper.annotations,
            text: helper.text,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CallToolResult {
    #[serde(rename = "_meta")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub meta: Option<Meta>,

    pub content: Vec<ContentBlock>,

    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub is_error: Option<bool>,

    #[serde(rename = "structuredContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub structured_content: Option<Map<String, Value>>,
}

impl CallToolResult {
    fn error(msg: String) -> Self {
        CallToolResult {
            is_error: Some(true),
            content: vec![ContentBlock::Text(TextContent {
                text: msg,
                ..Default::default()
            })],
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Duplicated argument types (must match Hash behaviour from types.rs)
// ---------------------------------------------------------------------------

#[derive(Default, Debug, Clone, Hash, Serialize, Deserialize)]
struct ResolveLibraryIdArguments {
    #[serde(rename = "libraryName")]
    pub library_name: String,
    pub query: String,
}

#[derive(Default, Debug, Clone, Hash, Serialize, Deserialize)]
struct QueryDocsArguments {
    #[serde(rename = "libraryId")]
    pub library_id: String,
    pub query: String,
}

// ---------------------------------------------------------------------------
// Replicated cache helpers that mirror cache.rs but accept a configurable
// cache directory and TTL so we can test without the PDK runtime.
// ---------------------------------------------------------------------------

fn compute_hash<T: Hash>(args: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    args.hash(&mut hasher);
    hasher.finish()
}

fn cache_path<T: Hash>(cache_dir: &Path, tool_name: &str, args: &T) -> PathBuf {
    let hash = compute_hash(args);
    cache_dir.join(format!("{}_{:x}.json", tool_name, hash))
}

fn is_fresh(path: &Path, ttl: Duration) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = std::time::SystemTime::now().duration_since(modified) else {
        return false;
    };
    elapsed < ttl
}

fn cache_get<T: Hash>(
    cache_dir: &Path,
    tool_name: &str,
    args: &T,
    ttl: Duration,
) -> Option<CallToolResult> {
    let path = cache_path(cache_dir, tool_name, args);
    if !is_fresh(&path, ttl) {
        return None;
    }
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

fn cache_put<T: Hash>(cache_dir: &Path, tool_name: &str, args: &T, result: &CallToolResult) {
    let path = cache_path(cache_dir, tool_name, args);
    let data = serde_json::to_string(result).expect("Failed to serialize CallToolResult");
    fs::write(&path, data).expect("Failed to write cache file");
}

fn cache_clear(cache_dir: &Path) -> (u64, Vec<String>) {
    let entries = fs::read_dir(cache_dir).expect("Failed to read cache dir");
    let mut removed = 0u64;
    let mut errors = Vec::new();

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            match fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(e) => errors.push(format!("{}: {}", path.display(), e)),
            }
        }
    }
    (removed, errors)
}

// ---------------------------------------------------------------------------
// Helper to build simple CallToolResult values for testing
// ---------------------------------------------------------------------------

fn make_text_result(text: &str) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: text.to_string(),
            ..Default::default()
        })],
        ..Default::default()
    }
}

fn make_structured_result(text: &str, key: &str, value: &str) -> CallToolResult {
    let mut map = Map::new();
    map.insert(key.to_string(), Value::String(value.to_string()));
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: text.to_string(),
            ..Default::default()
        })],
        structured_content: Some(map),
        ..Default::default()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

// --- Hash determinism ---

#[test]
fn test_hash_determinism_same_args() {
    let args1 = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "server-side rendering".to_string(),
    };
    let args2 = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "server-side rendering".to_string(),
    };
    assert_eq!(
        compute_hash(&args1),
        compute_hash(&args2),
        "Identical arguments must produce the same hash"
    );
}

#[test]
fn test_hash_determinism_different_query() {
    let args1 = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "server-side rendering".to_string(),
    };
    let args2 = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "client-side rendering".to_string(),
    };
    assert_ne!(
        compute_hash(&args1),
        compute_hash(&args2),
        "Different queries must produce different hashes"
    );
}

#[test]
fn test_hash_determinism_different_library() {
    let args1 = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "routing".to_string(),
    };
    let args2 = QueryDocsArguments {
        library_id: "/facebook/react".to_string(),
        query: "routing".to_string(),
    };
    assert_ne!(
        compute_hash(&args1),
        compute_hash(&args2),
        "Different library IDs must produce different hashes"
    );
}

#[test]
fn test_hash_determinism_resolve_library_id_args() {
    let args1 = ResolveLibraryIdArguments {
        library_name: "react".to_string(),
        query: "hooks".to_string(),
    };
    let args2 = ResolveLibraryIdArguments {
        library_name: "react".to_string(),
        query: "hooks".to_string(),
    };
    assert_eq!(compute_hash(&args1), compute_hash(&args2));
}

#[test]
fn test_hash_different_arg_types_differ() {
    // Even if the string content is similar, the struct types differ so
    // hashes should generally differ (fields are in different order / names).
    let query_args = QueryDocsArguments {
        library_id: "react".to_string(),
        query: "hooks".to_string(),
    };
    let resolve_args = ResolveLibraryIdArguments {
        library_name: "react".to_string(),
        query: "hooks".to_string(),
    };
    // We can't guarantee they differ (Hash is not cryptographic), but
    // the tool_name prefix in cache_path will disambiguate regardless.
    let path1 = cache_path(Path::new("/cache"), "query_docs", &query_args);
    let path2 = cache_path(Path::new("/cache"), "resolve_library_id", &resolve_args);
    assert_ne!(
        path1, path2,
        "Different tool names must produce different cache paths"
    );
}

// --- Cache path generation ---

#[test]
fn test_cache_path_format() {
    let args = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "middleware".to_string(),
    };
    let path = cache_path(Path::new("/cache"), "query_docs", &args);
    let filename = path.file_name().unwrap().to_str().unwrap();

    assert!(
        filename.starts_with("query_docs_"),
        "Cache filename should start with tool name: {}",
        filename
    );
    assert!(
        filename.ends_with(".json"),
        "Cache filename should end with .json: {}",
        filename
    );
    // The middle part should be a hex hash
    let hex_part = &filename["query_docs_".len()..filename.len() - ".json".len()];
    assert!(!hex_part.is_empty(), "Hash portion should not be empty");
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "Hash portion should be hex: {}",
        hex_part
    );
}

#[test]
fn test_cache_path_uses_tool_name_prefix() {
    let args = ResolveLibraryIdArguments {
        library_name: "react".to_string(),
        query: "hooks".to_string(),
    };
    let path = cache_path(Path::new("/tmp/test_cache"), "resolve_library_id", &args);
    assert!(
        path.to_str()
            .unwrap()
            .starts_with("/tmp/test_cache/resolve_library_id_")
    );
}

// --- CallToolResult serialization round-trip ---

#[test]
fn test_call_tool_result_text_round_trip() {
    let result = make_text_result("Hello, world!");
    let json = serde_json::to_string(&result).expect("serialize");
    let deserialized: CallToolResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(result, deserialized);
}

#[test]
fn test_call_tool_result_structured_round_trip() {
    let result = make_structured_result("some text", "codeSnippets", "[]");
    let json = serde_json::to_string(&result).expect("serialize");
    let deserialized: CallToolResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(result, deserialized);
}

#[test]
fn test_call_tool_result_error_round_trip() {
    let result = CallToolResult::error("something went wrong".to_string());
    let json = serde_json::to_string(&result).expect("serialize");
    let deserialized: CallToolResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(result.is_error, deserialized.is_error);
    assert_eq!(result.content.len(), deserialized.content.len());
}

#[test]
fn test_call_tool_result_with_nested_structured_content_round_trip() {
    let mut map = Map::new();
    map.insert(
        "codeSnippets".to_string(),
        serde_json::json!([{
            "codeTitle": "Example",
            "codeDescription": "An example snippet",
            "codeLanguage": "rust",
            "codeTokens": 42,
            "codeId": "https://example.com/snippet",
            "pageTitle": "Docs",
            "codeList": [{"language": "rust", "code": "fn main() {}"}]
        }]),
    );
    map.insert(
        "infoSnippets".to_string(),
        serde_json::json!([{
            "pageId": "https://example.com/page",
            "breadcrumb": "Docs > Example",
            "content": "Some documentation content",
            "contentTokens": 10
        }]),
    );

    let result = CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: "markdown content here".to_string(),
            ..Default::default()
        })],
        structured_content: Some(map.clone()),
        ..Default::default()
    };

    let json = serde_json::to_string(&result).expect("serialize");
    let deserialized: CallToolResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(result.structured_content, deserialized.structured_content);
}

// --- Cache put / get ---

#[test]
fn test_cache_put_then_get() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "middleware".to_string(),
    };
    let result = make_text_result("cached documentation");
    let ttl = Duration::from_secs(3600);

    cache_put(dir.path(), "query_docs", &args, &result);
    let cached = cache_get(dir.path(), "query_docs", &args, ttl);

    assert!(cached.is_some(), "Should get a cache hit after put");
    assert_eq!(cached.unwrap(), result);
}

#[test]
fn test_cache_put_then_get_with_structured_content() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "middleware".to_string(),
    };
    let result = make_structured_result("markdown text", "key", "value");
    let ttl = Duration::from_secs(3600);

    cache_put(dir.path(), "query_docs", &args, &result);
    let cached = cache_get(dir.path(), "query_docs", &args, ttl).unwrap();

    assert_eq!(cached.structured_content, result.structured_content);
}

#[test]
fn test_cache_miss_on_empty_directory() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "middleware".to_string(),
    };
    let ttl = Duration::from_secs(3600);

    let cached = cache_get(dir.path(), "query_docs", &args, ttl);
    assert!(cached.is_none(), "Empty cache should return None");
}

#[test]
fn test_cache_miss_on_different_args() {
    let dir = TempDir::new().unwrap();
    let args1 = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "middleware".to_string(),
    };
    let args2 = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "routing".to_string(),
    };
    let result = make_text_result("cached for middleware");
    let ttl = Duration::from_secs(3600);

    cache_put(dir.path(), "query_docs", &args1, &result);
    let cached = cache_get(dir.path(), "query_docs", &args2, ttl);

    assert!(
        cached.is_none(),
        "Different args should not produce a cache hit"
    );
}

#[test]
fn test_cache_miss_on_different_tool_name() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "middleware".to_string(),
    };
    let result = make_text_result("cached content");
    let ttl = Duration::from_secs(3600);

    cache_put(dir.path(), "query_docs", &args, &result);
    let cached = cache_get(dir.path(), "resolve_library_id", &args, ttl);

    assert!(
        cached.is_none(),
        "Different tool names should not share cache entries"
    );
}

#[test]
fn test_cache_put_overwrites_existing() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/vercel/next.js".to_string(),
        query: "middleware".to_string(),
    };
    let ttl = Duration::from_secs(3600);

    let result1 = make_text_result("first version");
    cache_put(dir.path(), "query_docs", &args, &result1);

    let result2 = make_text_result("second version");
    cache_put(dir.path(), "query_docs", &args, &result2);

    let cached = cache_get(dir.path(), "query_docs", &args, ttl).unwrap();
    assert_eq!(cached, result2, "Should return the latest cached value");
}

#[test]
fn test_cache_resolve_library_id() {
    let dir = TempDir::new().unwrap();
    let args = ResolveLibraryIdArguments {
        library_name: "react".to_string(),
        query: "hooks".to_string(),
    };
    let result = make_structured_result(r#"{"results":[]}"#, "results", "[]");
    let ttl = Duration::from_secs(3600);

    cache_put(dir.path(), "resolve_library_id", &args, &result);
    let cached = cache_get(dir.path(), "resolve_library_id", &args, ttl);

    assert!(cached.is_some(), "Should cache resolve_library_id results");
    assert_eq!(cached.unwrap(), result);
}

// --- Staleness ---

#[test]
fn test_cache_fresh_entry_is_returned() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "test".to_string(),
    };
    let result = make_text_result("fresh content");
    let ttl = Duration::from_secs(3600); // 1 hour

    cache_put(dir.path(), "query_docs", &args, &result);
    // Just written, so it should be fresh
    let cached = cache_get(dir.path(), "query_docs", &args, ttl);
    assert!(
        cached.is_some(),
        "Freshly written cache entry should be returned"
    );
}

#[test]
fn test_cache_stale_entry_is_not_returned() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "test".to_string(),
    };
    let result = make_text_result("will become stale");
    // Use a very short TTL
    let ttl = Duration::from_millis(50);

    cache_put(dir.path(), "query_docs", &args, &result);

    // Wait for the entry to become stale
    thread::sleep(Duration::from_millis(100));

    let cached = cache_get(dir.path(), "query_docs", &args, ttl);
    assert!(cached.is_none(), "Stale cache entry should not be returned");
}

#[test]
fn test_cache_zero_ttl_always_stale() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "test".to_string(),
    };
    let result = make_text_result("zero ttl content");
    let ttl = Duration::ZERO;

    cache_put(dir.path(), "query_docs", &args, &result);
    let cached = cache_get(dir.path(), "query_docs", &args, ttl);

    assert!(
        cached.is_none(),
        "Zero TTL should always treat entries as stale"
    );
}

#[test]
fn test_is_fresh_nonexistent_file() {
    assert!(
        !is_fresh(
            Path::new("/nonexistent/path/file.json"),
            Duration::from_secs(3600)
        ),
        "Non-existent file should not be fresh"
    );
}

#[test]
fn test_is_fresh_with_large_ttl() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.json");
    fs::write(&path, "{}").unwrap();

    let one_year = Duration::from_secs(365 * 24 * 60 * 60);
    assert!(
        is_fresh(&path, one_year),
        "Just-written file should be fresh with a large TTL"
    );
}

// --- Default TTL calculation ---

#[test]
fn test_default_cache_days_ttl() {
    // Verify the default TTL calculation: 1 day = 86400 seconds
    let default_days: u64 = 1;
    let ttl = Duration::from_secs(default_days * 24 * 60 * 60);
    assert_eq!(ttl.as_secs(), 86400);
}

#[test]
fn test_custom_cache_days_ttl() {
    // Verify TTL calculation for a custom number of days
    let days: u64 = 7;
    let ttl = Duration::from_secs(days * 24 * 60 * 60);
    assert_eq!(ttl.as_secs(), 604800);
}

// --- Cache clear ---

#[test]
fn test_clear_empty_cache() {
    let dir = TempDir::new().unwrap();
    let (removed, errors) = cache_clear(dir.path());

    assert_eq!(removed, 0, "Should remove 0 entries from empty cache");
    assert!(errors.is_empty(), "Should have no errors on empty cache");
}

#[test]
fn test_clear_removes_json_files() {
    let dir = TempDir::new().unwrap();
    let args1 = QueryDocsArguments {
        library_id: "/lib/one".to_string(),
        query: "query one".to_string(),
    };
    let args2 = QueryDocsArguments {
        library_id: "/lib/two".to_string(),
        query: "query two".to_string(),
    };

    cache_put(dir.path(), "query_docs", &args1, &make_text_result("one"));
    cache_put(dir.path(), "query_docs", &args2, &make_text_result("two"));

    let (removed, errors) = cache_clear(dir.path());

    assert_eq!(removed, 2, "Should remove 2 cache entries");
    assert!(errors.is_empty());

    // Verify files are gone
    let ttl = Duration::from_secs(3600);
    assert!(cache_get(dir.path(), "query_docs", &args1, ttl).is_none());
    assert!(cache_get(dir.path(), "query_docs", &args2, ttl).is_none());
}

#[test]
fn test_clear_leaves_non_json_files() {
    let dir = TempDir::new().unwrap();

    // Create a non-json file
    let non_json = dir.path().join("README.txt");
    fs::write(&non_json, "do not delete me").unwrap();

    // Create a cache entry
    let args = QueryDocsArguments {
        library_id: "/lib/test".to_string(),
        query: "test".to_string(),
    };
    cache_put(dir.path(), "query_docs", &args, &make_text_result("cached"));

    let (removed, errors) = cache_clear(dir.path());

    assert_eq!(removed, 1, "Should remove only the json file");
    assert!(errors.is_empty());
    assert!(
        non_json.exists(),
        "Non-JSON files should not be removed by clear"
    );
}

#[test]
fn test_clear_then_put_works() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/lib/test".to_string(),
        query: "test".to_string(),
    };
    let ttl = Duration::from_secs(3600);

    cache_put(
        dir.path(),
        "query_docs",
        &args,
        &make_text_result("before clear"),
    );
    cache_clear(dir.path());

    assert!(
        cache_get(dir.path(), "query_docs", &args, ttl).is_none(),
        "Cache should be empty after clear"
    );

    cache_put(
        dir.path(),
        "query_docs",
        &args,
        &make_text_result("after clear"),
    );
    let cached = cache_get(dir.path(), "query_docs", &args, ttl);

    assert!(cached.is_some(), "Should be able to cache after clearing");
    assert_eq!(cached.unwrap(), make_text_result("after clear"));
}

// --- Mixed tool cache entries ---

#[test]
fn test_clear_removes_all_tool_entries() {
    let dir = TempDir::new().unwrap();

    let query_args = QueryDocsArguments {
        library_id: "/lib/test".to_string(),
        query: "test".to_string(),
    };
    let resolve_args = ResolveLibraryIdArguments {
        library_name: "react".to_string(),
        query: "hooks".to_string(),
    };

    cache_put(
        dir.path(),
        "query_docs",
        &query_args,
        &make_text_result("docs"),
    );
    cache_put(
        dir.path(),
        "resolve_library_id",
        &resolve_args,
        &make_text_result("libs"),
    );

    let (removed, errors) = cache_clear(dir.path());
    assert_eq!(removed, 2);
    assert!(errors.is_empty());
}

// --- Cache file content verification ---

#[test]
fn test_cache_file_is_valid_json() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "check json".to_string(),
    };
    let result = make_structured_result("text content", "key", "value");

    cache_put(dir.path(), "query_docs", &args, &result);

    let path = cache_path(dir.path(), "query_docs", &args);
    let raw = fs::read_to_string(&path).expect("Should be able to read cache file");

    // Verify it's valid JSON
    let parsed: Value = serde_json::from_str(&raw).expect("Cache file should contain valid JSON");
    assert!(
        parsed.is_object(),
        "Cache file root should be a JSON object"
    );
}

#[test]
fn test_cache_file_contains_expected_fields() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "fields check".to_string(),
    };
    let result = make_structured_result("hello", "myKey", "myValue");

    cache_put(dir.path(), "query_docs", &args, &result);

    let path = cache_path(dir.path(), "query_docs", &args);
    let raw = fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();

    assert!(
        parsed.get("content").is_some(),
        "Cache file should have 'content' field"
    );
    assert!(
        parsed.get("structuredContent").is_some(),
        "Cache file should have 'structuredContent' field"
    );

    let sc = parsed.get("structuredContent").unwrap();
    assert_eq!(
        sc.get("myKey").and_then(|v| v.as_str()),
        Some("myValue"),
        "structuredContent should preserve the key/value pair"
    );
}

// --- Corrupted / malformed cache files ---

#[test]
fn test_cache_get_returns_none_for_corrupted_file() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "corrupted".to_string(),
    };
    let ttl = Duration::from_secs(3600);

    // Write garbage to the expected cache path
    let path = cache_path(dir.path(), "query_docs", &args);
    fs::write(&path, "this is not valid json!!!").unwrap();

    let cached = cache_get(dir.path(), "query_docs", &args, ttl);
    assert!(
        cached.is_none(),
        "Corrupted cache file should return None, not panic"
    );
}

#[test]
fn test_cache_get_returns_none_for_empty_file() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "empty".to_string(),
    };
    let ttl = Duration::from_secs(3600);

    let path = cache_path(dir.path(), "query_docs", &args);
    fs::write(&path, "").unwrap();

    let cached = cache_get(dir.path(), "query_docs", &args, ttl);
    assert!(cached.is_none(), "Empty cache file should return None");
}

#[test]
fn test_cache_get_returns_none_for_wrong_json_shape() {
    let dir = TempDir::new().unwrap();
    let args = QueryDocsArguments {
        library_id: "/test/lib".to_string(),
        query: "wrong shape".to_string(),
    };
    let ttl = Duration::from_secs(3600);

    // Valid JSON but not a CallToolResult
    let path = cache_path(dir.path(), "query_docs", &args);
    fs::write(&path, r#"{"unexpected": "structure"}"#).unwrap();

    let cached = cache_get(dir.path(), "query_docs", &args, ttl);
    assert!(cached.is_none(), "JSON with wrong shape should return None");
}
