use serde::{Deserialize, Serialize};
use url::Url;

const CONTEXT7_API_BASE_URL: &str = "https://context7.com/api";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DocumentState {
    #[default]
    Initial,
    Delete,
    Error,
    Finalized,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct Library {
    id: String,
    title: String,
    description: String,
    branch: String,
    #[serde(rename = "lastUpdateDate")]
    last_update_date: String,
    state: DocumentState,
    #[serde(rename = "totalTokens")]
    total_tokens: f64,
    #[serde(rename = "totalSnippets")]
    total_snippets: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    stars: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "trustScore")]
    trust_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "benchmarkScore")]
    benchmark_score: Option<f64>,
    #[serde(default)]
    versions: Vec<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct ResolveLibraryIdResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    results: Vec<Library>,
}

/// Test deserialization of ResolveLibraryIdResponse with actual results
/// Uses a real Context7 API call with data that should return results
#[tokio::test]
async fn test_resolve_library_id_response_with_results() {
    // Use a popular library that should have results
    let library_name = "react";
    let query = "How to use hooks in React";

    let mut url = Url::parse(&format!("{}/v2/libs/search", CONTEXT7_API_BASE_URL))
        .expect("Failed to parse base URL");
    url.query_pairs_mut()
        .append_pair("libraryName", library_name)
        .append_pair("query", query);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("X-Context7-Source", "hyper-mcp/context7-plugin")
        .header("X-Context7-Server-Version", env!("CARGO_PKG_VERSION"))
        .send()
        .await
        .expect("Failed to send request to Context7 API");

    assert!(
        response.status().is_success(),
        "API request failed with status: {}",
        response.status()
    );

    let body = response.text().await.expect("Failed to read response body");

    println!("Response body: {}", body);

    // Test deserialization
    let deserialized: ResolveLibraryIdResponse =
        serde_json::from_str(&body).expect("Failed to deserialize ResolveLibraryIdResponse");

    // Verify the structure
    assert!(
        deserialized.error.is_none(),
        "Expected no error in response"
    );
    assert!(
        !deserialized.results.is_empty(),
        "Expected at least one result for popular library 'react'"
    );

    // Verify first result has expected fields
    let first_result = &deserialized.results[0];
    assert!(
        !first_result.id.is_empty(),
        "Library id should not be empty"
    );
    assert!(
        !first_result.title.is_empty(),
        "Library title should not be empty"
    );
    assert!(
        !first_result.branch.is_empty(),
        "Library branch should not be empty"
    );
    assert!(
        first_result.total_tokens > 0.0,
        "Library should have tokens"
    );

    println!(
        "Successfully deserialized {} results",
        deserialized.results.len()
    );
    println!("First result ID: {}", first_result.id);
    println!("First result title: {}", first_result.title);
    println!("First result description: {}", first_result.description);
}

/// Test deserialization of ResolveLibraryIdResponse with various query results
/// Uses a real Context7 API call to verify deserialization works regardless of result count
#[tokio::test]
async fn test_resolve_library_id_response_with_any_results() {
    // Use an unusual library name that may or may not have results
    // The API may return fuzzy matches, so we just test deserialization works
    let library_name = "zxcvbnmasdfghjklqwertyuiop12345nonexistent";
    let query = "some random query";

    let mut url = Url::parse(&format!("{}/v2/libs/search", CONTEXT7_API_BASE_URL))
        .expect("Failed to parse base URL");
    url.query_pairs_mut()
        .append_pair("libraryName", library_name)
        .append_pair("query", query);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("X-Context7-Source", "hyper-mcp/context7-plugin")
        .header("X-Context7-Server-Version", env!("CARGO_PKG_VERSION"))
        .send()
        .await
        .expect("Failed to send request to Context7 API");

    assert!(
        response.status().is_success(),
        "API request failed with status: {}",
        response.status()
    );

    let body = response.text().await.expect("Failed to read response body");

    println!("Response body for unusual query: {}", body);

    // Test deserialization - the key is that it deserializes successfully
    let deserialized: ResolveLibraryIdResponse =
        serde_json::from_str(&body).expect("Failed to deserialize ResolveLibraryIdResponse");

    // Verify the structure is valid (error field is optional)
    assert!(
        deserialized.error.is_none(),
        "Expected no error in response"
    );

    println!(
        "Successfully deserialized response with {} results (API may return fuzzy matches)",
        deserialized.results.len()
    );
}

/// Test deserialization with another popular library (Next.js)
#[tokio::test]
async fn test_resolve_library_id_response_nextjs() {
    let library_name = "next.js";
    let query = "server-side rendering with Next.js";

    let mut url = Url::parse(&format!("{}/v2/libs/search", CONTEXT7_API_BASE_URL))
        .expect("Failed to parse base URL");
    url.query_pairs_mut()
        .append_pair("libraryName", library_name)
        .append_pair("query", query);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("X-Context7-Source", "hyper-mcp/context7-plugin")
        .header("X-Context7-Server-Version", env!("CARGO_PKG_VERSION"))
        .send()
        .await
        .expect("Failed to send request to Context7 API");

    assert!(
        response.status().is_success(),
        "API request failed with status: {}",
        response.status()
    );

    let body = response.text().await.expect("Failed to read response body");

    // Test deserialization
    let deserialized: ResolveLibraryIdResponse = serde_json::from_str(&body)
        .expect("Failed to deserialize ResolveLibraryIdResponse for Next.js");

    assert!(
        deserialized.error.is_none(),
        "Expected no error in response"
    );

    if !deserialized.results.is_empty() {
        let first_result = &deserialized.results[0];
        println!("Next.js result ID: {}", first_result.id);
        println!("Next.js result title: {}", first_result.title);
        println!("Next.js result versions: {:?}", first_result.versions);
        println!("Next.js branch: {}", first_result.branch);
        println!("Next.js total tokens: {}", first_result.total_tokens);
        println!("Next.js total snippets: {}", first_result.total_snippets);

        // Verify optional fields can be handled
        if let Some(stars) = first_result.stars {
            println!("Stars: {}", stars);
        }
        if let Some(trust_score) = first_result.trust_score {
            println!("Trust score: {}", trust_score);
        }
        if let Some(benchmark_score) = first_result.benchmark_score {
            println!("Benchmark score: {}", benchmark_score);
        }
    }

    println!(
        "Successfully tested Next.js with {} results",
        deserialized.results.len()
    );
}

/// Test deserialization with MongoDB library
#[tokio::test]
async fn test_resolve_library_id_response_mongodb() {
    let library_name = "mongodb";
    let query = "connecting to MongoDB database";

    let mut url = Url::parse(&format!("{}/v2/libs/search", CONTEXT7_API_BASE_URL))
        .expect("Failed to parse base URL");
    url.query_pairs_mut()
        .append_pair("libraryName", library_name)
        .append_pair("query", query);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("X-Context7-Source", "hyper-mcp/context7-plugin")
        .header("X-Context7-Server-Version", env!("CARGO_PKG_VERSION"))
        .send()
        .await
        .expect("Failed to send request to Context7 API");

    assert!(
        response.status().is_success(),
        "API request failed with status: {}",
        response.status()
    );

    let body = response.text().await.expect("Failed to read response body");

    // Test deserialization
    let deserialized: ResolveLibraryIdResponse = serde_json::from_str(&body)
        .expect("Failed to deserialize ResolveLibraryIdResponse for MongoDB");

    println!("MongoDB results count: {}", deserialized.results.len());

    // Print all results for debugging
    for (idx, result) in deserialized.results.iter().enumerate() {
        println!(
            "Result {}: ID={}, Title={}",
            idx + 1,
            result.id,
            result.title
        );
    }
}

/// Test that all DocumentState variants can be deserialized
#[test]
fn test_document_state_deserialization() {
    let test_cases = vec![
        (r#""delete""#, "Delete"),
        (r#""error""#, "Error"),
        (r#""finalized""#, "Finalized"),
        (r#""initial""#, "Initial"),
    ];

    for (json_str, expected_variant) in test_cases {
        let deserialized: DocumentState = serde_json::from_str(json_str).unwrap_or_else(|e| {
            panic!(
                "Failed to deserialize DocumentState from {}: {}",
                json_str, e
            )
        });

        let variant_name = match deserialized {
            DocumentState::Delete => "Delete",
            DocumentState::Error => "Error",
            DocumentState::Finalized => "Finalized",
            DocumentState::Initial => "Initial",
        };

        assert_eq!(
            variant_name, expected_variant,
            "Expected {:?} but got {:?}",
            expected_variant, variant_name
        );
    }

    println!("All DocumentState variants deserialized successfully");
}

/// Test that a complete Library object can be deserialized from JSON
#[test]
fn test_library_deserialization_complete() {
    let json = r#"{
        "id": "/facebook/react",
        "title": "React",
        "description": "A JavaScript library for building user interfaces",
        "branch": "main",
        "lastUpdateDate": "2024-01-15T10:30:00Z",
        "state": "finalized",
        "totalTokens": 150000.0,
        "totalSnippets": 500.0,
        "stars": 200000.0,
        "trustScore": 95.5,
        "benchmarkScore": 98.0,
        "versions": ["v18.0.0", "v17.0.0"]
    }"#;

    let library: Library =
        serde_json::from_str(json).expect("Failed to deserialize complete Library object");

    assert_eq!(library.id, "/facebook/react");
    assert_eq!(library.title, "React");
    assert_eq!(library.branch, "main");
    assert_eq!(library.total_tokens, 150000.0);
    assert_eq!(library.stars, Some(200000.0));
    assert_eq!(library.trust_score, Some(95.5));
    assert_eq!(library.benchmark_score, Some(98.0));
    assert_eq!(library.versions.len(), 2);

    println!("Successfully deserialized complete Library object");
}

/// Test that a minimal Library object can be deserialized from JSON
#[test]
fn test_library_deserialization_minimal() {
    let json = r#"{
        "id": "/test/library",
        "title": "Test Library",
        "description": "A test library",
        "branch": "main",
        "lastUpdateDate": "2024-01-15T10:30:00Z",
        "state": "initial",
        "totalTokens": 1000.0,
        "totalSnippets": 10.0
    }"#;

    let library: Library =
        serde_json::from_str(json).expect("Failed to deserialize minimal Library object");

    assert_eq!(library.id, "/test/library");
    assert_eq!(library.title, "Test Library");
    assert!(library.stars.is_none());
    assert!(library.trust_score.is_none());
    assert!(library.benchmark_score.is_none());
    assert!(library.versions.is_empty());

    println!("Successfully deserialized minimal Library object");
}

/// Test that ResolveLibraryIdResponse can handle error responses
#[test]
fn test_resolve_library_id_response_with_error() {
    let json = r#"{
        "error": "Invalid request parameters",
        "results": []
    }"#;

    let response: ResolveLibraryIdResponse = serde_json::from_str(json)
        .expect("Failed to deserialize ResolveLibraryIdResponse with error");

    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap(), "Invalid request parameters");
    assert!(response.results.is_empty());

    println!("Successfully deserialized error response");
}

/// Test that ResolveLibraryIdResponse can handle empty results (no error, just no matches)
#[test]
fn test_resolve_library_id_response_empty_results() {
    let json = r#"{
        "results": []
    }"#;

    let response: ResolveLibraryIdResponse = serde_json::from_str(json)
        .expect("Failed to deserialize ResolveLibraryIdResponse with empty results");

    assert!(response.error.is_none());
    assert!(response.results.is_empty());

    println!("Successfully deserialized response with no results and no error");
}

/// Test that ResolveLibraryIdResponse can handle multiple results
#[test]
fn test_resolve_library_id_response_multiple_results() {
    let json = r#"{
        "results": [
            {
                "id": "/org1/lib1",
                "title": "Library 1",
                "description": "First library",
                "branch": "main",
                "lastUpdateDate": "2024-01-15T10:30:00Z",
                "state": "finalized",
                "totalTokens": 1000.0,
                "totalSnippets": 10.0
            },
            {
                "id": "/org2/lib2",
                "title": "Library 2",
                "description": "Second library",
                "branch": "develop",
                "lastUpdateDate": "2024-01-16T11:30:00Z",
                "state": "finalized",
                "totalTokens": 2000.0,
                "totalSnippets": 20.0
            }
        ]
    }"#;

    let response: ResolveLibraryIdResponse = serde_json::from_str(json)
        .expect("Failed to deserialize ResolveLibraryIdResponse with multiple results");

    assert!(response.error.is_none());
    assert_eq!(response.results.len(), 2);
    assert_eq!(response.results[0].id, "/org1/lib1");
    assert_eq!(response.results[1].id, "/org2/lib2");

    println!("Successfully deserialized response with multiple results");
}
