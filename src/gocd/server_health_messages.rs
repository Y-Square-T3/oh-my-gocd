// Server Health Messages section of the GoCD API docs: a thin GocdCall
// constructor for /go/api/server_health_messages, API v1. Read-only — GoCD
// documents no query params, body or If-Match for it, and sends no ETag for
// the collection.

use super::GocdCall;

const PATH: &str = "api/server_health_messages";

pub fn list() -> GocdCall {
    GocdCall::get(PATH).version(1)
}
