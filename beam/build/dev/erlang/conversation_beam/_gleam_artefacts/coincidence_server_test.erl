-module(coincidence_server_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/coincidence_server_test.gleam").
-export([coincidence_server_starts_and_stops_test/0, coincidence_server_double_start_test/0, shannon_equivalence_via_server_test/0, connected_via_server_test/0, bipartite_via_server_test/0, exhaustive_via_server_test/0, check_action_dispatches_test/0, unknown_action_fallback_test/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("test/coincidence_server_test.gleam", 6).
?DOC(" Server starts, reports running, stops cleanly.\n").
-spec coincidence_server_starts_and_stops_test() -> nil.
coincidence_server_starts_and_stops_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"coincidence_server_starts_and_stops_test"/utf8>>,
                        line => 7,
                        value => _assert_fail,
                        start => 188,
                        'end' => 233,
                        pattern_start => 199,
                        pattern_end => 204})
    end,
    gleeunit@should:be_true(coincidence_server:is_running()),
    case coincidence_server:stop() of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"coincidence_server_starts_and_stops_test"/utf8>>,
                        line => 9,
                        value => _assert_fail@1,
                        start => 279,
                        'end' => 323,
                        pattern_start => 290,
                        pattern_end => 295})
    end,
    gleeunit@should:be_false(coincidence_server:is_running()).

-file("test/coincidence_server_test.gleam", 14).
?DOC(" Double-start returns error (already registered).\n").
-spec coincidence_server_double_start_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
coincidence_server_double_start_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"coincidence_server_double_start_test"/utf8>>,
                        line => 15,
                        value => _assert_fail,
                        start => 474,
                        'end' => 519,
                        pattern_start => 485,
                        pattern_end => 490})
    end,
    case coincidence_server:start() of
        {error, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"coincidence_server_double_start_test"/utf8>>,
                        line => 16,
                        value => _assert_fail@1,
                        start => 522,
                        'end' => 570,
                        pattern_start => 533,
                        pattern_end => 541})
    end,
    _assert_subject = coincidence_server:stop(),
    case _assert_subject of
        {ok, _} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"coincidence_server_double_start_test"/utf8>>,
                        line => 17,
                        value => _assert_fail@2,
                        start => 573,
                        'end' => 617,
                        pattern_start => 584,
                        pattern_end => 589})
    end.

-file("test/coincidence_server_test.gleam", 21).
?DOC(" Named action: shannon_equivalence via domain.call_action.\n").
-spec shannon_equivalence_via_server_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
shannon_equivalence_via_server_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"shannon_equivalence_via_server_test"/utf8>>,
                        line => 22,
                        value => _assert_fail,
                        start => 732,
                        'end' => 777,
                        pattern_start => 743,
                        pattern_end => 748})
    end,
    Source = <<"grammar @test {\n  type = a | b | c\n}\n"/utf8>>,
    case domain_server:call_action(
        <<"coincidence"/utf8>>,
        <<"shannon_equivalence"/utf8>>,
        [Source]
    ) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"shannon_equivalence_via_server_test"/utf8>>,
                        line => 24,
                        value => _assert_fail@1,
                        start => 838,
                        'end' => 927,
                        pattern_start => 849,
                        pattern_end => 854})
    end,
    _assert_subject = coincidence_server:stop(),
    case _assert_subject of
        {ok, _} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"shannon_equivalence_via_server_test"/utf8>>,
                        line => 26,
                        value => _assert_fail@2,
                        start => 930,
                        'end' => 974,
                        pattern_start => 941,
                        pattern_end => 946})
    end.

-file("test/coincidence_server_test.gleam", 30).
?DOC(" Named action: connected via domain.call_action.\n").
-spec connected_via_server_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
connected_via_server_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"connected_via_server_test"/utf8>>,
                        line => 31,
                        value => _assert_fail,
                        start => 1069,
                        'end' => 1114,
                        pattern_start => 1080,
                        pattern_end => 1085})
    end,
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    case domain_server:call_action(
        <<"coincidence"/utf8>>,
        <<"connected"/utf8>>,
        [Source]
    ) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"connected_via_server_test"/utf8>>,
                        line => 33,
                        value => _assert_fail@1,
                        start => 1171,
                        'end' => 1246,
                        pattern_start => 1182,
                        pattern_end => 1187})
    end,
    _assert_subject = coincidence_server:stop(),
    case _assert_subject of
        {ok, _} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"connected_via_server_test"/utf8>>,
                        line => 34,
                        value => _assert_fail@2,
                        start => 1249,
                        'end' => 1293,
                        pattern_start => 1260,
                        pattern_end => 1265})
    end.

-file("test/coincidence_server_test.gleam", 38).
?DOC(" Named action: bipartite via domain.call_action.\n").
-spec bipartite_via_server_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
bipartite_via_server_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"bipartite_via_server_test"/utf8>>,
                        line => 39,
                        value => _assert_fail,
                        start => 1388,
                        'end' => 1433,
                        pattern_start => 1399,
                        pattern_end => 1404})
    end,
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    case domain_server:call_action(
        <<"coincidence"/utf8>>,
        <<"bipartite"/utf8>>,
        [Source]
    ) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"bipartite_via_server_test"/utf8>>,
                        line => 41,
                        value => _assert_fail@1,
                        start => 1490,
                        'end' => 1565,
                        pattern_start => 1501,
                        pattern_end => 1506})
    end,
    _assert_subject = coincidence_server:stop(),
    case _assert_subject of
        {ok, _} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"bipartite_via_server_test"/utf8>>,
                        line => 42,
                        value => _assert_fail@2,
                        start => 1568,
                        'end' => 1612,
                        pattern_start => 1579,
                        pattern_end => 1584})
    end.

-file("test/coincidence_server_test.gleam", 46).
?DOC(" Named action: exhaustive via domain.call_action.\n").
-spec exhaustive_via_server_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
exhaustive_via_server_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"exhaustive_via_server_test"/utf8>>,
                        line => 47,
                        value => _assert_fail,
                        start => 1709,
                        'end' => 1754,
                        pattern_start => 1720,
                        pattern_end => 1725})
    end,
    Source = <<"grammar @test {\n  type = a | b\n}\n"/utf8>>,
    case domain_server:call_action(
        <<"coincidence"/utf8>>,
        <<"exhaustive"/utf8>>,
        [Source]
    ) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"exhaustive_via_server_test"/utf8>>,
                        line => 49,
                        value => _assert_fail@1,
                        start => 1811,
                        'end' => 1887,
                        pattern_start => 1822,
                        pattern_end => 1827})
    end,
    _assert_subject = coincidence_server:stop(),
    case _assert_subject of
        {ok, _} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"exhaustive_via_server_test"/utf8>>,
                        line => 50,
                        value => _assert_fail@2,
                        start => 1890,
                        'end' => 1934,
                        pattern_start => 1901,
                        pattern_end => 1906})
    end.

-file("test/coincidence_server_test.gleam", 54).
?DOC(" Generic check action dispatches by property name.\n").
-spec check_action_dispatches_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
check_action_dispatches_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"check_action_dispatches_test"/utf8>>,
                        line => 55,
                        value => _assert_fail,
                        start => 2034,
                        'end' => 2079,
                        pattern_start => 2045,
                        pattern_end => 2050})
    end,
    Source = <<"grammar @test {\n  type = a | b | c\n}\n"/utf8>>,
    case domain_server:call_action(
        <<"coincidence"/utf8>>,
        <<"check"/utf8>>,
        [<<"shannon_equivalence"/utf8>>, Source]
    ) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"check_action_dispatches_test"/utf8>>,
                        line => 57,
                        value => _assert_fail@1,
                        start => 2140,
                        'end' => 2251,
                        pattern_start => 2151,
                        pattern_end => 2156})
    end,
    _assert_subject = coincidence_server:stop(),
    case _assert_subject of
        {ok, _} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"check_action_dispatches_test"/utf8>>,
                        line => 61,
                        value => _assert_fail@2,
                        start => 2254,
                        'end' => 2298,
                        pattern_start => 2265,
                        pattern_end => 2270})
    end.

-file("test/coincidence_server_test.gleam", 65).
?DOC(" Fallback: unknown action returns domain echo tuple.\n").
-spec unknown_action_fallback_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
unknown_action_fallback_test() ->
    case coincidence_server:start() of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"unknown_action_fallback_test"/utf8>>,
                        line => 66,
                        value => _assert_fail,
                        start => 2400,
                        'end' => 2445,
                        pattern_start => 2411,
                        pattern_end => 2416})
    end,
    case domain_server:call_action(
        <<"coincidence"/utf8>>,
        <<"whatever"/utf8>>,
        [<<"arg1"/utf8>>]
    ) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"unknown_action_fallback_test"/utf8>>,
                        line => 67,
                        value => _assert_fail@1,
                        start => 2448,
                        'end' => 2526,
                        pattern_start => 2459,
                        pattern_end => 2464})
    end,
    _assert_subject = coincidence_server:stop(),
    case _assert_subject of
        {ok, _} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"coincidence_server_test"/utf8>>,
                        function => <<"unknown_action_fallback_test"/utf8>>,
                        line => 69,
                        value => _assert_fail@2,
                        start => 2529,
                        'end' => 2573,
                        pattern_start => 2540,
                        pattern_end => 2545})
    end.
