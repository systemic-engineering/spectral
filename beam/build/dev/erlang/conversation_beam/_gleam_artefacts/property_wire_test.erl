-module(property_wire_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/property_wire_test.gleam").
-export([compiled_module_has_ensures_test/0, compiled_module_empty_ensures_test/0, boot_ordering_test/0, boot_empty_grammars_test/0, compiled_module_has_requires_test/0, compiled_module_has_invariants_test/0, compiler_calls_coincidence_on_requires_test/0, compiled_module_empty_requires_invariants_test/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("test/property_wire_test.gleam", 98).
?DOC(" Compiled module exposes ensures/0 with declared ensures.\n").
-spec compiled_module_has_ensures_test() -> nil.
compiled_module_has_ensures_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_ensures_test"/utf8>>,
                        line => 99,
                        value => _assert_fail,
                        start => 3134,
                        'end' => 3175,
                        pattern_start => 3145,
                        pattern_end => 3156})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    Source = <<"grammar @ensures_wire_test {
  type = a | b

  ensures response_time
}
"/utf8>>,
    gleam@erlang@process:send(Subject, {compile_grammar, Source, Reply}),
    T@1 = case gleam@erlang@process:'receive'(Reply, 10000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_ensures_test"/utf8>>,
                        line => 111,
                        value => _assert_fail@1,
                        start => 3401,
                        'end' => 3454,
                        pattern_start => 3412,
                        pattern_end => 3421})
    end,
    Compiled = conversation@trace:value(T@1),
    Ensures@1 = case loader_ffi:get_ensures(erlang:element(4, Compiled)) of
        {ok, Ensures} -> Ensures;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_ensures_test"/utf8>>,
                        line => 114,
                        value => _assert_fail@2,
                        start => 3490,
                        'end' => 3550,
                        pattern_start => 3501,
                        pattern_end => 3512})
    end,
    gleeunit@should:equal(Ensures@1, [<<"response_time"/utf8>>]),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/property_wire_test.gleam", 121).
?DOC(" Module without ensures has empty ensures/0.\n").
-spec compiled_module_empty_ensures_test() -> nil.
compiled_module_empty_ensures_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_empty_ensures_test"/utf8>>,
                        line => 122,
                        value => _assert_fail,
                        start => 3737,
                        'end' => 3778,
                        pattern_start => 3748,
                        pattern_end => 3759})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @no_ensures {\n  type = x | y\n}\n"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 10000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_empty_ensures_test"/utf8>>,
                        line => 133,
                        value => _assert_fail@1,
                        start => 3981,
                        'end' => 4034,
                        pattern_start => 3992,
                        pattern_end => 4001})
    end,
    Compiled = conversation@trace:value(T@1),
    Ensures@1 = case loader_ffi:get_ensures(erlang:element(4, Compiled)) of
        {ok, Ensures} -> Ensures;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_empty_ensures_test"/utf8>>,
                        line => 136,
                        value => _assert_fail@2,
                        start => 4070,
                        'end' => 4130,
                        pattern_start => 4081,
                        pattern_end => 4092})
    end,
    gleeunit@should:equal(Ensures@1, []),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/property_wire_test.gleam", 143).
?DOC(" Boot ordering: infra domains before app domains.\n").
-spec boot_ordering_test() -> {ok, nil} | {error, gleam@dynamic:dynamic_()}.
boot_ordering_test() ->
    Infra = <<"grammar @infra {
  type = service
}
"/utf8>>,
    App = <<"grammar @app {
  type = feature
  requires shannon_equivalence
}
"/utf8>>,
    Compiler_name = gleam_erlang_ffi:new_name(<<"compiler"/utf8>>),
    Garden_name = gleam_erlang_ffi:new_name(<<"garden"/utf8>>),
    case conversation@supervisor:start(Compiler_name, Garden_name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"boot_ordering_test"/utf8>>,
                        line => 156,
                        value => _assert_fail,
                        start => 4523,
                        'end' => 4584,
                        pattern_start => 4534,
                        pattern_end => 4539})
    end,
    _ = coincidence_server:start(),
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    Infra_beams@1 = case conversation@boot:boot(Subject, Garden_name, [Infra]) of
        {ok, Infra_beams} -> Infra_beams;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"boot_ordering_test"/utf8>>,
                        line => 161,
                        value => _assert_fail@1,
                        start => 4710,
                        'end' => 4783,
                        pattern_start => 4721,
                        pattern_end => 4736})
    end,
    App_beams@1 = case conversation@boot:boot(Subject, Garden_name, [App]) of
        {ok, App_beams} -> App_beams;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"boot_ordering_test"/utf8>>,
                        line => 163,
                        value => _assert_fail@2,
                        start => 4786,
                        'end' => 4855,
                        pattern_start => 4797,
                        pattern_end => 4810})
    end,
    Infra_domains = conversation@boot:results(Infra_beams@1),
    App_domains = conversation@boot:results(App_beams@1),
    gleeunit@should:be_true(domain_server:is_running(<<"infra"/utf8>>)),
    gleeunit@should:be_true(domain_server:is_running(<<"app"/utf8>>)),
    Infra_names = gleam@list:map(
        Infra_domains,
        fun(D) -> erlang:element(2, D) end
    ),
    App_names = gleam@list:map(
        App_domains,
        fun(D@1) -> erlang:element(2, D@1) end
    ),
    gleeunit@should:equal(Infra_names, [<<"infra"/utf8>>]),
    gleeunit@should:equal(App_names, [<<"app"/utf8>>]),
    _ = coincidence_server:stop().

-file("test/property_wire_test.gleam", 182).
?DOC(" Boot handles empty grammar list.\n").
-spec boot_empty_grammars_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
boot_empty_grammars_test() ->
    App = <<"grammar @solo_app {
  type = widget
}
"/utf8>>,
    Compiler_name = gleam_erlang_ffi:new_name(<<"compiler"/utf8>>),
    Garden_name = gleam_erlang_ffi:new_name(<<"garden"/utf8>>),
    case conversation@supervisor:start(Compiler_name, Garden_name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"boot_empty_grammars_test"/utf8>>,
                        line => 190,
                        value => _assert_fail,
                        start => 5550,
                        'end' => 5611,
                        pattern_start => 5561,
                        pattern_end => 5566})
    end,
    _ = coincidence_server:start(),
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [App]) of
        {ok, Beams} -> Beams;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"boot_empty_grammars_test"/utf8>>,
                        line => 194,
                        value => _assert_fail@1,
                        start => 5705,
                        'end' => 5766,
                        pattern_start => 5716,
                        pattern_end => 5725})
    end,
    Domains = conversation@boot:results(Beams@1),
    gleeunit@should:be_true(domain_server:is_running(<<"solo_app"/utf8>>)),
    Names = gleam@list:map(Domains, fun(D) -> erlang:element(2, D) end),
    gleeunit@should:equal(Names, [<<"solo_app"/utf8>>]),
    _ = coincidence_server:stop().

-file("test/property_wire_test.gleam", 26).
?DOC(" Compiled module exposes requires/0 with declared properties.\n").
-spec compiled_module_has_requires_test() -> nil.
compiled_module_has_requires_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_requires_test"/utf8>>,
                        line => 27,
                        value => _assert_fail,
                        start => 586,
                        'end' => 627,
                        pattern_start => 597,
                        pattern_end => 608})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @prop_test {
  type = a | b | c

  requires shannon_equivalence
  invariant connected
}
"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_requires_test"/utf8>>,
                        line => 34,
                        value => _assert_fail@1,
                        start => 787,
                        'end' => 838,
                        pattern_start => 798,
                        pattern_end => 807})
    end,
    Compiled = conversation@trace:value(T@1),
    Requires@1 = case loader_ffi:get_requires(erlang:element(4, Compiled)) of
        {ok, Requires} -> Requires;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_requires_test"/utf8>>,
                        line => 36,
                        value => _assert_fail@2,
                        start => 873,
                        'end' => 935,
                        pattern_start => 884,
                        pattern_end => 896})
    end,
    gleeunit@should:equal(Requires@1, [<<"shannon_equivalence"/utf8>>]),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/property_wire_test.gleam", 42).
?DOC(" Compiled module exposes invariants/0 with declared invariants.\n").
-spec compiled_module_has_invariants_test() -> nil.
compiled_module_has_invariants_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_invariants_test"/utf8>>,
                        line => 43,
                        value => _assert_fail,
                        start => 1148,
                        'end' => 1189,
                        pattern_start => 1159,
                        pattern_end => 1170})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @prop_test {
  type = a | b | c

  requires shannon_equivalence
  invariant connected
}
"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_invariants_test"/utf8>>,
                        line => 50,
                        value => _assert_fail@1,
                        start => 1349,
                        'end' => 1400,
                        pattern_start => 1360,
                        pattern_end => 1369})
    end,
    Compiled = conversation@trace:value(T@1),
    Invariants@1 = case loader_ffi:get_invariants(erlang:element(4, Compiled)) of
        {ok, Invariants} -> Invariants;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_has_invariants_test"/utf8>>,
                        line => 52,
                        value => _assert_fail@2,
                        start => 1435,
                        'end' => 1501,
                        pattern_start => 1446,
                        pattern_end => 1460})
    end,
    gleeunit@should:equal(Invariants@1, [<<"connected"/utf8>>]),
    gleam@erlang@process:send(Subject, shutdown).

-file("test/property_wire_test.gleam", 77).
?DOC(
    " Compiler actor calls @coincidence when processing grammar with requires.\n"
    " The grammar compiles successfully even with property checks running.\n"
).
-spec compiler_calls_coincidence_on_requires_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
compiler_calls_coincidence_on_requires_test() ->
    _ = coincidence_server:start(),
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiler_calls_coincidence_on_requires_test"/utf8>>,
                        line => 81,
                        value => _assert_fail,
                        start => 2539,
                        'end' => 2580,
                        pattern_start => 2550,
                        pattern_end => 2561})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @prop_test {
  type = a | b | c

  requires shannon_equivalence
  invariant connected
}
"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiler_calls_coincidence_on_requires_test"/utf8>>,
                        line => 89,
                        value => _assert_fail@1,
                        start => 2815,
                        'end' => 2866,
                        pattern_start => 2826,
                        pattern_end => 2835})
    end,
    Compiled = conversation@trace:value(T@1),
    gleeunit@should:equal(erlang:element(2, Compiled), <<"prop_test"/utf8>>),
    gleam@erlang@process:send(Subject, shutdown),
    _ = coincidence_server:stop().

-file("test/property_wire_test.gleam", 58).
?DOC(" Module without properties has empty requires/0 and invariants/0.\n").
-spec compiled_module_empty_requires_invariants_test() -> nil.
compiled_module_empty_requires_invariants_test() ->
    Started@1 = case conversation@compiler:start() of
        {ok, Started} -> Started;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_empty_requires_invariants_test"/utf8>>,
                        line => 59,
                        value => _assert_fail,
                        start => 1719,
                        'end' => 1760,
                        pattern_start => 1730,
                        pattern_end => 1741})
    end,
    Subject = erlang:element(3, Started@1),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @plain_test {
  type = x | y
}
"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_empty_requires_invariants_test"/utf8>>,
                        line => 66,
                        value => _assert_fail@1,
                        start => 1912,
                        'end' => 1963,
                        pattern_start => 1923,
                        pattern_end => 1932})
    end,
    Compiled = conversation@trace:value(T@1),
    Requires@1 = case loader_ffi:get_requires(erlang:element(4, Compiled)) of
        {ok, Requires} -> Requires;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_empty_requires_invariants_test"/utf8>>,
                        line => 68,
                        value => _assert_fail@2,
                        start => 1998,
                        'end' => 2060,
                        pattern_start => 2009,
                        pattern_end => 2021})
    end,
    Invariants@1 = case loader_ffi:get_invariants(erlang:element(4, Compiled)) of
        {ok, Invariants} -> Invariants;
        _assert_fail@3 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_wire_test"/utf8>>,
                        function => <<"compiled_module_empty_requires_invariants_test"/utf8>>,
                        line => 69,
                        value => _assert_fail@3,
                        start => 2063,
                        'end' => 2129,
                        pattern_start => 2074,
                        pattern_end => 2088})
    end,
    gleeunit@should:equal(Requires@1, []),
    gleeunit@should:equal(Invariants@1, []),
    gleam@erlang@process:send(Subject, shutdown).
