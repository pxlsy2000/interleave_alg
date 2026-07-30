# v1 Conformance Traceability

This document maps every normative corner-case row in Chapter 8 and every
acceptance bullet in Chapter 9 of `docs/spec.en.md` to at least one named test
ID. Integration-test IDs use the form
`<integration-test-target>::<test-function>`; bounded-output seam tests use
their fully qualified library-test ID.

Run all listed tests with:

```text
cargo test --locked --all-targets
```

## Chapter 8: Deterministic Corner Cases

### 8.1 Mapping

| ID | Normative case | Passing test ID |
| --- | --- | --- |
| 8.1-01 | `targets.count = 1` | `conformance_mapping::g1_n1_preserves_the_complete_line_address_and_zero_offset` |
| 8.1-02 | `N = 2^n` | `conformance_mapping::n_equals_two_to_n_has_zero_local_line_at_the_address_maximum` |
| 8.1-03 | `granule_bytes = 1` | `conformance_mapping::g1_n1_preserves_the_complete_line_address_and_zero_offset` |
| 8.1-04 | Granule exceeds `2^A` | `mapping_schema::enforces_cross_family_precedence_and_exact_reasons` |
| 8.1-05 | Granule exceeds `2^52` after the intrinsic relation passes | `mapping_schema::enforces_cross_family_precedence_and_exact_reasons`; `conformance_mapping::validate_cli_locks_mapping_support_caps_and_overlong_scalar_exit_two` |
| 8.1-06 | Target count or granule is not a power of two | `conformance_exclusions::non_power_of_two_targets_are_rejected_without_fallback`; `conformance_mapping::validate_cli_locks_mapping_support_caps_and_overlong_scalar_exit_two` |
| 8.1-07 | Target count exceeds 65,536 | `mapping_schema::rejects_target_cap_only_after_intrinsic_relation_passes`; `conformance_mapping::validate_cli_locks_mapping_support_caps_and_overlong_scalar_exit_two` |
| 8.1-08 | Target count exceeds the line-address combinations | `mapping_schema::enforces_intrinsic_relations_before_support_caps` |
| 8.1-09 | Tap is negative or at least `n` | `mapping_schema::applies_first_constraint_to_rows_and_taps` |
| 8.1-10 | Row repeats a tap | `conformance_mapping::duplicate_taps_are_rejected_instead_of_xor_canceled` |
| 8.1-11 | `M` or explicit `L` has the wrong row count | `mapping_schema::validates_dimensions_taps_duplicates_and_conditional_l_rows`; `mapping_schema::reports_explicit_row_count_and_keeps_row_source_order` |
| 8.1-12 | `preserve_high` also supplies rows | `mapping_schema::validates_dimensions_taps_duplicates_and_conditional_l_rows` |
| 8.1-13 | `explicit` omits rows | `mapping_schema::explicit_mode_requires_rows_and_preserves_empty_rows` |
| 8.1-14 | Structurally valid Mapping has an unreachable Target | `mapping_math::retains_all_checks_but_only_unreachable_primary_error` |
| 8.1-15 | Reachable Targets but rank-deficient `F` | `conformance_output::json_business_failure_is_a_complete_envelope_on_stdout` |
| 8.1-16 | Bijective Mapping has non-natural LA order | `conformance_mapping::non_natural_warning_is_retained_by_validate_map_and_run` |

### 8.2 Addresses and Numeric Values

| ID | Normative case | Passing test ID |
| --- | --- | --- |
| 8.2-01 | Address equals `2^A - 1` | `conformance_mapping::a64_maximum_passes_and_exclusive_bound_fails_without_rows` |
| 8.2-02 | Address is at least `2^A` | `conformance_mapping::a64_maximum_passes_and_exclusive_bound_fails_without_rows`; `map_cli::arbitrary_address_magnitudes_use_range_errors_and_full_canonical_hex` |
| 8.2-03 | Address is negative | `map_cli::direct_negative_hex_reaches_address_validation_with_options_on_either_side` |
| 8.2-04 | Address has a sign, leading zero, or invalid underscore | `map_cli::every_invalid_address_grammar_edge_is_rejected_without_rows` |
| 8.2-05 | A 64-bit generated-address expression carries | `conformance_scenario::duplicate_sweep_values_and_generated_carry_fail_before_analysis` |
| 8.2-06 | Base or stride is unaligned | `address_mapping::maps_documented_example_and_preserves_unaligned_offset`; `scenario_generate::unaligned_base_and_stride_preserve_generated_address_byte_offsets` |
| 8.2-07 | Stride is zero | `conformance_scenario::zero_stride_repeats_one_address_for_the_complete_test` |
| 8.2-08 | Decoded scalar is exactly 128 UTF-8 bytes | `yaml_contract::decoded_scalar_bound_accepts_exact_plain_and_quoted_key_values` |
| 8.2-09 | Decoded scalar is 129 UTF-8 bytes | `yaml_contract::decoded_scalar_bound_rejects_plain_quoted_and_multibyte_key_values_at_plus_one`; `yaml_contract::first_overlong_decoded_scalar_wins_by_source_byte_position`; `conformance_mapping::validate_cli_locks_mapping_support_caps_and_overlong_scalar_exit_two`; `run_cli::overlong_decoded_scenario_scalar_is_a_json_business_failure` |
| 8.2-10 | Prohibited YAML competes with an overlong scalar | `yaml_contract::prohibited_yaml_retains_precedence_over_decoded_scalar_bound` |
| 8.2-11 | Core integer first-match spellings use base-10 observations | `yaml_contract::core_integer_first_match_keeps_lexemes_and_separates_signed_base_prefixes`; `yaml_contract::root_integer_errors_use_exact_ungrouped_decimal_observations` |
| 8.2-12 | Signed octal and hexadecimal spellings remain strings | `yaml_contract::core_integer_first_match_keeps_lexemes_and_separates_signed_base_prefixes` |

### 8.3 Scenario

| ID | Normative case | Passing test ID |
| --- | --- | --- |
| 8.3-01 | Access count is zero | `scenario_schema::rejects_empty_shapes_wrong_kind_fields_and_invalid_schedule` |
| 8.3-02 | Window is zero, duplicated, or list is empty | `scenario_schema::enforces_scalar_type_lexeme_and_range_ladders`; `scenario_schema::rejects_duplicate_names_windows_and_normalized_sweep_addresses`; `scenario_schema::rejects_empty_shapes_wrong_kind_fields_and_invalid_schedule` |
| 8.3-03 | Effective window exceeds `Q` | `scenario_expand::selected_window_larger_than_q_uses_its_source_path` |
| 8.3-04 | Duplicate case names | `scenario_schema::rejects_duplicate_names_windows_and_normalized_sweep_addresses` |
| 8.3-05 | Duplicate stream names | `scenario_schema::enforces_case_and_stream_name_contracts_and_stream_uniqueness` |
| 8.3-06 | Sweep base or stride list is empty | `scenario_schema::rejects_empty_shapes_wrong_kind_fields_and_invalid_schedule` |
| 8.3-07 | Sweep base or stride value repeats | `conformance_scenario::duplicate_sweep_values_and_generated_carry_fail_before_analysis`; `scenario_schema::rejects_duplicate_names_windows_and_normalized_sweep_addresses` |
| 8.3-08 | Multi-stream case has one stream | `conformance_scenario::unequal_and_single_stream_round_robin_keep_declared_order`; `scenario_generate::one_stream_round_robin_matches_its_linear_sequence` |
| 8.3-09 | Multi-stream lengths differ | `conformance_scenario::unequal_and_single_stream_round_robin_keep_declared_order`; `scenario_generate::unequal_streams_use_active_declaration_order_each_round` |
| 8.3-10 | All cases are disabled without `--case` | `run_cli::missing_and_empty_selection_have_deterministic_failures` |
| 8.3-11 | Disabled case is selected explicitly | `conformance_scenario::disabled_selection_and_repeated_case_names_are_deterministic` |
| 8.3-12 | Repeated `--case` runs once | `conformance_scenario::disabled_selection_and_repeated_case_names_are_deterministic` |
| 8.3-13 | Generated address is out of range | `conformance_scenario::duplicate_sweep_values_and_generated_carry_fail_before_analysis` |
| 8.3-14 | Per-case or global resource cap is exceeded | `run_cli::all_five_global_resource_caps_fail_as_scenario_errors`; `run_cli::per_case_window_and_stream_caps_fail_before_analysis` |
| 8.3-15 | `Q=10,000,000`, `K=1,024` exceeds exact window work | `scenario_expand::single_maximum_access_descriptor_with_1024_windows_fails_exact_work_sum` |

### 8.4 Metrics

| ID | Normative case | Passing test ID |
| --- | --- | --- |
| 8.4-01 | `Q` is not divisible by `N` | `metrics::counts_and_max_load_use_exact_unreduced_formulas` |
| 8.4-02 | `W < N` | `metrics::counts_and_max_load_use_exact_unreduced_formulas`; `metrics::exhaustive_small_sequences_match_simple_oracle` |
| 8.4-03 | Maximum-load Targets tie | `conformance_scenario::all_metric_ties_follow_the_normative_representatives` |
| 8.4-04 | Worst windows tie | `conformance_scenario::all_metric_ties_follow_the_normative_representatives` |
| 8.4-05 | Longest runs tie | `conformance_scenario::all_metric_ties_follow_the_normative_representatives` |
| 8.4-06 | A Target is never accessed | `conformance_scenario::all_metric_ties_follow_the_normative_representatives` |

## Chapter 9: Acceptance Criteria

### 9.1 Mathematical Correctness

| ID | Acceptance criterion | Passing test ID |
| --- | --- | --- |
| 9.1-01 | Small address spaces match direct per-address enumeration | `address_mapping::natural_mapping_matches_direct_formula_for_every_small_address` |
| 9.1-02 | `rank(M)=r` agrees with exhaustive Target reachability | `mapping_math::ranks_match_direct_enumeration_for_every_input` |
| 9.1-03 | `rank(F)=n` agrees with no duplicate or hole | `mapping_math::ranks_match_direct_enumeration_for_every_input` |
| 9.1-04 | Natural predicate yields exact LA order for each Target | `address_mapping::natural_mapping_matches_direct_formula_for_every_small_address` |
| 9.1-05 | Permuted LA is non-natural and warns in all three commands | `conformance_mapping::non_natural_warning_is_retained_by_validate_map_and_run` |
| 9.1-06 | Every Mapping preserves the byte offset | `address_mapping::natural_mapping_matches_direct_formula_for_every_small_address`; `address_mapping::explicit_local_rows_preserve_every_nontrivial_byte_offset` |

### 9.2 Input and Commands

| ID | Acceptance criterion | Passing test ID |
| --- | --- | --- |
| 9.2-01 | Both templates feed their corresponding commands unchanged | `template_cli::generated_templates_feed_the_mapping_and_scenario_preflight_unchanged` |
| 9.2-02 | Unknown fields, duplicate fields, and wrong types are rejected | `mapping_schema::reports_unknown_raw_key_and_missing_fields_in_contract_order`; `mapping_schema::reports_wrong_containers_without_descendant_cascades`; `scenario_schema::combined_failures_follow_declaration_and_source_order_without_fallback`; `scenario_schema::wrong_typed_kind_validates_only_common_fields_and_unknown_keys`; `yaml_contract::duplicate_key_uses_second_occurrence_and_terminal_safe_path` |
| 9.2-03 | Decoded-scalar exact/plus-one, ordering, precedence, and exits | `yaml_contract::decoded_scalar_bound_accepts_exact_plain_and_quoted_key_values`; `yaml_contract::decoded_scalar_bound_rejects_plain_quoted_and_multibyte_key_values_at_plus_one`; `yaml_contract::first_overlong_decoded_scalar_wins_by_source_byte_position`; `yaml_contract::prohibited_yaml_retains_precedence_over_decoded_scalar_bound`; `conformance_mapping::validate_cli_locks_mapping_support_caps_and_overlong_scalar_exit_two`; `run_cli::overlong_decoded_scenario_scalar_is_a_json_business_failure` |
| 9.2-04 | Core first-match and typed-v1 lexeme gates are independent | `yaml_contract::core_integer_first_match_keeps_lexemes_and_separates_signed_base_prefixes`; `yaml_contract::root_integer_errors_use_exact_ungrouped_decimal_observations`; `mapping_schema::applies_scalar_type_lexeme_and_name_gates`; `scenario_schema::enforces_scalar_type_lexeme_and_range_ladders`; `scalar_contract::public_generic_integer_parser_is_distinct_from_address_parser` |
| 9.2-05 | Options, selection order, and exits match Chapters 5 and 7 | `cli_usage::duplicate_singleton_options_are_usage_failures_before_input_read`; `validate_cli::invalid_mathematics_keeps_a_complete_result_and_exits_two`; `map_cli::all_bad_operands_are_reported_in_source_order_after_mapping_validation`; `run_cli::all_five_global_resource_caps_fail_as_scenario_errors`; `conformance_scenario::disabled_selection_and_repeated_case_names_are_deterministic`; `app::output::tests::stdout_report_cap_accepts_exact_json_and_rejects_plus_one_without_partial_output` |
| 9.2-06 | Every Chapter 8 corner case is locked | `conformance_mapping::g1_n1_preserves_the_complete_line_address_and_zero_offset`; `conformance_scenario::all_metric_ties_follow_the_normative_representatives`; the 49-row Chapter 8 matrix above |
| 9.2-07 | Failures leave no partial result or truncated output | `conformance_output::preflight_failure_leaves_existing_text_output_and_directory_untouched`; `map_cli::a_late_bad_address_suppresses_every_preceding_valid_row`; `app::output::tests::stdout_report_cap_accepts_exact_json_and_rejects_plus_one_without_partial_output`; `app::output::tests::file_report_cap_accepts_exact_text_and_rejects_plus_one_atomically` |

### 9.3 Performance Metrics

| ID | Acceptance criterion | Passing test ID |
| --- | --- | --- |
| 9.3-01 | Target counts sum to `Q` | `metrics::exhaustive_small_sequences_match_simple_oracle` |
| 9.3-02 | Maximum load, windows, and run match direct formulas | `metrics::exhaustive_small_sequences_match_simple_oracle` |
| 9.3-03 | Equal long-term counts can differ by access order | `metrics::same_counts_in_different_order_change_windows_and_runs` |
| 9.3-04 | Aligned and staggered streams differ deterministically | `scenario_generate::aligned_and_staggered_multistream_generation_produces_hand_calculated_metrics` |
| 9.3-05 | Sweep order, IDs, and metrics are independent | `scenario_generate::sweep_combinations_keep_independent_metrics_in_base_major_order` |

### 9.4 Output

| ID | Acceptance criterion | Passing test ID |
| --- | --- | --- |
| 9.4-01 | Text alone states Mapping validity | `text_golden::every_validate_classification_matches_golden` |
| 9.4-02 | Warnings and errors explain their effect | `text_golden::every_validate_classification_matches_golden`; `conformance_output::text_parse_schema_math_and_preflight_failures_use_stderr_only` |
| 9.4-03 | Text and JSON express the same conclusions | `text_golden::text_parity::text_and_json_project_the_same_run_semantics` |
| 9.4-04 | JSON fields, ordering, addresses, and ratios match Chapter 6 | `json_golden::json_validation_cases::valid_natural_validation_matches_golden_shape`; `json_golden::structural_failure_has_null_result_when_rendered`; `json_golden::json_command_cases::map_success_preserves_order_and_canonicalizes_addresses`; `json_golden::json_command_cases::run_success_matches_ordered_multi_case_golden`; `json_golden::every_number_is_safe_and_rendering_is_byte_stable`; `input::limits::tests::v1_limits_match_the_normative_operational_table`; `report::bounded::tests::shared_sink_accepts_exact_limit_and_rejects_next_byte_without_retaining_it`; `report::json::tests::json_report_accepts_the_exact_limit_and_rejects_the_next_byte`; `report::text::tests::text_report_accepts_the_exact_limit_and_rejects_the_next_byte` |
| 9.4-05 | Identical input has identical order and result | `conformance_output::text_and_json_tie_reports_match_reviewed_goldens_byte_for_byte` |

## Chapter 10: Explicit Exclusions

| Excluded capability | Rejection and absence test ID |
| --- | --- |
| Non-power-of-two Target count | `conformance_exclusions::non_power_of_two_targets_are_rejected_without_fallback` |
| Unequal Target capacities | `conformance_exclusions::unequal_capacity_and_hole_fields_are_both_unknown` |
| Address-space holes | `conformance_exclusions::unequal_capacity_and_hole_fields_are_both_unknown` |
| TOML or JSON configuration input | `conformance_exclusions::json_and_toml_roots_never_fall_back_to_other_decoders` |
| Hardware trace import | `conformance_exclusions::trace_import_command_and_scenario_field_are_both_absent` |
| Non-round-robin scheduling | `conformance_exclusions::non_round_robin_schedule_is_rejected_without_substitution` |
| Aggregate sweep metrics | `conformance_exclusions::aggregate_sweep_field_and_summary_are_both_absent` |
