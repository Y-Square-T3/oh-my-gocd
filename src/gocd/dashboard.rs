// Dashboard section of the GoCD API docs: thin GocdCall constructors for
// /go/api/dashboard, API v4. Read-only — GoCD documents no query params, body
// or If-Match for it.

use super::GocdCall;

const PATH: &str = "api/dashboard";

pub fn read() -> GocdCall {
    GocdCall::get(PATH).version(4)
}
