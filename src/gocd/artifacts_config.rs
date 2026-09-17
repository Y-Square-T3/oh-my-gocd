// Artifacts Config section of the GoCD API docs: thin GocdCall constructors for
// /go/api/admin/config/server/artifact_config, API v1.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/admin/config/server/artifact_config";

pub fn read() -> GocdCall {
    GocdCall::get(PATH).version(1)
}

pub fn update(etag: &str, body: Value) -> GocdCall {
    GocdCall::put(PATH)
        .version(1)
        .body(body)
        .etag(etag.to_owned())
}
