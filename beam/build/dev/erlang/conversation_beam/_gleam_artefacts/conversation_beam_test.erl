-module(conversation_beam_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/conversation_beam_test.gleam").
-export([main/0, pass_spec_test/0, desired_state_spec_test/0, wildcard_arm_matches_test/0, cmp_arm_falls_through_to_wildcard_test/0, when_guard_applies_test/0, empty_case_produces_no_deltas_test/0, branch_empty_test/0, branch_single_wildcard_fires_test/0, branch_all_wildcards_fire_test/0, branch_cmp_no_match_skipped_test/0, branch_collects_matching_skips_nonmatching_test/0, all_ops_construct_test/0]).

-file("test/conversation_beam_test.gleam", 9).
-spec main() -> nil.
main() ->
    gleeunit:main().

-file("test/conversation_beam_test.gleam", 15).
-spec pass_spec_test() -> nil.
pass_spec_test() ->
    Spec = pass,
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(_pipe, []).

-file("test/conversation_beam_test.gleam", 20).
-spec desired_state_spec_test() -> nil.
desired_state_spec_test() ->
    Spec = {desired_state, <<"health_monitor"/utf8>>, <<"critical"/utf8>>},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(
        _pipe,
        [{start_process, <<"health_monitor"/utf8>>, <<"critical"/utf8>>}]
    ).

-file("test/conversation_beam_test.gleam", 25).
-spec wildcard_arm_matches_test() -> nil.
wildcard_arm_matches_test() ->
    Spec = {'case',
        <<"x"/utf8>>,
        [{arm, wildcard, {desired_state, <<"p"/utf8>>, <<"s"/utf8>>}}]},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(_pipe, [{start_process, <<"p"/utf8>>, <<"s"/utf8>>}]).

-file("test/conversation_beam_test.gleam", 30).
-spec cmp_arm_falls_through_to_wildcard_test() -> nil.
cmp_arm_falls_through_to_wildcard_test() ->
    Spec = {'case',
        <<"x"/utf8>>,
        [{arm,
                {cmp, gt, <<"0.1"/utf8>>},
                {desired_state, <<"a"/utf8>>, <<"high"/utf8>>}},
            {arm, wildcard, {desired_state, <<"a"/utf8>>, <<"low"/utf8>>}}]},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(
        _pipe,
        [{start_process, <<"a"/utf8>>, <<"low"/utf8>>}]
    ).

-file("test/conversation_beam_test.gleam", 40).
-spec when_guard_applies_test() -> nil.
when_guard_applies_test() ->
    Spec = {'when',
        gt,
        <<"error.rate"/utf8>>,
        <<"0.1"/utf8>>,
        {desired_state, <<"monitor"/utf8>>, <<"alert"/utf8>>}},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(
        _pipe,
        [{start_process, <<"monitor"/utf8>>, <<"alert"/utf8>>}]
    ).

-file("test/conversation_beam_test.gleam", 45).
-spec empty_case_produces_no_deltas_test() -> nil.
empty_case_produces_no_deltas_test() ->
    Spec = {'case', <<"x"/utf8>>, []},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(_pipe, []).

-file("test/conversation_beam_test.gleam", 52).
-spec branch_empty_test() -> nil.
branch_empty_test() ->
    Spec = {branch, []},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(_pipe, []).

-file("test/conversation_beam_test.gleam", 57).
-spec branch_single_wildcard_fires_test() -> nil.
branch_single_wildcard_fires_test() ->
    Spec = {branch,
        [{arm, wildcard, {desired_state, <<"p"/utf8>>, <<"s"/utf8>>}}]},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(_pipe, [{start_process, <<"p"/utf8>>, <<"s"/utf8>>}]).

-file("test/conversation_beam_test.gleam", 62).
-spec branch_all_wildcards_fire_test() -> nil.
branch_all_wildcards_fire_test() ->
    Spec = {branch,
        [{arm, wildcard, {desired_state, <<"a"/utf8>>, <<"x"/utf8>>}},
            {arm, wildcard, {desired_state, <<"b"/utf8>>, <<"y"/utf8>>}}]},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(
        _pipe,
        [{start_process, <<"a"/utf8>>, <<"x"/utf8>>},
            {start_process, <<"b"/utf8>>, <<"y"/utf8>>}]
    ).

-file("test/conversation_beam_test.gleam", 72).
-spec branch_cmp_no_match_skipped_test() -> nil.
branch_cmp_no_match_skipped_test() ->
    Spec = {branch,
        [{arm,
                {cmp, gt, <<"0.1"/utf8>>},
                {desired_state, <<"a"/utf8>>, <<"high"/utf8>>}}]},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(_pipe, []).

-file("test/conversation_beam_test.gleam", 78).
-spec branch_collects_matching_skips_nonmatching_test() -> nil.
branch_collects_matching_skips_nonmatching_test() ->
    Spec = {branch,
        [{arm,
                {cmp, gt, <<"0.1"/utf8>>},
                {desired_state, <<"a"/utf8>>, <<"high"/utf8>>}},
            {arm, wildcard, {desired_state, <<"b"/utf8>>, <<"low"/utf8>>}}]},
    _pipe = conversation@runtime:converge(Spec),
    gleeunit@should:equal(
        _pipe,
        [{start_process, <<"b"/utf8>>, <<"low"/utf8>>}]
    ).

-file("test/conversation_beam_test.gleam", 90).
-spec all_ops_construct_test() -> nil.
all_ops_construct_test() ->
    _ = {cmp, gt, <<"1"/utf8>>},
    _ = {cmp, lt, <<"2"/utf8>>},
    _ = {cmp, gte, <<"3"/utf8>>},
    _ = {cmp, lte, <<"4"/utf8>>},
    _ = {cmp, eq, <<"5"/utf8>>},
    _ = {cmp, ne, <<"6"/utf8>>},
    gleeunit@should:be_true(true).
