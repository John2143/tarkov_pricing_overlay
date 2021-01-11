#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    #[serde(rename = "ParsedResults")]
    pub parsed_results: Vec<ParsedResult>,
    #[serde(rename = "OCRExitCode")]
    pub ocrexit_code: i64,
    #[serde(rename = "IsErroredOnProcessing")]
    pub is_errored_on_processing: bool,
    #[serde(rename = "ProcessingTimeInMilliseconds")]
    pub processing_time_in_milliseconds: String,
    #[serde(rename = "SearchablePDFURL")]
    pub searchable_pdfurl: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedResult {
    #[serde(rename = "TextOverlay")]
    pub text_overlay: TextOverlay,
    #[serde(rename = "TextOrientation")]
    pub text_orientation: String,
    #[serde(rename = "FileParseExitCode")]
    pub file_parse_exit_code: i64,
    #[serde(rename = "ParsedText")]
    pub parsed_text: String,
    #[serde(rename = "ErrorMessage")]
    pub error_message: String,
    #[serde(rename = "ErrorDetails")]
    pub error_details: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextOverlay {
    #[serde(rename = "Lines")]
    pub lines: Vec<::serde_json::Value>,
    #[serde(rename = "HasOverlay")]
    pub has_overlay: bool,
    #[serde(rename = "Message")]
    pub message: String,
}

