// Agents section of the GoCD API docs: thin GocdCall constructors for
// /go/api/agents and its sub-resources. Agents are v7; job run history is v1.
// GoCD neither surfaces an ETag for agents nor reads filter query params on the
// collection, so no constructor takes an etag or a filter.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/agents";

pub fn list() -> GocdCall {
    GocdCall::get(PATH).version(7)
}

pub fn read(uuid: &str) -> GocdCall {
    GocdCall::get(&for_agent(uuid)).version(7)
}

pub fn update(uuid: &str, body: Value) -> GocdCall {
    GocdCall::patch(&for_agent(uuid)).version(7).body(body)
}

pub fn remove(uuid: &str) -> GocdCall {
    GocdCall::delete(&for_agent(uuid)).version(7)
}

pub fn bulk_update(body: Value) -> GocdCall {
    GocdCall::patch(PATH).version(7).body(body)
}

pub fn bulk_delete(body: Value) -> GocdCall {
    GocdCall::delete(PATH).version(7).body(body)
}

pub fn job_run_history(uuid: &str, query: Vec<(String, String)>) -> GocdCall {
    GocdCall::get(&format!("{}/job_run_history", for_agent(uuid)))
        .version(1)
        .query(query)
}

pub fn kill_running_tasks(uuid: &str) -> GocdCall {
    GocdCall::post(&format!("{}/kill_running_tasks", for_agent(uuid))).version(7)
}

fn for_agent(uuid: &str) -> String {
    format!("{PATH}/{uuid}")
}
