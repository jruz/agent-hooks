use serde::Serialize;

#[derive(Serialize)]
struct BlockResponse {
    decision: &'static str,
    reason: String,
}

pub fn block(reason: String) -> Result<String, serde_json::Error> {
    let response = BlockResponse {
        decision: "block",
        reason,
    };
    serde_json::to_string(&response)
}
