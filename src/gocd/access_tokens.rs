// Access Tokens section of the GoCD API docs: thin GocdCall constructors for
// /go/api/current_user/access_tokens and /go/api/admin/access_tokens, API v1.
// Creation is deliberately not exposed (spec #1). GoCD documents no If-Match
// on the revoke POSTs, so no constructor takes an etag.

use super::GocdCall;
use serde_json::Value;

const CURRENT_USER_PATH: &str = "api/current_user/access_tokens";
const ADMIN_PATH: &str = "api/admin/access_tokens";

pub fn current_user_list() -> GocdCall {
    GocdCall::get(CURRENT_USER_PATH).version(1)
}

pub fn current_user_read(token_id: &str) -> GocdCall {
    GocdCall::get(&format!("{CURRENT_USER_PATH}/{token_id}")).version(1)
}

pub fn current_user_revoke(token_id: &str, body: Value) -> GocdCall {
    GocdCall::post(format!("{CURRENT_USER_PATH}/{token_id}/revoke").as_str())
        .version(1)
        .body(body)
}

pub fn admin_list(query: Vec<(String, String)>) -> GocdCall {
    GocdCall::get(ADMIN_PATH).version(1).query(query)
}

pub fn admin_read(token_id: &str) -> GocdCall {
    GocdCall::get(&format!("{ADMIN_PATH}/{token_id}")).version(1)
}

pub fn admin_revoke(token_id: &str, body: Value) -> GocdCall {
    GocdCall::post(format!("{ADMIN_PATH}/{token_id}/revoke").as_str())
        .version(1)
        .body(body)
}
