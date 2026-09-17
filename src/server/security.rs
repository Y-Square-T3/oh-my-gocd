// The tool tier table: every MCP tool omg serves is classified here as view,
// operate or danger, and the configured Mode decides which tiers stay visible
// to the MCP client. The ladder is cumulative: each mode exposes its own tier
// and everything below it. An unclassified tool is treated as danger whenever
// the mode can hide anything, so a forgotten entry fails closed; the
// exhaustiveness tests make it impossible to ship one silently.

use crate::config::Mode;

/// What a tool can do to the GoCD server, independent of the current mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Pure reads: GETs only, including the sensitive admin ones.
    View,
    /// Writes that run or configure pipelines and routine objects.
    Operate,
    /// Deletes, security writes and server-wide availability toggles.
    Danger,
}

// The accepted tier table: 32 view, 22 operate, 17 danger — 71 tools.
const VIEW_TOOLS: &[&str] = &[
    "check_server_health",
    "compare_pipeline_instances",
    "get_admin_access_token",
    "get_admin_access_tokens",
    "get_agent",
    "get_agent_job_run_history",
    "get_agents",
    "get_all_plugin_info",
    "get_artifact_store",
    "get_artifact_stores",
    "get_artifacts_config",
    "get_auth_config",
    "get_auth_configs",
    "get_backup",
    "get_backup_config",
    "get_current_user",
    "get_current_user_access_token",
    "get_current_user_access_tokens",
    "get_dashboard",
    "get_job_history",
    "get_job_instance",
    "get_maintenance_mode_info",
    "get_pipeline_history",
    "get_pipeline_instance",
    "get_pipeline_status",
    "get_plugin_info",
    "get_server_health_messages",
    "get_stage_history",
    "get_stage_instance",
    "get_user",
    "get_users",
    "get_version",
];

const OPERATE_TOOLS: &[&str] = &[
    "bulk_update_agents",
    "cancel_stage_instance",
    "comment_pipeline_instance",
    "create_artifact_store",
    "encrypt_value",
    "notify_git_material",
    "notify_hg_material",
    "notify_scm_material",
    "notify_svn_material",
    "pause_pipeline",
    "run_failed_stage_jobs",
    "run_selected_stage_jobs",
    "run_stage",
    "schedule_backup",
    "schedule_pipeline",
    "unlock_pipeline",
    "unpause_pipeline",
    "update_agent",
    "update_artifact_store",
    "update_artifacts_config",
    "update_backup_config",
    "update_current_user",
];

const DANGER_TOOLS: &[&str] = &[
    "bulk_delete_agents",
    "bulk_delete_users",
    "bulk_enable_disable_users",
    "create_auth_config",
    "create_user",
    "delete_agent",
    "delete_artifact_store",
    "delete_auth_config",
    "delete_backup_config",
    "delete_user",
    "disable_maintenance_mode",
    "enable_maintenance_mode",
    "kill_agent_running_tasks",
    "revoke_admin_access_token",
    "revoke_current_user_access_token",
    "update_auth_config",
    "update_user",
];

/// The tier of a known tool, or `None` for a name omg does not classify.
pub fn tier_for(name: &str) -> Option<Tier> {
    if VIEW_TOOLS.contains(&name) {
        Some(Tier::View)
    } else if OPERATE_TOOLS.contains(&name) {
        Some(Tier::Operate)
    } else if DANGER_TOOLS.contains(&name) {
        Some(Tier::Danger)
    } else {
        None
    }
}

impl Tier {
    /// Lowercase tier name, as the user-facing messages spell it.
    fn as_str(self) -> &'static str {
        match self {
            Tier::View => "view",
            Tier::Operate => "operate",
            Tier::Danger => "danger",
        }
    }

    /// The cumulative ladder: every mode exposes its tier and all below it.
    /// An unclassified tool is treated as danger whenever anything can hide.
    pub fn allowed_in(self, mode: Mode) -> bool {
        matches!(
            (self, mode),
            (Tier::View, _)
                | (Tier::Operate, Mode::Operate | Mode::Full)
                | (Tier::Danger, Mode::Full)
        )
    }

    fn lowest_mode_allowing(self) -> Mode {
        match self {
            Tier::View => Mode::View,
            Tier::Operate => Mode::Operate,
            Tier::Danger => Mode::Full,
        }
    }
}

/// Whether `name` may be exposed to the client under `mode`.
pub fn tool_visible(name: &str, mode: Mode) -> bool {
    match tier_for(name) {
        Some(tier) => tier.allowed_in(mode),
        None => mode == Mode::Full,
    }
}

/// When `mode` forbids `name`, the text of the tool error explaining why: the
/// tool, the current mode, the tier it needs, and the way out. `None` for
/// allowed tools and for names omg does not know at all — those still hit the
/// router's own not-found answer.
pub fn blocked_message(name: &str, mode: Mode) -> Option<String> {
    let tier = match tier_for(name) {
        Some(tier) => tier,
        None if mode == Mode::Full => return None,
        None => Tier::Danger,
    };
    if tier.allowed_in(mode) {
        return None;
    }
    Some(format!(
        "`{name}` is not available in omg's `{mode}` mode: it is a `{}` tool. \
         To allow it, run `omg config --mode {}` and restart the MCP session.",
        tier.as_str(),
        tier.lowest_mode_allowing()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::mcp::fake::{FakeGocd, service_in};
    use serde_json::json;

    #[test]
    fn reads_are_classified_view_and_writes_split_by_the_accepted_table() {
        assert_eq!(tier_for("get_dashboard"), Some(Tier::View));
        assert_eq!(tier_for("get_auth_configs"), Some(Tier::View));
        assert_eq!(tier_for("schedule_pipeline"), Some(Tier::Operate));
        assert_eq!(tier_for("update_artifacts_config"), Some(Tier::Operate));
        assert_eq!(tier_for("enable_maintenance_mode"), Some(Tier::Danger));
        assert_eq!(tier_for("delete_user"), Some(Tier::Danger));
    }

    #[test]
    fn an_unknown_tool_name_has_no_tier() {
        assert_eq!(tier_for("delete_the_world"), None);
    }

    #[test]
    fn the_view_tier_is_allowed_in_every_mode() {
        for mode in [Mode::View, Mode::Operate, Mode::Full] {
            assert!(Tier::View.allowed_in(mode), "view blocked in {mode}");
        }
    }

    #[test]
    fn the_operate_tier_needs_the_operate_mode_or_above() {
        assert!(!Tier::Operate.allowed_in(Mode::View));
        assert!(Tier::Operate.allowed_in(Mode::Operate));
        assert!(Tier::Operate.allowed_in(Mode::Full));
    }

    #[test]
    fn the_danger_tier_needs_the_full_mode() {
        assert!(!Tier::Danger.allowed_in(Mode::View));
        assert!(!Tier::Danger.allowed_in(Mode::Operate));
        assert!(Tier::Danger.allowed_in(Mode::Full));
    }

    #[test]
    fn tool_visibility_follows_the_ladder() {
        assert!(tool_visible("get_dashboard", Mode::View));
        assert!(!tool_visible("schedule_pipeline", Mode::View));
        assert!(tool_visible("schedule_pipeline", Mode::Operate));
        assert!(!tool_visible("delete_user", Mode::Operate));
        assert!(tool_visible("delete_user", Mode::Full));
    }

    #[test]
    fn an_unclassified_tool_is_danger_outside_full_mode() {
        assert!(!tool_visible("mystery_tool", Mode::View));
        assert!(!tool_visible("mystery_tool", Mode::Operate));
        assert!(tool_visible("mystery_tool", Mode::Full));
    }

    #[test]
    fn allowed_tools_have_no_blocked_message() {
        assert_eq!(blocked_message("get_dashboard", Mode::View), None);
        assert_eq!(blocked_message("schedule_pipeline", Mode::Full), None);
        assert_eq!(blocked_message("delete_the_world", Mode::Full), None);
    }

    #[test]
    fn every_registered_tool_is_classified_in_the_tier_table() {
        let service = service_in(FakeGocd::replies(json!({}), None), Mode::Full);
        for name in service.tool_names() {
            assert!(tier_for(&name).is_some(), "tool {name} has no tier");
        }
    }

    #[test]
    fn every_tier_table_entry_is_a_registered_tool() {
        let service = service_in(FakeGocd::replies(json!({}), None), Mode::Full);
        let registered = service.tool_names();
        for name in VIEW_TOOLS.iter().chain(OPERATE_TOOLS).chain(DANGER_TOOLS) {
            assert!(
                registered.contains(&name.to_string()),
                "unknown tool in table: {name}"
            );
        }
    }

    #[test]
    fn the_blocked_message_names_the_tool_the_mode_the_way_out_and_a_restart() {
        let msg = blocked_message("schedule_pipeline", Mode::View).unwrap();
        assert!(msg.contains("schedule_pipeline"), "got: {msg}");
        assert!(msg.contains("`view` mode"), "got: {msg}");
        assert!(msg.contains("omg config --mode operate"), "got: {msg}");
        assert!(msg.contains("restart"), "got: {msg}");

        let msg = blocked_message("delete_user", Mode::Operate).unwrap();
        assert!(msg.contains("omg config --mode full"), "got: {msg}");
    }
}
