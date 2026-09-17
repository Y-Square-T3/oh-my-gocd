// Notify Materials section of the GoCD API docs: thin GocdCall constructors
// for /go/api/admin/materials/{svn,git,hg,scm}/notify, API v2. All four
// operations are body-carrying POSTs; GoCD documents no If-Match guard and
// no query params here, so no etag or query constructors exist.

use super::GocdCall;
use serde_json::Value;

const BASE: &str = "api/admin/materials";

pub fn notify_svn(body: Value) -> GocdCall {
    GocdCall::post(&format!("{BASE}/svn/notify"))
        .version(2)
        .body(body)
}

pub fn notify_git(body: Value) -> GocdCall {
    GocdCall::post(&format!("{BASE}/git/notify"))
        .version(2)
        .body(body)
}

pub fn notify_hg(body: Value) -> GocdCall {
    GocdCall::post(&format!("{BASE}/hg/notify"))
        .version(2)
        .body(body)
}

pub fn notify_scm(body: Value) -> GocdCall {
    GocdCall::post(&format!("{BASE}/scm/notify"))
        .version(2)
        .body(body)
}
