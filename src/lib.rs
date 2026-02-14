mod pdk;

use anyhow::Result;
use extism_pdk::*;
use pdk::types::*;
use schemars::{JsonSchema, schema_for};
use serde_json::{Map, Value, json, map};
use url::Url;

const CONTEXT7_API_BASE_URL: &str = "https://context7.com/api"; // Guessed API base URL

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[encoding(Json)]
struct ResolveLibraryIdArguments {
    #[schemars(
        description = "Library name to search for and retrieve a Context7-compatible library ID."
    )]
    #[serde(rename = "libraryName")]
    library_name: String,

    #[schemars(
        description = "The question or task you need help with. This is used to rank library results \
        by relevance to what the user is trying to accomplish. The query is sent to the Context7 API for processing. \
        Do not include any sensitive or confidential information such as API keys, passwords, credentials, personal data, \
        or proprietary code in your query."
    )]
    query: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[encoding(Json)]
#[serde(rename_all = "lowercase")]
enum DocumentState {
    Delete,
    Error,
    Finalized,
    Initial,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[encoding(Json)]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[encoding(Json)]
struct ResolveLibraryIdResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    results: Vec<Library>,
}

struct QueryDocsArguments {
    #[schemars(
        description = "Exact Context7-compatible library ID (e.g., '/mongodb/docs', '/vercel/next.js', '/supabase/supabase', \
        '/vercel/next.js/v14.3.0-canary.87') retrieved from 'resolve_library_id' or directly from user query in the format '/org/project' \
        or '/org/project/version'."
    )]
    #[serde(rename = "libraryId")]
    library_id: String,

    #[schemar(
        description = "The question or task you need help with. Be specific and include relevant details. \
        Good: 'How to set up authentication with JWT in Express.js' or 'React useEffect cleanup function examples'. \
        Bad: 'auth' or 'hooks'. The query is sent to the Context7 API for processing. Do not include any sensitive or \
        confidential information such as API keys, passwords, credentials, personal data, or proprietary code in your query."
    )]
    query: String,
}

fn error_result(error: String) -> CallToolResult {
    CallToolResult {
        is_error: Some(true),
        content: vec![ContentBlock::Text(TextContent {
            text: e.to_string(),
            ..Default::default()
        })],

        ..Default::default()
    }
}

pub(crate) fn call_tool(input: CallToolRequest) -> Result<CallToolResult> {
    match input.request.name.as_str() {
        "resolve_library_id" => resolve_library_id(input),
        "query_docs" => query_docs(input),
        _ => Ok(error_result(format!("Unknown tool: {}", input.params.name))),
    }
}

pub(crate) fn list_tools(_input: ListToolsRequest) -> Result<ListToolsResult> {
    let schema = schema_for!(ResolveLibraryIdArguments);
    Ok(ListToolsResult {
        tools: vec![
            Tool {
              name: "query_docs".to_string(),
              annotations: Some(ToolAnnotations{
                  read_only_hint: Some(true),

                  ..Default::default()
              }),
              description: Some(
                  r#"Retrieves and queries up-to-date documentation and code examples from Context7 for any programming library or framework.

                  You must call 'resolve_library_id' first to obtain the exact Context7-compatible library ID required to use this tool, UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query.

                  IMPORTANT: Do not call this tool more than 3 times per question. If you cannot find what you need after 3 calls, use the best information you have."#.to_string()
              ),
              input_schema: schema_for!(QueryDocsArguments),
              title: Some("Query Documentation".to_string()),

              ..Default::default()
            },
            Tool {
                name: "resolve_library_id".to_string(),
                annotations: Some(ToolAnnotations{
                    read_only_hint: Some(true),

                    ..Default::default()
                }),
                description: Some(
                r#"Resolves a package/product name to a Context7-compatible library ID and returns matching libraries.

                You MUST call this function before 'query_docs' to obtain a valid Context7-compatible library ID UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query.

                Selection Process:
                1. Analyze the query to understand what library/package the user is looking for
                2. Return the most relevant match based on:
                - Name similarity to the query (exact matches prioritized)
                - Description relevance to the query's intent
                - Documentation coverage (prioritize libraries with higher Code Snippet counts)
                - Source reputation (consider libraries with High or Medium reputation more authoritative)
                - Benchmark Score: Quality indicator (100 is the highest score)

                Response Format:
                - Return the selected library ID in a clearly marked section
                - Provide a brief explanation for why this library was chosen
                - If multiple good matches exist, acknowledge this but proceed with the most relevant one
                - If no good matches exist, clearly state this and suggest query refinements

                For ambiguous queries, request clarification before proceeding with a best-guess match.

                IMPORTANT: Do not call this tool more than 3 times per question. If you cannot find what you need after 3 calls, use the best result you have."#.to_string(),
                ),
                input_schema: schema_for!(ResolveLibraryIdArguments),
                output_schema: Some(schema_for!(ResolveLibraryIdResponse)),
                title: Some("Resolve Context7 Library ID".to_string()),

                ..Default::default()
            }
        ],
    })
}

fn query_docs(input: CallToolRequest) -> Result<CallToolResult> {
    let args: QueryDocsArguments =
        serde_json::from_value(Value::Object(input.request.arguments.unwrap_or_default()))?;
    let mut url = match Url::parse(&format!("{}/v2/context", CONTEXT7_API_BASE_URL)) {
        Ok(url) => url,
        Err(e) => {
            return Ok(error_result(e.to_string()));
        }
    };
    url.query_pairs_mut()
        .append_pair("libraryId", &args.library_id)
        .append_pair("query", &args.query);

    let mut req = HttpRequest::new(&url).with_method("GET");
    req.headers.insert(
        "X-Context7-Source".to_string(),
        "hyper-mcp/context7-plugin".to_string(),
    );
    req.headers.insert(
        "X-Context7-Server-Version".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    // TODO: Add authorization, gotten from keyring secret

    match http::request::<()>(&req, None) {
        Ok(res) => {
            let body = String::from_utf8_lossy(&res.body()).to_string();
            if res.status_code() >= 200 && res.status_code() < 300 {
                Ok(CallToolResult {
                    content: vec![ContentBlock::Text(TextContent {
                        text: body,

                        ..Default::default()
                    })],

                    ..Default::default()
                })
            } else {
                Ok(error_result(format!(
                    "API request failed with status {}: {}",
                    res.status_code(),
                    body,
                )))
            }
        }
        Err(e) => Ok(error_result(e.to_string())),
    }
}

fn resolve_library_id(input: CallToolRequest) -> Result<CallToolResult> {
    let args: ResolveLibraryIdArguments =
        serde_json::from_value(Value::Object(input.request.arguments.unwrap_or_default()))?;
    let mut url = match Url::parse(&format!("{}/v2/libs/search", CONTEXT7_API_BASE_URL)) {
        Ok(url) => url,
        Err(e) => {
            return Ok(error_result(e.to_string()));
        }
    };
    url.query_pairs_mut()
        .append_pair("libraryName", &args.library_name)
        .append_pair("query", &args.query);

    let mut req = HttpRequest::new(&url).with_method("GET");
    req.headers.insert(
        "X-Context7-Source".to_string(),
        "hyper-mcp/context7-plugin".to_string(),
    );
    req.headers.insert(
        "X-Context7-Server-Version".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    // TODO: Add authorization, gotten from keyring secret

    match http::request::<()>(&req, None) {
        Ok(res) => {
            if res.status_code() >= 200 && res.status_code() < 300 {
                match serde_json::from_str::<ResolveLibraryIdResponse>(&body_str) {
                    Ok(context7_response) => {
                        let mut call_tool_result = CallToolResult {
                            content: vec![ContentBlock::Text(TextContent {
                                text: body_str,

                                ..Default::default()
                            })],

                            ..Default::default()
                        };
                        match serde_json::to_value(context7_response) {
                            Ok(value) => match value {
                                Value::Object(map) => {
                                    call_tool_result.structured_content = Some(map);
                                }
                                _ => {}
                            },
                            Err(_) => {}
                        }

                        Ok(call_tool_result)
                    }
                    Err(e) => Ok(error_result(e.to_string())),
                }
            } else {
                Ok(error_result(format!(
                    "API request failed with status {}: {}",
                    res.status_code(),
                    String::from_utf8_lossy(&res.body()).to_string(),
                )))
            }
        }
        Err(e) => Ok(error_result(e.to_string())),
    }
}
