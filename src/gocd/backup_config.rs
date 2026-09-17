// Backup Config section of the GoCD API docs: thin GocdCall constructors for
// /go/api/config/backup, API v1. The docs send no ETag and require no
// If-Match on this resource, so no constructor takes one.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/config/backup";

pub fn read() -> GocdCall {
    GocdCall::get(PATH).version(1)
}

pub fn update(body: Value) -> GocdCall {
    GocdCall::put(PATH).version(1).body(body)
}

pub fn remove() -> GocdCall {
    GocdCall::delete(PATH).version(1)
}
