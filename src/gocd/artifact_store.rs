// Artifact Store section of the GoCD API docs: thin GocdCall constructors for
// /go/api/admin/artifact_stores, API v1.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/admin/artifact_stores";

pub fn list() -> GocdCall {
    GocdCall::get(PATH).version(1)
}

pub fn read(store_id: &str) -> GocdCall {
    GocdCall::get(&for_store(store_id)).version(1)
}

pub fn create(body: Value) -> GocdCall {
    GocdCall::post(PATH).version(1).body(body)
}

pub fn update(store_id: &str, etag: &str, body: Value) -> GocdCall {
    GocdCall::put(&for_store(store_id))
        .version(1)
        .body(body)
        .etag(etag.to_owned())
}

pub fn remove(store_id: &str) -> GocdCall {
    GocdCall::delete(&for_store(store_id)).version(1)
}

fn for_store(store_id: &str) -> String {
    format!("{PATH}/{store_id}")
}
