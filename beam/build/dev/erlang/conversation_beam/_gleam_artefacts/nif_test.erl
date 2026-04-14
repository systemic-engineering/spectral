-module(nif_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/nif_test.gleam").
-export([parse_conv_returns_oid_test/0, parse_conv_error_test/0, parse_conv_empty_grammar_test/0, parse_conv_deterministic_test/0]).

-file("test/nif_test.gleam", 4).
-spec parse_conv_returns_oid_test() -> nil.
parse_conv_returns_oid_test() ->
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    Oid@1 = case conversation_nif:parse_conv(Source) of
        {ok, Oid} -> Oid;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"nif_test"/utf8>>,
                        function => <<"parse_conv_returns_oid_test"/utf8>>,
                        line => 6,
                        value => _assert_fail,
                        start => 143,
                        'end' => 186,
                        pattern_start => 154,
                        pattern_end => 161})
    end,
    gleeunit@should:not_equal(Oid@1, <<""/utf8>>).

-file("test/nif_test.gleam", 10).
-spec parse_conv_error_test() -> nil.
parse_conv_error_test() ->
    Msg@1 = case conversation_nif:parse_conv(<<"@@@invalid"/utf8>>) of
        {error, Msg} -> Msg;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"nif_test"/utf8>>,
                        function => <<"parse_conv_error_test"/utf8>>,
                        line => 11,
                        value => _assert_fail,
                        start => 253,
                        'end' => 305,
                        pattern_start => 264,
                        pattern_end => 274})
    end,
    gleeunit@should:not_equal(Msg@1, <<""/utf8>>).

-file("test/nif_test.gleam", 15).
-spec parse_conv_empty_grammar_test() -> nil.
parse_conv_empty_grammar_test() ->
    Source = <<"grammar @empty {\n}\n"/utf8>>,
    Oid@1 = case conversation_nif:parse_conv(Source) of
        {ok, Oid} -> Oid;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"nif_test"/utf8>>,
                        function => <<"parse_conv_empty_grammar_test"/utf8>>,
                        line => 17,
                        value => _assert_fail,
                        start => 419,
                        'end' => 462,
                        pattern_start => 430,
                        pattern_end => 437})
    end,
    gleeunit@should:not_equal(Oid@1, <<""/utf8>>).

-file("test/nif_test.gleam", 21).
-spec parse_conv_deterministic_test() -> nil.
parse_conv_deterministic_test() ->
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    Oid1@1 = case conversation_nif:parse_conv(Source) of
        {ok, Oid1} -> Oid1;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"nif_test"/utf8>>,
                        function => <<"parse_conv_deterministic_test"/utf8>>,
                        line => 23,
                        value => _assert_fail,
                        start => 591,
                        'end' => 635,
                        pattern_start => 602,
                        pattern_end => 610})
    end,
    Oid2@1 = case conversation_nif:parse_conv(Source) of
        {ok, Oid2} -> Oid2;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"nif_test"/utf8>>,
                        function => <<"parse_conv_deterministic_test"/utf8>>,
                        line => 24,
                        value => _assert_fail@1,
                        start => 638,
                        'end' => 682,
                        pattern_start => 649,
                        pattern_end => 657})
    end,
    gleeunit@should:equal(Oid1@1, Oid2@1).
