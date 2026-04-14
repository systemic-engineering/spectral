-module(supervisor_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/supervisor_test.gleam").
-export([supervision_tree_starts_test/0, garden_restarts_killed_domain_test/0, supervised_compile_and_garden_test/0, supervised_boot_from_files_test/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("test/supervisor_test.gleam", 13).
?DOC(" Full supervision tree starts: @compiler + garden.\n").
-spec supervision_tree_starts_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
supervision_tree_starts_test() ->
    Compiler_name = gleam_erlang_ffi:new_name(<<"compiler"/utf8>>),
    Garden_name = gleam_erlang_ffi:new_name(<<"garden"/utf8>>),
    case conversation@supervisor:start(Compiler_name, Garden_name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervision_tree_starts_test"/utf8>>,
                        line => 17,
                        value => _assert_fail,
                        start => 479,
                        'end' => 540,
                        pattern_start => 490,
                        pattern_end => 495})
    end,
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Subject,
        {compile_grammar,
            <<"grammar @sup_test {\n  type = a | b\n}\n"/utf8>>,
            Reply}
    ),
    T@1 = case gleam@erlang@process:'receive'(Reply, 5000) of
        {ok, {ok, T}} -> T;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervision_tree_starts_test"/utf8>>,
                        line => 29,
                        value => _assert_fail@1,
                        start => 811,
                        'end' => 862,
                        pattern_start => 822,
                        pattern_end => 831})
    end,
    Compiled = conversation@trace:value(T@1),
    gleeunit@should:equal(erlang:element(2, Compiled), <<"sup_test"/utf8>>),
    gleeunit@should:be_true(loader_ffi:is_loaded(<<"conv_sup_test"/utf8>>)),
    gleeunit@should:be_false(domain_server:is_running(<<"sup_test"/utf8>>)),
    case conversation@garden:start_domain(Garden_name, <<"sup_test"/utf8>>) of
        {ok, _} -> nil;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervision_tree_starts_test"/utf8>>,
                        line => 38,
                        value => _assert_fail@2,
                        start => 1141,
                        'end' => 1204,
                        pattern_start => 1152,
                        pattern_end => 1157})
    end,
    gleeunit@should:be_true(conversation@garden:is_running(<<"sup_test"/utf8>>)),
    _ = conversation@garden:stop_domain(<<"sup_test"/utf8>>).

-file("test/supervisor_test.gleam", 84).
?DOC(" Garden restarts killed domain — factory supervisor fault tolerance.\n").
-spec garden_restarts_killed_domain_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
garden_restarts_killed_domain_test() ->
    Compiler_name = gleam_erlang_ffi:new_name(<<"compiler"/utf8>>),
    Garden_name = gleam_erlang_ffi:new_name(<<"garden"/utf8>>),
    case conversation@supervisor:start(Compiler_name, Garden_name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"garden_restarts_killed_domain_test"/utf8>>,
                        line => 88,
                        value => _assert_fail,
                        start => 2583,
                        'end' => 2644,
                        pattern_start => 2594,
                        pattern_end => 2599})
    end,
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    Grammar = <<"grammar @resilient {
  type = a | b
}
"/utf8>>,
    case conversation@boot:boot(Subject, Garden_name, [Grammar]) of
        {ok, _} -> nil;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"garden_restarts_killed_domain_test"/utf8>>,
                        line => 95,
                        value => _assert_fail@1,
                        start => 2758,
                        'end' => 2819,
                        pattern_start => 2769,
                        pattern_end => 2774})
    end,
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"resilient"/utf8>>)
    ),
    domain_server:kill(<<"resilient"/utf8>>),
    gleam_erlang_ffi:sleep(100),
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"resilient"/utf8>>)
    ),
    _ = conversation@garden:stop_domain(<<"resilient"/utf8>>).

-file("test/supervisor_test.gleam", 141).
-spec first_domain_name(list(conversation@boot:booted_domain())) -> binary().
first_domain_name(Domains) ->
    case Domains of
        [D | _] ->
            erlang:element(2, D);

        [] ->
            <<""/utf8>>
    end.

-file("test/supervisor_test.gleam", 45).
?DOC(" Compile grammars through supervised path and start domains via garden.\n").
-spec supervised_compile_and_garden_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
supervised_compile_and_garden_test() ->
    Compiler_name = gleam_erlang_ffi:new_name(<<"compiler"/utf8>>),
    Garden_name = gleam_erlang_ffi:new_name(<<"garden"/utf8>>),
    case conversation@supervisor:start(Compiler_name, Garden_name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervised_compile_and_garden_test"/utf8>>,
                        line => 49,
                        value => _assert_fail,
                        start => 1520,
                        'end' => 1581,
                        pattern_start => 1531,
                        pattern_end => 1536})
    end,
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    Grammar = <<"grammar @garden_compile {
  type = module | function
  type module = atom
  type function = atom
  action exec {
    module: module
    function: function
    args: type
  }
}
"/utf8>>,
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [Grammar]) of
        {ok, Beams} -> Beams;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervised_compile_and_garden_test"/utf8>>,
                        line => 63,
                        value => _assert_fail@1,
                        start => 1833,
                        'end' => 1902,
                        pattern_start => 1844,
                        pattern_end => 1853})
    end,
    Domains = conversation@boot:results(Beams@1),
    gleeunit@should:be_true(
        conversation@garden:is_running(<<"garden_compile"/utf8>>)
    ),
    gleeunit@should:equal(
        begin
            _pipe = Domains,
            first_domain_name(_pipe)
        end,
        <<"garden_compile"/utf8>>
    ),
    Result@1 = case domain_server:exec(
        <<"garden_compile"/utf8>>,
        <<"erlang"/utf8>>,
        <<"abs"/utf8>>,
        [-42]
    ) of
        {ok, Result} -> Result;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervised_compile_and_garden_test"/utf8>>,
                        line => 76,
                        value => _assert_fail@2,
                        start => 2176,
                        'end' => 2257,
                        pattern_start => 2187,
                        pattern_end => 2197})
    end,
    case gleam@dynamic@decode:run(
        Result@1,
        {decoder, fun gleam@dynamic@decode:decode_int/1}
    ) of
        {ok, 42} -> nil;
        _assert_fail@3 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervised_compile_and_garden_test"/utf8>>,
                        line => 78,
                        value => _assert_fail@3,
                        start => 2260,
                        'end' => 2310,
                        pattern_start => 2271,
                        pattern_end => 2277})
    end,
    _ = conversation@garden:stop_domain(<<"garden_compile"/utf8>>).

-file("test/supervisor_test.gleam", 148).
-spec domain_names(list(conversation@boot:booted_domain())) -> list(binary()).
domain_names(Domains) ->
    case Domains of
        [] ->
            [];

        [D | Rest] ->
            [erlang:element(2, D) | domain_names(Rest)]
    end.

-file("test/supervisor_test.gleam", 155).
-spec list_contains(list(binary()), binary()) -> boolean().
list_contains(Items, Target) ->
    case Items of
        [] ->
            false;

        [X | Rest] ->
            case X =:= Target of
                true ->
                    true;

                false ->
                    list_contains(Rest, Target)
            end
    end.

-file("test/supervisor_test.gleam", 109).
?DOC(" Supervised boot from garden .conv files on disk.\n").
-spec supervised_boot_from_files_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
supervised_boot_from_files_test() ->
    Compiler_name = gleam_erlang_ffi:new_name(<<"compiler"/utf8>>),
    Garden_name = gleam_erlang_ffi:new_name(<<"garden"/utf8>>),
    case conversation@supervisor:start(Compiler_name, Garden_name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervised_boot_from_files_test"/utf8>>,
                        line => 113,
                        value => _assert_fail,
                        start => 3295,
                        'end' => 3356,
                        pattern_start => 3306,
                        pattern_end => 3311})
    end,
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    Garden_path = <<"/Users/alexwolf/dev/systemic.engineering/garden/public"/utf8>>,
    Beams@1 = case conversation@boot:boot_from_files(
        Subject,
        Garden_name,
        [<<Garden_path/binary, "/@reed/reed.conv"/utf8>>,
            <<Garden_path/binary, "/@erlang/erlang.conv"/utf8>>]
    ) of
        {ok, Beams} -> Beams;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"supervisor_test"/utf8>>,
                        function => <<"supervised_boot_from_files_test"/utf8>>,
                        line => 118,
                        value => _assert_fail@1,
                        start => 3494,
                        'end' => 3658,
                        pattern_start => 3505,
                        pattern_end => 3514})
    end,
    Domains = conversation@boot:results(Beams@1),
    gleeunit@should:be_true(conversation@garden:is_running(<<"reed"/utf8>>)),
    gleeunit@should:be_true(conversation@garden:is_running(<<"erlang"/utf8>>)),
    gleeunit@should:be_true(loader_ffi:is_loaded(<<"conv_reed"/utf8>>)),
    gleeunit@should:be_true(loader_ffi:is_loaded(<<"conv_erlang"/utf8>>)),
    Names = begin
        _pipe = Domains,
        domain_names(_pipe)
    end,
    gleeunit@should:be_true(list_contains(Names, <<"reed"/utf8>>)),
    gleeunit@should:be_true(list_contains(Names, <<"erlang"/utf8>>)),
    _ = conversation@garden:stop_domain(<<"reed"/utf8>>),
    _ = conversation@garden:stop_domain(<<"erlang"/utf8>>).
