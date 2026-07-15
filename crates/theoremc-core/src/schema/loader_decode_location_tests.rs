//! Unit tests for decode-failure YAML location lookup.

use super::*;

#[test]
fn yaml_keys_and_terminal_arguments_preserve_identifier_text() {
    let key = YamlKey::new("outer: target");

    assert_eq!(key.as_str(), "outer: target");
    assert_eq!(terminal_argument_name(key).as_str(), "target");
    assert_eq!(
        terminal_argument_name(YamlKey::new("target")).as_str(),
        "target"
    );
}

#[test]
fn argument_and_mapping_lines_require_complete_yaml_keys() {
    let argument = YamlKey::new("target");

    assert_eq!(
        locate_argument_line(3, "    target: value", argument),
        Some((4, 5))
    );
    assert_eq!(
        locate_argument_line(3, "    target_value: value", argument),
        None
    );
    assert_eq!(locate_argument_line(3, "    target", argument), None);
    assert!(mapping_key_line("  target:", argument));
    assert!(!mapping_key_line("  target_value:", argument));
}

#[test]
fn line_classifiers_distinguish_sections_headers_and_scope_boundaries() {
    assert!(section_line("  Let:", YamlKey::new("Let")));
    assert!(!section_line("Let: nested", YamlKey::new("Let")));
    assert!(is_top_level_section("Evidence:"));
    assert!(!is_top_level_section("  Evidence:"));
    assert!(is_theorem_header("theorem:"));
    assert!(is_list_item("  - call:"));
    assert!(exits_scope("  next:", 2));
    assert!(!exits_scope("    nested:", 2));
    assert_eq!(indent_width("    nested:"), 4);
}

#[test]
fn document_lines_stop_before_theorem_header_marker() {
    let input = concat!("Theorem: First\n", "Let:\n", "Theorem:\n", "Let:\n");

    let lines = document_lines(input, 0).collect::<Vec<_>>();

    assert_eq!(lines, vec![(0, "Theorem: First"), (1, "Let:")]);
}

#[test]
fn let_binding_scan_finds_arguments_and_stops_at_the_next_binding() {
    let input = concat!(
        "Theorem: Example\n",
        "Let:\n",
        "  first:\n",
        "    call:\n",
        "      args:\n",
        "        target:\n",
        "          ref: source\n",
        "  second:\n",
        "    call:\n",
    );

    assert_eq!(
        locate_let_binding_argument(input, 0, YamlKey::new("first"), YamlKey::new("target")),
        Some((6, 9))
    );
    assert_eq!(
        locate_let_binding_argument(input, 0, YamlKey::new("first"), YamlKey::new("missing")),
        None
    );
}

#[test]
fn binding_line_scan_reports_all_outcomes() {
    let argument = YamlKey::new("target");

    assert_eq!(
        scan_binding_line(4, "    target: source", 2, argument),
        BindingLineOutcome::Found(5, 5)
    );
    assert_eq!(
        scan_binding_line(4, "  next:", 2, argument),
        BindingLineOutcome::ExitScope
    );
    assert_eq!(
        scan_binding_line(4, "    other: source", 2, argument),
        BindingLineOutcome::Continue
    );
}

#[test]
fn do_step_scan_finds_the_selected_argument_and_stops_after_it() {
    let input = concat!(
        "Theorem: Example\n",
        "Do:\n",
        "  - call:\n",
        "      action: account.read\n",
        "      args:\n",
        "        target:\n",
        "          ref: source\n",
        "  - call:\n",
        "      action: account.write\n",
    );

    assert_eq!(
        locate_do_step_argument(input, 0, 1, YamlKey::new("target")),
        Some((6, 9))
    );
    assert_eq!(
        locate_do_step_argument(input, 0, 1, YamlKey::new("missing")),
        None
    );
}

#[test]
fn do_step_line_scan_reports_all_outcomes() {
    let selected_context = DoStepLineContext {
        scope: StepScope::SelectedStart,
        indent: 2,
        argument_name: YamlKey::new("target"),
    };
    let inside_context = DoStepLineContext {
        scope: StepScope::InsideSelected,
        ..selected_context
    };

    assert_eq!(
        scan_do_step_line(4, "    target: source", selected_context),
        DoStepLineOutcome::Found(5, 5)
    );
    assert_eq!(
        scan_do_step_line(4, "  next:", inside_context),
        DoStepLineOutcome::ExitScope
    );
    assert_eq!(
        scan_do_step_line(4, "    other: source", inside_context),
        DoStepLineOutcome::Continue
    );
}

#[test]
fn step_scope_tracks_selected_and_subsequent_list_items() {
    let mut current_step = 0;
    let mut selected_indent = None;

    assert_eq!(
        update_step_scope(
            "  detail: value",
            2,
            &mut current_step,
            &mut selected_indent
        ),
        StepScope::OutsideSelected
    );
    assert_eq!(
        update_step_scope("  - call:", 2, &mut current_step, &mut selected_indent),
        StepScope::OutsideSelected
    );
    assert_eq!(
        update_step_scope("  - call:", 2, &mut current_step, &mut selected_indent),
        StepScope::SelectedStart
    );
    assert_eq!(
        update_step_scope("    args:", 2, &mut current_step, &mut selected_indent),
        StepScope::InsideSelected
    );
    assert_eq!(
        update_step_scope("  - call:", 2, &mut current_step, &mut selected_indent),
        StepScope::PastSelected
    );
}
