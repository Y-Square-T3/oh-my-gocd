// Users section of the GoCD API docs: thin GocdCall constructors for
// /go/api/users and its bulk endpoints, API v3. GoCD neither documents nor
// enforces If-Match on these writes (see UsersControllerV3), so no constructor
// takes an etag.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/users";

pub fn list() -> GocdCall {
    GocdCall::get(PATH).version(3)
}

pub fn read(login_name: &str) -> GocdCall {
    GocdCall::get(&for_user(login_name)).version(3)
}

pub fn create(body: Value) -> GocdCall {
    GocdCall::post(PATH).version(3).body(body)
}

pub fn update(login_name: &str, body: Value) -> GocdCall {
    GocdCall::patch(&for_user(login_name)).version(3).body(body)
}

pub fn remove(login_name: &str) -> GocdCall {
    GocdCall::delete(&for_user(login_name)).version(3)
}

pub fn bulk_delete(body: Value) -> GocdCall {
    GocdCall::delete(PATH).version(3).body(body)
}

pub fn bulk_update_state(body: Value) -> GocdCall {
    GocdCall::patch(format!("{PATH}/operations/state").as_str())
        .version(3)
        .body(body)
}

fn for_user(login_name: &str) -> String {
    format!("{PATH}/{login_name}")
}
