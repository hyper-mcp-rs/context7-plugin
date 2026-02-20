use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

// --- resolve_library_id request/response types ---

#[derive(Default, Debug, Clone, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ResolveLibraryIdArguments {
    #[schemars(
        description = "Library name to search for and retrieve a Context7-compatible library ID."
    )]
    #[serde(rename = "libraryName")]
    pub library_name: String,

    #[schemars(
        description = "The question or task you need help with. This is used to rank library results \
        by relevance to what the user is trying to accomplish. The query is sent to the Context7 API for processing. \
        Do not include any sensitive or confidential information such as API keys, passwords, credentials, personal data, \
        or proprietary code in your query."
    )]
    pub query: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DocumentState {
    Delete,
    Error,
    Finalized,
    #[default]
    Initial,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct Library {
    pub id: String,
    pub title: String,
    pub description: String,
    pub branch: String,
    #[serde(rename = "lastUpdateDate")]
    pub last_update_date: String,
    pub state: DocumentState,
    #[serde(rename = "totalTokens")]
    pub total_tokens: f64,
    #[serde(rename = "totalSnippets")]
    pub total_snippets: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stars: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "trustScore")]
    pub trust_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "benchmarkScore")]
    pub benchmark_score: Option<f64>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ResolveLibraryIdResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub results: Vec<Library>,
}

// --- query_docs request/response types ---

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct CodeListEntry {
    pub language: String,
    pub code: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct CodeSnippet {
    #[serde(rename = "codeTitle")]
    pub code_title: String,
    #[serde(rename = "codeDescription")]
    pub code_description: String,
    #[serde(rename = "codeLanguage")]
    pub code_language: String,
    #[serde(rename = "codeTokens")]
    pub code_tokens: f64,
    #[serde(rename = "codeId")]
    pub code_id: String,
    #[serde(rename = "pageTitle")]
    pub page_title: String,
    #[serde(rename = "codeList")]
    pub code_list: Vec<CodeListEntry>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct InfoSnippet {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "pageId")]
    pub page_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breadcrumb: Option<String>,
    pub content: String,
    #[serde(rename = "contentTokens")]
    pub content_tokens: f64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct Rules {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub global: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "libraryOwn")]
    pub library_own: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "libraryTeam")]
    pub library_team: Vec<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct QueryDocsResponse {
    #[serde(rename = "codeSnippets")]
    pub code_snippets: Vec<CodeSnippet>,
    #[serde(rename = "infoSnippets")]
    pub info_snippets: Vec<InfoSnippet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ClearCacheArguments {}

#[derive(Default, Debug, Clone, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) struct QueryDocsArguments {
    #[schemars(
        description = "Exact Context7-compatible library ID (e.g., '/mongodb/docs', '/vercel/next.js', '/supabase/supabase', \
        '/vercel/next.js/v14.3.0-canary.87') retrieved from 'resolve_library_id' or directly from user query in the format '/org/project' \
        or '/org/project/version'."
    )]
    #[serde(rename = "libraryId")]
    pub library_id: String,

    #[schemars(
        description = "The question or task you need help with. Be specific and include relevant details. \
        Good: 'How to set up authentication with JWT in Express.js' or 'React useEffect cleanup function examples'. \
        Bad: 'auth' or 'hooks'. The query is sent to the Context7 API for processing. Do not include any sensitive or \
        confidential information such as API keys, passwords, credentials, personal data, or proprietary code in your query."
    )]
    pub query: String,
}
