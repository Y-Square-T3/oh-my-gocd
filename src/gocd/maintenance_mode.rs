// Maintenance Mode section of the GoCD API docs: /go/api/admin/maintenance_mode,
// API v1 throughout. Info is a GET; the docs' "HTTP Request" lines list
// enable and disable as GET-as-action, but their curl examples — and the
// routes the GoCD server actually registers — send them as body-less POSTs
// carrying the documented X-GoCD-Confirm header.

use super::GocdCall;

const BASE: &str = "api/admin/maintenance_mode";

pub fn info() -> GocdCall {
    GocdCall::get(&format!("{BASE}/info")).version(1)
}

pub fn enable() -> GocdCall {
    GocdCall::post(&format!("{BASE}/enable"))
        .version(1)
        .confirm()
}

pub fn disable() -> GocdCall {
    GocdCall::post(&format!("{BASE}/disable"))
        .version(1)
        .confirm()
}
