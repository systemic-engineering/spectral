-module(prism_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/prism_test.gleam").
-export([identity_prism_preview_test/0, basis_selector_preview_test/0, preview_no_match_test/0, review_embeds_test/0, modify_identity_transform_test/0, modify_scales_subspace_test/0, compose_intersection_test/0, dimension_accessor_test/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("test/prism_test.gleam", 5).
?DOC(" Identity matrix = traverse all. Preview always matches.\n").
-spec identity_prism_preview_test() -> nil.
identity_prism_preview_test() ->
    P = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, 1.0, +0.0], [+0.0, +0.0, 1.0]]
    ),
    Source = [1.0, 2.0, 3.0],
    Focus@1 = case conversation@prism:preview(P, Source) of
        {ok, Focus} -> Focus;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"prism_test"/utf8>>,
                        function => <<"identity_prism_preview_test"/utf8>>,
                        line => 14,
                        value => _assert_fail,
                        start => 285,
                        'end' => 332,
                        pattern_start => 296,
                        pattern_end => 305})
    end,
    gleeunit@should:equal(Focus@1, [1.0, 2.0, 3.0]).

-file("test/prism_test.gleam", 19).
?DOC(" Single-variant selection: e1 from R3.\n").
-spec basis_selector_preview_test() -> nil.
basis_selector_preview_test() ->
    P = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, +0.0, +0.0], [+0.0, +0.0, +0.0]]
    ),
    Source = [5.0, 7.0, 9.0],
    Focus@1 = case conversation@prism:preview(P, Source) of
        {ok, Focus} -> Focus;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"prism_test"/utf8>>,
                        function => <<"basis_selector_preview_test"/utf8>>,
                        line => 28,
                        value => _assert_fail,
                        start => 592,
                        'end' => 639,
                        pattern_start => 603,
                        pattern_end => 612})
    end,
    gleeunit@should:equal(Focus@1, [5.0, +0.0, +0.0]).

-file("test/prism_test.gleam", 33).
?DOC(" Zero in projected subspace -> Error.\n").
-spec preview_no_match_test() -> nil.
preview_no_match_test() ->
    P = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, +0.0, +0.0], [+0.0, +0.0, +0.0]]
    ),
    Source = [+0.0, 3.0, 4.0],
    gleeunit@should:be_error(conversation@prism:preview(P, Source)).

-file("test/prism_test.gleam", 46).
?DOC(" Review embeds focus into full space via P^T.\n").
-spec review_embeds_test() -> nil.
review_embeds_test() ->
    P = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, 1.0, +0.0], [+0.0, +0.0, +0.0]]
    ),
    Focus = [3.0, 4.0, +0.0],
    Result = conversation@prism:review(P, Focus),
    gleeunit@should:equal(Result, [3.0, 4.0, +0.0]).

-file("test/prism_test.gleam", 60).
?DOC(" Identity transform in modify = no change.\n").
-spec modify_identity_transform_test() -> nil.
modify_identity_transform_test() ->
    P = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, +0.0, +0.0], [+0.0, +0.0, +0.0]]
    ),
    Identity = [[1.0, +0.0, +0.0], [+0.0, 1.0, +0.0], [+0.0, +0.0, 1.0]],
    Source = [5.0, 7.0, 9.0],
    Result = conversation@prism:modify(P, Source, Identity),
    gleeunit@should:equal(Result, [5.0, 7.0, 9.0]).

-file("test/prism_test.gleam", 80).
?DOC(" Transform scales the matched subspace, complement unchanged.\n").
-spec modify_scales_subspace_test() -> nil.
modify_scales_subspace_test() ->
    P = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, +0.0, +0.0], [+0.0, +0.0, +0.0]]
    ),
    Scale2 = [[2.0, +0.0, +0.0], [+0.0, 2.0, +0.0], [+0.0, +0.0, 2.0]],
    Source = [5.0, 7.0, 9.0],
    Result = conversation@prism:modify(P, Source, Scale2),
    gleeunit@should:equal(Result, [10.0, 7.0, 9.0]).

-file("test/prism_test.gleam", 100).
?DOC(" Composing two projections selects intersection of subspaces.\n").
-spec compose_intersection_test() -> nil.
compose_intersection_test() ->
    P1 = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, 1.0, +0.0], [+0.0, +0.0, +0.0]]
    ),
    P2 = conversation@prism:new(
        [[+0.0, +0.0, +0.0], [+0.0, 1.0, +0.0], [+0.0, +0.0, 1.0]]
    ),
    Composed = conversation@prism:compose(P1, P2),
    Source = [1.0, 2.0, 3.0],
    Focus@1 = case conversation@prism:preview(Composed, Source) of
        {ok, Focus} -> Focus;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"prism_test"/utf8>>,
                        function => <<"compose_intersection_test"/utf8>>,
                        line => 118,
                        value => _assert_fail,
                        start => 2428,
                        'end' => 2482,
                        pattern_start => 2439,
                        pattern_end => 2448})
    end,
    gleeunit@should:equal(Focus@1, [+0.0, 2.0, +0.0]).

-file("test/prism_test.gleam", 123).
?DOC(" Opaque type correctly reports dimension.\n").
-spec dimension_accessor_test() -> nil.
dimension_accessor_test() ->
    P = conversation@prism:new(
        [[1.0, +0.0, +0.0], [+0.0, 1.0, +0.0], [+0.0, +0.0, 1.0]]
    ),
    gleeunit@should:equal(conversation@prism:dimension(P), 3).
