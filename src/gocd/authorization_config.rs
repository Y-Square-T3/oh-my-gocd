// Authorization Configuration section of the GoCD API docs: thin GocdCall
// constructors for /go/api/admin/security/auth_configs, API v2.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/admin/security/auth_configs";

pub fn list() -> GocdCall {
    GocdCall::get(PATH).version(2)
}

pub fn read(auth_config_id: &str) -> GocdCall {
    GocdCall::get(&for_config(auth_config_id)).version(2)
}

pub fn create(body: Value) -> GocdCall {
    GocdCall::post(PATH).version(2).body(body)
}

pub fn update(auth_config_id: &str, etag: &str, body: Value) -> GocdCall {
    GocdCall::put(&for_config(auth_config_id))
        .version(2)
        .body(body)
        .etag(etag.to_owned())
}

pub fn remove(auth_config_id: &str) -> GocdCall {
    GocdCall::delete(&for_config(auth_config_id)).version(2)
}

fn for_config(auth_config_id: &str) -> String {
    format!("{PATH}/{auth_config_id}")
}
