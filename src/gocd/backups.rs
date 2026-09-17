// Backups section of the GoCD API docs: thin GocdCall constructors for
// /go/api/backups, API v2.

use super::GocdCall;

const PATH: &str = "api/backups";

pub fn schedule() -> GocdCall {
    GocdCall::post(PATH).version(2)
}

pub fn status(backup_id: &str) -> GocdCall {
    GocdCall::get(&format!("{PATH}/{backup_id}")).version(2)
}
