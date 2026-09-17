// Encryption section of the GoCD API docs: thin GocdCall constructors for
// /go/api/admin/encrypt, API v1. GoCD rate-limits the endpoint to 30 requests
// per minute and documents no If-Match, so no constructor takes an etag.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/admin/encrypt";

pub fn encrypt(body: Value) -> GocdCall {
    GocdCall::post(PATH).version(1).body(body)
}
