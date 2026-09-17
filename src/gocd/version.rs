// Version section of the GoCD API docs: thin GocdCall constructors for
// /go/api/version, API v1. Read-only — GoCD documents no query params, body
// or If-Match for it.

use super::GocdCall;

const PATH: &str = "api/version";

pub fn read() -> GocdCall {
    GocdCall::get(PATH).version(1)
}
