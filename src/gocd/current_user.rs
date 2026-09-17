// Current User section of the GoCD API docs: thin GocdCall constructors for
// /go/api/current_user, API v1. GoCD requires no If-Match on the PATCH, so no
// constructor takes an etag.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/current_user";

pub fn update(body: Value) -> GocdCall {
    GocdCall::patch(PATH).version(1).body(body)
}
