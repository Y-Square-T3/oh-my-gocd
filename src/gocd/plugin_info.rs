// Plugin Info section of the GoCD API docs: thin GocdCall constructors for
// /go/api/admin/plugin_info, API v7. The section is read-only — GoCD documents
// no writes, so no body or If-Match constructors exist.

use super::GocdCall;

const PATH: &str = "api/admin/plugin_info";

pub fn list() -> GocdCall {
    GocdCall::get(PATH).version(7)
}

pub fn read(plugin_id: &str) -> GocdCall {
    GocdCall::get(&format!("{PATH}/{plugin_id}")).version(7)
}
