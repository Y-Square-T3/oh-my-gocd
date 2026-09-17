// Server Health section of the GoCD API docs: a thin GocdCall constructor for
// /go/api/v1/health. The API version lives in the path — the endpoint serves
// plain JSON with no documented accept version — and GoCD defines no query
// params, body or If-Match for it.

use super::GocdCall;

const PATH: &str = "api/v1/health";

pub fn check() -> GocdCall {
    GocdCall::get(PATH).version(1)
}
