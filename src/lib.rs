mod cache;
mod pdk;
mod types;

use crate::{
    pdk::imports::{get_keyring_secret, notify_logging_message},
    types::*,
};
use anyhow::Result;
use extism_pdk::*;
use pdk::types::*;
use schemars::schema_for;
use serde_json::{Map, Value, json};
use std::sync::OnceLock;
use url::Url;

const CONTEXT7_API_BASE_URL: &str = "https://context7.com/api";
static CONTEXT7_API_KEY: OnceLock<Option<String>> = OnceLock::new();

fn resolve_context7_api_key() -> Option<String> {
    let api_key = match config::get("CONTEXT7_API_KEY") {
        Ok(Some(item)) => match serde_json::from_str::<KeyringEntryId>(item.as_str()) {
            Ok(entry_id) => match get_keyring_secret(entry_id) {
                Ok(secret_bytes) => match String::from_utf8(secret_bytes) {
                    Ok(secret_str) => Some(secret_str),
                    Err(e) => {
                        notify_logging_message(LoggingMessageNotificationParam {
                            data: json!(e.to_string()),
                            level: LoggingLevel::Error,

                            ..Default::default()
                        })
                        .ok();
                        None
                    }
                },
                Err(_) => None,
            },
            Err(_) => Some(item),
        },
        Ok(None) => None,
        Err(e) => {
            notify_logging_message(LoggingMessageNotificationParam {
                data: json!(e.to_string()),
                level: LoggingLevel::Error,

                ..Default::default()
            })
            .ok();
            None
        }
    };
    if api_key.is_none() {
        notify_logging_message(LoggingMessageNotificationParam {
            data: json!(
                "Unable to resolve api key for Context7, using anonymous access".to_string()
            ),
            level: LoggingLevel::Info,

            ..Default::default()
        })
        .ok();
    }
    api_key
}

pub(crate) fn call_tool(input: CallToolRequest) -> Result<CallToolResult> {
    Ok(match input.request.name.as_str() {
        "resolve_library_id" => resolve_library_id(input),
        "query_docs" => query_docs(input),
        "clear_cache" => cache::clear(),
        _ => CallToolResult::error(format!("Unknown tool: {}", input.request.name)),
    })
}

pub(crate) fn list_tools(_input: ListToolsRequest) -> Result<ListToolsResult> {
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
              output_schema: Some(schema_for!(QueryDocsResponse)),
              title: Some("Query Documentation".to_string()),
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
            },
            Tool {
                name: "clear_cache".to_string(),
                annotations: Some(ToolAnnotations {
                    destructive_hint: Some(true),
                    read_only_hint: Some(false),

                    ..Default::default()
                }),
                description: Some(
                    "Clears the local documentation cache. Use this when cached results appear stale or outdated.".to_string(),
                ),
                input_schema: schema_for!(ClearCacheArguments),
                output_schema: None,
                title: Some("Clear Cache".to_string()),
            },
        ],
    })
}

trait Context7Headers {
    fn insert_context7_headers(self, context7_api_key: Option<&str>) -> Self;
}

impl Context7Headers for HttpRequest {
    fn insert_context7_headers(mut self, context7_api_key: Option<&str>) -> Self {
        self.headers.insert(
            "X-Context7-Source".to_string(),
            "hyper-mcp/context7-plugin".to_string(),
        );
        self.headers.insert(
            "X-Context7-Server-Version".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
        if let Some(api_key) = context7_api_key.map(Some).unwrap_or_else(|| {
            CONTEXT7_API_KEY
                .get_or_init(resolve_context7_api_key)
                .as_deref()
        }) {
            self.headers
                .insert("Authorization".to_string(), format!("Bearer {api_key}"));
        }
        self
    }
}

fn query_docs(input: CallToolRequest) -> CallToolResult {
    let mut args: QueryDocsArguments =
        match serde_json::from_value(Value::Object(input.request.arguments.unwrap_or_default())) {
            Ok(args) => args,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {e}")),
        };

    if args.r#type.is_none() {
        args.r#type = Some(QueryDocsType::Json);
    }

    if let Some(cached) = cache::get("query_docs", &args) {
        return cached;
    }

    let mut base_url = match Url::parse(&format!("{}/v2/context", CONTEXT7_API_BASE_URL)) {
        Ok(url) => url,
        Err(e) => {
            return CallToolResult::error(e.to_string());
        }
    };
    base_url
        .query_pairs_mut()
        .append_pair("libraryId", &args.library_id)
        .append_pair("query", &args.query);

    // Fetch text content if requested
    let content: Option<String> = if matches!(args.r#type, Some(QueryDocsType::Text)) {
        let mut txt_url = base_url.clone();
        txt_url.query_pairs_mut().append_pair("type", "txt");

        let txt_req = HttpRequest::new(txt_url.as_str())
            .with_method("GET")
            .insert_context7_headers(args.context7_api_key.as_deref());

        let res = match http::request::<()>(&txt_req, None) {
            Ok(res) => res,
            Err(e) => return CallToolResult::error(format!("Text request failed: {}", e)),
        };

        let body = String::from_utf8_lossy(&res.body()).to_string();
        if res.status_code() < 200 || res.status_code() >= 300 {
            return CallToolResult::error(format!(
                "Text API request failed with status {}: {}",
                res.status_code(),
                body,
            ));
        }

        Some(body)
    } else {
        None
    };

    // Fetch JSON content if requested (also the default when type is omitted)
    let structured_content: Option<Map<String, Value>> =
        if matches!(args.r#type, Some(QueryDocsType::Json)) {
            let mut json_url = base_url;
            json_url.query_pairs_mut().append_pair("type", "json");

            let json_req = HttpRequest::new(json_url.as_str())
                .with_method("GET")
                .insert_context7_headers(args.context7_api_key.as_deref());

            let res = match http::request::<()>(&json_req, None) {
                Ok(res) => res,
                Err(e) => return CallToolResult::error(format!("JSON request failed: {}", e)),
            };

            let body = String::from_utf8_lossy(&res.body()).to_string();
            if res.status_code() < 200 || res.status_code() >= 300 {
                return CallToolResult::error(format!(
                    "JSON API request failed with status {}: {}",
                    res.status_code(),
                    body,
                ));
            }

            let response: QueryDocsResponse = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    return CallToolResult::error(format!(
                        "Failed to deserialize JSON response: {}",
                        e
                    ));
                }
            };

            match serde_json::to_value(response) {
                Ok(Value::Object(map)) => Some(map),
                _ => {
                    return CallToolResult::error(
                        "Failed to convert QueryDocsResponse to JSON object".to_string(),
                    );
                }
            }
        } else {
            None
        };

    let result = CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: content.unwrap_or_else(|| {
                structured_content
                    .as_ref()
                    .and_then(|sc| serde_json::to_string(sc).ok())
                    .unwrap_or_default()
            }),
            ..Default::default()
        })],
        structured_content,
        ..Default::default()
    };

    cache::put("query_docs", &args, &result);
    result
}

fn resolve_library_id(input: CallToolRequest) -> CallToolResult {
    let args: ResolveLibraryIdArguments =
        match serde_json::from_value(Value::Object(input.request.arguments.unwrap_or_default())) {
            Ok(args) => args,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {e}")),
        };

    if let Some(cached) = cache::get("resolve_library_id", &args) {
        return cached;
    }

    let mut url = match Url::parse(&format!("{}/v2/libs/search", CONTEXT7_API_BASE_URL)) {
        Ok(url) => url,
        Err(e) => {
            return CallToolResult::error(e.to_string());
        }
    };
    url.query_pairs_mut()
        .append_pair("libraryName", &args.library_name)
        .append_pair("query", &args.query);

    let req = HttpRequest::new(url.as_str())
        .with_method("GET")
        .insert_context7_headers(args.context7_api_key.as_deref());

    match http::request::<()>(&req, None) {
        Ok(res) => {
            let body_str = String::from_utf8_lossy(&res.body()).to_string();
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
                        if let Ok(Value::Object(map)) = serde_json::to_value(context7_response) {
                            call_tool_result.structured_content = Some(map);
                        }

                        cache::put("resolve_library_id", &args, &call_tool_result);
                        call_tool_result
                    }
                    Err(e) => CallToolResult::error(e.to_string()),
                }
            } else {
                CallToolResult::error(format!(
                    "API request failed with status {}: {}",
                    res.status_code(),
                    body_str,
                ))
            }
        }
        Err(e) => CallToolResult::error(e.to_string()),
    }
}

// Stub functions for MCP handlers not implemented in this tools-only plugin
pub(crate) fn complete(_input: CompleteRequest) -> Result<CompleteResult> {
    Ok(CompleteResult::default())
}

pub(crate) fn get_prompt(_input: GetPromptRequest) -> Result<GetPromptResult> {
    Err(anyhow::anyhow!("Prompts are not supported by this plugin"))
}

pub(crate) fn list_prompts(_input: ListPromptsRequest) -> Result<ListPromptsResult> {
    Ok(ListPromptsResult::default())
}

pub(crate) fn list_resource_templates(
    _input: ListResourceTemplatesRequest,
) -> Result<ListResourceTemplatesResult> {
    Ok(ListResourceTemplatesResult::default())
}

pub(crate) fn list_resources(_input: ListResourcesRequest) -> Result<ListResourcesResult> {
    Ok(ListResourcesResult::default())
}

pub(crate) fn on_roots_list_changed(_input: Value) -> Result<()> {
    Ok(())
}

pub(crate) fn read_resource(_input: ReadResourceRequest) -> Result<ReadResourceResult> {
    Err(anyhow::anyhow!(
        "Resources are not supported by this plugin"
    ))
}
