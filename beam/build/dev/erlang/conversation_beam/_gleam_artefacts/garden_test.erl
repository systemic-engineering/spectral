-module(garden_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/garden_test.gleam").
-export([garden_starts_domain_test/0, garden_multiple_domains_test/0, garden_restarts_on_kill_test/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("test/garden_test.gleam", 7).
?DOC(" Garden factory supervisor starts a domain server.\n").
-spec garden_starts_domain_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
garden_starts_domain_test() ->
    Name = gleam_erlang_ffi:new_name(<<"garden_test_start"/utf8>>),
    case conversation@garden:start(Name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_starts_domain_test"/utf8>>,
                        line => 9,
                        value => _assert_fail,
                        start => 250,
                        'end' => 287,
                        pattern_start => 261,
                        pattern_end => 266})
    end,
    case conversation@garden:start_domain(Name, <<"garden_alpha"/utf8>>) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_starts_domain_test"/utf8>>,
                        line => 11,
                        value => _assert_fail@1,
                        start => 291,
                        'end' => 351,
                        pattern_start => 302,
                        pattern_end => 307})
    end,
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"garden_alpha"/utf8>>)
    ),
    _ = conversation@garden:stop_domain(<<"garden_alpha"/utf8>>).

-file("test/garden_test.gleam", 18).
?DOC(" Garden starts and stops multiple domains.\n").
-spec garden_multiple_domains_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
garden_multiple_domains_test() ->
    Name = gleam_erlang_ffi:new_name(<<"garden_test_multi"/utf8>>),
    case conversation@garden:start(Name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_multiple_domains_test"/utf8>>,
                        line => 20,
                        value => _assert_fail,
                        start => 592,
                        'end' => 629,
                        pattern_start => 603,
                        pattern_end => 608})
    end,
    case conversation@garden:start_domain(Name, <<"garden_one"/utf8>>) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_multiple_domains_test"/utf8>>,
                        line => 22,
                        value => _assert_fail@1,
                        start => 633,
                        'end' => 691,
                        pattern_start => 644,
                        pattern_end => 649})
    end,
    case conversation@garden:start_domain(Name, <<"garden_two"/utf8>>) of
        {ok, _} -> nil;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_multiple_domains_test"/utf8>>,
                        line => 23,
                        value => _assert_fail@2,
                        start => 694,
                        'end' => 752,
                        pattern_start => 705,
                        pattern_end => 710})
    end,
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"garden_one"/utf8>>)
    ),
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"garden_two"/utf8>>)
    ),
    case conversation@garden:stop_domain(<<"garden_one"/utf8>>) of
        {ok, _} -> nil;
        _assert_fail@3 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_multiple_domains_test"/utf8>>,
                        line => 29,
                        value => _assert_fail@3,
                        start => 871,
                        'end' => 922,
                        pattern_start => 882,
                        pattern_end => 887})
    end,
    gleeunit@should:be_false(
        conversation@garden:is_running(<<"garden_one"/utf8>>)
    ),
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"garden_two"/utf8>>)
    ),
    _ = conversation@garden:stop_domain(<<"garden_two"/utf8>>).

-file("test/garden_test.gleam", 37).
?DOC(" Garden factory supervisor restarts crashed domain servers.\n").
-spec garden_restarts_on_kill_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
garden_restarts_on_kill_test() ->
    Name = gleam_erlang_ffi:new_name(<<"garden_test_restart"/utf8>>),
    case conversation@garden:start(Name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_restarts_on_kill_test"/utf8>>,
                        line => 39,
                        value => _assert_fail,
                        start => 1229,
                        'end' => 1266,
                        pattern_start => 1240,
                        pattern_end => 1245})
    end,
    case conversation@garden:start_domain(Name, <<"garden_phoenix"/utf8>>) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"garden_test"/utf8>>,
                        function => <<"garden_restarts_on_kill_test"/utf8>>,
                        line => 41,
                        value => _assert_fail@1,
                        start => 1270,
                        'end' => 1332,
                        pattern_start => 1281,
                        pattern_end => 1286})
    end,
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"garden_phoenix"/utf8>>)
    ),
    domain_server:kill(<<"garden_phoenix"/utf8>>),
    gleam_erlang_ffi:sleep(100),
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"garden_phoenix"/utf8>>)
    ),
    _ = conversation@garden:stop_domain(<<"garden_phoenix"/utf8>>).
