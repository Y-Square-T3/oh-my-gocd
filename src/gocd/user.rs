use serde_json::{Value, json};

/// Prune a GoCD user object down to the identity fields omg reports:
/// login_name, display_name, enabled, email, checkin_aliases.
pub fn prune_identity(user: &Value) -> Value {
    json!({
        "login_name": user["login_name"],
        "display_name": user["display_name"],
        "enabled": user["enabled"],
        "email": user["email"],
        "checkin_aliases": user["checkin_aliases"],
    })
}

/// Shape taken verbatim from the GoCD API docs' current-user example.
#[cfg(test)]
pub fn gocd_doc_example() -> Value {
    json!({
        "_links": {
            "doc": { "href": "https://api.gocd.org/#users" },
            "self": { "href": "https://ci.example.com/go/api/users/jdoe" },
            "find": { "href": "https://ci.example.com/go/api/users/:login_name" }
        },
        "login_name": "jdoe",
        "display_name": "John Doe",
        "enabled": true,
        "email": null,
        "email_me": false,
        "checkin_aliases": ["jdoe"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_identity_fields_and_drops_the_rest() {
        let pruned = prune_identity(&gocd_doc_example());
        assert_eq!(
            pruned,
            json!({
                "login_name": "jdoe",
                "display_name": "John Doe",
                "enabled": true,
                "email": null,
                "checkin_aliases": ["jdoe"]
            })
        );
    }

    #[test]
    fn missing_fields_become_null() {
        let pruned = prune_identity(&json!({ "login_name": "jdoe" }));
        assert_eq!(pruned["login_name"], json!("jdoe"));
        assert_eq!(pruned["display_name"], Value::Null);
    }
}
