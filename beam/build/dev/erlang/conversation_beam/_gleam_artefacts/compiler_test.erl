-module(compiler_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/compiler_test.gleam").
-export([compile_grammar_returns_trace_test/0, compile_grammar_loads_module_test/0, trace_is_verifiable_test/0, compile_grammar_error_test/0, trace_has_parent_chain_test/0, trace_source_oid_deterministic_test/0, hierarchical_compiler_trace_verifiable_test/0]).

-file("test/compiler_test.gleam", 9).
-spec compile_grammar_returns_trace_test() -> nil.
compile_grammar_returns_trace_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"compile_grammar_returns_trace_test"/utf8>>,
                        line => 10,
                        value => _assert_fail,
                        start => 223,
                        'end' => 264,
                        pattern_start => 234,
                        pattern_end => 245})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @test_compile {\n  type = a | b\n}\n"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"compile_grammar_returns_trace_test"/utf8>>,
                        line => 17,
                        value => _assert_fail@1,
                        start => 449,
                        'end' => 500,
                        pattern_start => 460,
                        pattern_end => 469})
    end,
    case conversation@trace:value(T@1) of
        {compiled_domain, <<"test_compile"/utf8>>, _, _} ->
            gleeunit@should:be_true(true);

        _ ->
            gleeunit@should:be_true(false)
    end,
    gleam@erlang@process:send(Subject, shutdown).

-file("test/compiler_test.gleam", 25).
-spec compile_grammar_loads_module_test() -> nil.
compile_grammar_loads_module_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"compile_grammar_loads_module_test"/utf8>>,
                        line => 26,
                        value => _assert_fail,
                        start => 733,
                        'end' => 774,
                        pattern_start => 744,
                        pattern_end => 755})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @test_loaded {\n  type = x | y\n}\n"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"compile_grammar_loads_module_test"/utf8>>,
                        line => 33,
                        value => _assert_fail@1,
                        start => 958,
                        'end' => 1009,
                        pattern_start => 969,
                        pattern_end => 978})
    end,
    Compiled = conversation@trace:value(T@1),
    gleeunit@should:equal(
        erlang:element(4, Compiled),
        <<"conv_test_loaded"/utf8>>
    ),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/compiler_test.gleam", 40).
-spec trace_is_verifiable_test() -> nil.
trace_is_verifiable_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"trace_is_verifiable_test"/utf8>>,
                        line => 41,
                        value => _assert_fail,
                        start => 1237,
                        'end' => 1278,
                        pattern_start => 1248,
                        pattern_end => 1259})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @test_verify {\n  type = p | q\n}\n"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"trace_is_verifiable_test"/utf8>>,
                        line => 48,
                        value => _assert_fail@1,
                        start => 1462,
                        'end' => 1513,
                        pattern_start => 1473,
                        pattern_end => 1482})
    end,
    _pipe = conversation@trace:verify(T@1, conversation@compiler:public_key()),
    gleeunit@should:be_true(_pipe),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/compiler_test.gleam", 53).
-spec compile_grammar_error_test() -> nil.
compile_grammar_error_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"compile_grammar_error_test"/utf8>>,
                        line => 54,
                        value => _assert_fail,
                        start => 1661,
                        'end' => 1702,
                        pattern_start => 1672,
                        pattern_end => 1683})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar, <<"template $t {\n  slug\n}\n"/utf8>>, Reply}
    ),
    case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {error, _}} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"compile_grammar_error_test"/utf8>>,
                        line => 62,
                        value => _assert_fail@1,
                        start => 1916,
                        'end' => 1973,
                        pattern_start => 1927,
                        pattern_end => 1942})
    end,
    gleeunit@should:be_true(true),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/compiler_test.gleam", 67).
-spec trace_has_parent_chain_test() -> nil.
trace_has_parent_chain_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"trace_has_parent_chain_test"/utf8>>,
                        line => 68,
                        value => _assert_fail,
                        start => 2084,
                        'end' => 2125,
                        pattern_start => 2095,
                        pattern_end => 2106})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @test_chain {\n  type = x | y\n}\n"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"trace_has_parent_chain_test"/utf8>>,
                        line => 78,
                        value => _assert_fail@1,
                        start => 2327,
                        'end' => 2378,
                        pattern_start => 2338,
                        pattern_end => 2347})
    end,
    case erlang:element(4, T@1) of
        {some, _} ->
            gleeunit@should:be_true(true);

        none ->
            gleeunit@should:be_true(false)
    end,
    gleam@erlang@process:send(Subject, shutdown).

-file("test/compiler_test.gleam", 87).
-spec trace_source_oid_deterministic_test() -> nil.
trace_source_oid_deterministic_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"trace_source_oid_deterministic_test"/utf8>>,
                        line => 88,
                        value => _assert_fail,
                        start => 2658,
                        'end' => 2699,
                        pattern_start => 2669,
                        pattern_end => 2680})
    end,
    Subject = erlang:element(3, Started@1),
    Source = <<"grammar @test_det {\n  type = a | b\n}\n"/utf8>>,
    Reply1 = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(Subject, {compile_grammar, Source, Reply1}),
    T1@1 = case gleam@erlang@process:'receive'(Reply1, 5000) of
        {ok, {ok, T1}} -> T1;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"trace_source_oid_deterministic_test"/utf8>>,
                        line => 94,
                        value => _assert_fail@1,
                        start => 2892,
                        'end' => 2945,
                        pattern_start => 2903,
                        pattern_end => 2913})
    end,
    Reply2 = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(Subject, {compile_grammar, Source, Reply2}),
    T2@1 = case gleam@erlang@process:'receive'(Reply2, 5000) of
        {ok, {ok, T2}} -> T2;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"trace_source_oid_deterministic_test"/utf8>>,
                        line => 98,
                        value => _assert_fail@2,
                        start => 3051,
                        'end' => 3104,
                        pattern_start => 3062,
                        pattern_end => 3072})
    end,
    V1 = conversation@trace:value(T1@1),
    V2 = conversation@trace:value(T2@1),
    gleeunit@should:equal(erlang:element(3, V1), erlang:element(3, V2)),
    gleeunit@should:equal(
        erlang:element(3, V1),
        conversation@oid:from_bytes(<<Source/binary>>)
    ),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/compiler_test.gleam", 110).
-spec hierarchical_compiler_trace_verifiable_test() -> nil.
hierarchical_compiler_trace_verifiable_test() ->
    Root = conversation@key:generate(),
    Root_pub = conversation@key:public_key(Root),
    Started@1 = case conversation@compiler:start_with_root(Root_pub) of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"hierarchical_compiler_trace_verifiable_test"/utf8>>,
                        line => 113,
                        value => _assert_fail,
                        start => 3537,
                        'end' => 3596,
                        pattern_start => 3548,
                        pattern_end => 3559})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @test_hier {\n  type = a | b\n}\n"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"compiler_test"/utf8>>,
                        function => <<"hierarchical_compiler_trace_verifiable_test"/utf8>>,
                        line => 123,
                        value => _assert_fail@1,
                        start => 3797,
                        'end' => 3848,
                        pattern_start => 3808,
                        pattern_end => 3817})
    end,
    Derived_pub = conversation@compiler:public_key_from(Root_pub),
    _pipe = conversation@trace:verify(T@1, Derived_pub),
    gleeunit@should:be_true(_pipe),
    _pipe@1 = conversation@trace:verify(T@1, conversation@compiler:public_key()),
    gleeunit@should:be_false(_pipe@1),
    gleam@erlang@process:send(Subject, shutdown).
