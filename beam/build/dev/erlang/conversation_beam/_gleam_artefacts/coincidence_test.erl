-module(coincidence_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/coincidence_test.gleam").
-export([shannon_equivalence_passes_test/0, shannon_equivalence_via_check_property_test/0, connected_trivially_passes_test/0, bipartite_trivially_passes_test/0, exhaustive_passes_test/0, check_property_unknown_property_test/0, check_property_invalid_source_test/0, check_property_no_grammar_test/0]).

-file("test/coincidence_test.gleam", 6).
-spec shannon_equivalence_passes_test() -> nil.
shannon_equivalence_passes_test() ->
    Source = <<"grammar @test {\n  type = a | b | c\n}\n"/utf8>>,
    Reason@1 = case conversation_nif:check_shannon_equivalence(Source) of
        {ok, Reason} -> Reason;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"shannon_equivalence_passes_test"/utf8>>,
                        line => 8,
                        value => _assert_fail,
                        start => 189,
                        'end' => 258,
                        pattern_start => 200,
                        pattern_end => 210})
    end,
    gleeunit@should:not_equal(Reason@1, <<""/utf8>>).

-file("test/coincidence_test.gleam", 12).
-spec shannon_equivalence_via_check_property_test() -> nil.
shannon_equivalence_via_check_property_test() ->
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    Reason@1 = case conversation_nif:check_property(
        Source,
        <<"shannon_equivalence"/utf8>>
    ) of
        {ok, Reason} -> Reason;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"shannon_equivalence_via_check_property_test"/utf8>>,
                        line => 14,
                        value => _assert_fail,
                        start => 404,
                        'end' => 485,
                        pattern_start => 415,
                        pattern_end => 425})
    end,
    gleeunit@should:not_equal(Reason@1, <<""/utf8>>).

-file("test/coincidence_test.gleam", 20).
-spec connected_trivially_passes_test() -> nil.
connected_trivially_passes_test() ->
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    Reason@1 = case conversation_nif:check_connected(Source) of
        {ok, Reason} -> Reason;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"connected_trivially_passes_test"/utf8>>,
                        line => 23,
                        value => _assert_fail,
                        start => 695,
                        'end' => 754,
                        pattern_start => 706,
                        pattern_end => 716})
    end,
    gleeunit@should:not_equal(Reason@1, <<""/utf8>>).

-file("test/coincidence_test.gleam", 29).
-spec bipartite_trivially_passes_test() -> nil.
bipartite_trivially_passes_test() ->
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    Reason@1 = case conversation_nif:check_bipartite(Source) of
        {ok, Reason} -> Reason;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"bipartite_trivially_passes_test"/utf8>>,
                        line => 31,
                        value => _assert_fail,
                        start => 908,
                        'end' => 967,
                        pattern_start => 919,
                        pattern_end => 929})
    end,
    gleeunit@should:not_equal(Reason@1, <<""/utf8>>).

-file("test/coincidence_test.gleam", 37).
-spec exhaustive_passes_test() -> nil.
exhaustive_passes_test() ->
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    Reason@1 = case conversation_nif:check_exhaustive(Source) of
        {ok, Reason} -> Reason;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"exhaustive_passes_test"/utf8>>,
                        line => 39,
                        value => _assert_fail,
                        start => 1113,
                        'end' => 1173,
                        pattern_start => 1124,
                        pattern_end => 1134})
    end,
    gleeunit@should:not_equal(Reason@1, <<""/utf8>>).

-file("test/coincidence_test.gleam", 45).
-spec check_property_unknown_property_test() -> nil.
check_property_unknown_property_test() ->
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    Reason@1 = case conversation_nif:check_property(
        Source,
        <<"nonexistent"/utf8>>
    ) of
        {error, Reason} -> Reason;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"check_property_unknown_property_test"/utf8>>,
                        line => 47,
                        value => _assert_fail,
                        start => 1334,
                        'end' => 1410,
                        pattern_start => 1345,
                        pattern_end => 1358})
    end,
    gleeunit@should:equal(Reason@1, <<"unknown property: nonexistent"/utf8>>).

-file("test/coincidence_test.gleam", 51).
-spec check_property_invalid_source_test() -> {ok, binary()} | {error, binary()}.
check_property_invalid_source_test() ->
    _assert_subject = conversation_nif:check_shannon_equivalence(
        <<"@@@invalid"/utf8>>
    ),
    case _assert_subject of
        {error, _} -> _assert_subject;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"check_property_invalid_source_test"/utf8>>,
                        line => 52,
                        value => _assert_fail,
                        start => 1518,
                        'end' => 1597,
                        pattern_start => 1529,
                        pattern_end => 1543})
    end.

-file("test/coincidence_test.gleam", 55).
-spec check_property_no_grammar_test() -> nil.
check_property_no_grammar_test() ->
    Reason@1 = case conversation_nif:check_shannon_equivalence(
        <<"in @something"/utf8>>
    ) of
        {error, Reason} -> Reason;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_test"/utf8>>,
                        function => <<"check_property_no_grammar_test"/utf8>>,
                        line => 57,
                        value => _assert_fail,
                        start => 1679,
                        'end' => 1760,
                        pattern_start => 1690,
                        pattern_end => 1703})
    end,
    gleeunit@should:equal(Reason@1, <<"no grammar block"/utf8>>).
