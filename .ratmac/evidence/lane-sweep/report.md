# Lane sweep report

- swept: 2026-08-25T10:31:02Z
- lanes-root: test-hidden
- verify-expired: no
- verdicts: 28 pass, 2 expired, 20 red, 0 missing - 50 crates

| crate | verdict | detail |
| :--- | :--- | :--- |
| t-058 | red | ht_058_02_input_routing, ht_058_01_regression, ht_058_03_lifecycle, ht_058_06_cross_feature, ht_058_05_output_filesystem, ht_058_04_durability |
| t-059 | red | ht_059_02_input_routing, ht_059_04_durability, ht_059_01_regression, ht_059_03_lifecycle, ht_059_05_output_filesystem, ht_059_06_cross_feature |
| t-060 | red | ht_060_03_lifecycle, ht_060_01_regression, ht_060_02_input_routing, ht_060_06_cross_feature, ht_060_05_output_filesystem, ht_060_04_durability |
| t-061 | red | build refused: error[E0609]: no field `phase` on type `RunState` |
| t-062 | red | build refused: error[E0609]: no field `phase` on type `RunState` |
| t-063 | red | build refused: error[E0560]: struct `HoldRequest` has no field named `ticket` |
| t-064 | red | build refused: error[E0425]: cannot find function `render_json` in module `doctor` |
| t-065 | red | phrase_near_misses_refuse_before_any_write, abandonment_shape_survives_the_phrase_change, spawn_is_legal_only_in_the_spawning_phase_of_a_live_run, spawn_stays_orthogonal_to_siblings_goal_and_guards, interrupted_respawn_converges_on_retry, spawn_writes_only_the_child_directory |
| t-066 | red | join_counts_only_live_passed_children, only_ledger_and_ordinary_run_files_exist, disjoint_parent_ledgers_survive_retirement, failed_spawn_leaves_no_child_and_no_entry, mangled_ledger_refuses_strictly_at_read, uncomposed_runs_never_gain_ledger_content |
| t-067 | red | build refused: error[E0425]: cannot find function `render_json` in module `doctor` |
| t-068 | red | membership_addresses_the_right_parent, obstructed_tree_gets_the_identical_refusal_with_no_write, refused_child_spawn_leaves_digests_equal, successor_and_abandoned_child_refuse_naming_the_cap, cap_join_and_guard_refusals_keep_their_own_names, top_level_spawn_behavior_is_unchanged |
| t-069 | red | archive_precedes_advance_for_child_bytes, child_authored_verdicts_get_no_leniency, reviewer_flow_composes_with_cap_join_and_guards, human_and_child_verdicts_archive_identically, no_route_before_terminal_fact_and_never_from_abandoned, evidence_lands_only_in_the_parents_archive |
| t-070 | red | build refused: error[E0425]: cannot find function `render_json` in module `ratmac::doctor` |
| t-071 | pass | 6 lane(s) green |
| t-072 | pass | 6 lane(s) green |
| t-073 | pass | 6 lane(s) green |
| t-074 | pass | 6 lane(s) green |
| t-075 | pass | 6 lane(s) green |
| t-076 | pass | 6 lane(s) green |
| t-077 | pass | 6 lane(s) green |
| t-078 | expired | last passed at edition-001 (marked 2026-08-10) |
| t-079 | expired | last passed at edition-001 (marked 2026-08-10) |
| t-080 | pass | 6 lane(s) green |
| t-081 | pass | 6 lane(s) green |
| t-082 | pass | 6 lane(s) green |
| t-083 | red | ht_083_02_the_state_prompt_carries_prose_guards_and_inputs |
| t-084 | pass | 6 lane(s) green |
| t-085 | red | ht_085_05_exit_codes_and_receipts_match_the_baseline, ht_085_03_every_terminal_path_ends_the_run_as_before, ht_085_06_a_spawned_child_joins_exactly_as_before |
| t-086 | pass | 6 lane(s) green |
| t-087 | pass | 6 lane(s) green |
| t-088 | pass | 6 lane(s) green |
| t-089 | pass | 6 lane(s) green |
| t-090 | pass | 6 lane(s) green |
| t-091 | pass | 6 lane(s) green |
| t-092 | red | ht_092_05_output_a_traversal_writes_nothing_of_its_own, ht_092_06_cross_feature_every_consumed_feature_fires_in_its_own_stage |
| t-093 | red | ht_093_06_cross_feature_every_run_answers_for_itself |
| t-094 | pass | 6 lane(s) green |
| t-095 | red | ht_095_01_regression_the_live_repository_passes_as_history_grows, ht_095_06_cross_feature_trial_tags_are_ignored_and_near_misses_are_not_editions |
| t-096 | red | ht_096_01_regression_the_annotated_half_keeps_its_verdicts |
| t-097 | pass | 6 lane(s) green |
| t-098 | pass | 6 lane(s) green |
| t-099 | pass | 6 lane(s) green |
| t-100 | red | ht_100_06_cross_feature_gates_and_audit_agree |
| t-101 | pass | 6 lane(s) green |
| t-102 | pass | 6 lane(s) green |
| t-103 | pass | 6 lane(s) green |
| t-104 | pass | 6 lane(s) green |
| t-105 | pass | 6 lane(s) green |
| t-106 | pass | 6 lane(s) green |
| t-107 | pass | 6 lane(s) green |

stray entries (in the folder, not in the roster): t-108 (a crate id the roster does not declare)
