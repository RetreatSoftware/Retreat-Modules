use retreat_manifest::source_scan::{declares_plugin, registered_actions, registered_commands};

#[test]
fn finds_commands_across_lines_and_ignores_comments_and_definitions() {
    let source = r#"
        // app.register_command("commented.out", "x", f);
        fn register_command(a: u8) {}
        app.register_command("a.one", "a.one", f)
            .register_command(
                "a.two",
                "a.two <x>",
                g,
            );
    "#;
    assert_eq!(registered_commands(source), ["a.one", "a.two"]);
}

#[test]
fn finds_actions_from_struct_literals_and_helpers() {
    let source = r#"
        app.register_action(InputAction { id: "jump", command_line: "a.go", default_keys: &[] })
           .register_action(held_action("left", InputKey::ArrowLeft));
    "#;
    assert_eq!(registered_actions(source), ["jump", "left"]);
}

#[test]
fn plugin_needs_declaration_and_impl() {
    let ok = "pub struct P;\nimpl Plugin for P { fn build(&self, a: &mut App) {} }";
    assert!(declares_plugin(ok, "P"));
    assert!(!declares_plugin("pub struct P;", "P"));
    assert!(!declares_plugin("impl Plugin for P {}", "P"));
    assert!(!declares_plugin(ok, "Q"));
}
