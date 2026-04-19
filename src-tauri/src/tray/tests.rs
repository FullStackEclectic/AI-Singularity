use super::menu;

#[test]
fn parse_quick_switch_id_works() {
    let id = menu::quick_switch_menu_id("Codex", "acc-1");
    let parsed = menu::parse_quick_switch_menu_id(&id);
    assert_eq!(parsed, Some(("codex".to_string(), "acc-1".to_string())));
}

#[test]
fn parse_quick_switch_id_rejects_invalid_payload() {
    assert_eq!(
        menu::parse_quick_switch_menu_id("quick_switch_account_only_platform"),
        None
    );
    assert_eq!(menu::parse_quick_switch_menu_id("something_else"), None);
}

#[test]
fn account_summary_label_is_stable() {
    assert_eq!(
        menu::format_accounts_summary_label(12, 4, 3),
        "账号概览：总 12 · 当前 4 · 需关注 3"
    );
}
