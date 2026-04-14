-module(boot_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/boot_test.gleam").
-export([boot_reed_from_garden_test/0, boot_populates_lenses_test/0, boot_unresolved_imports_test/0, supervisor_restarts_domain_test/0, boot_populates_extends_test/0, boot_unresolved_extends_test/0, boot_exec_reality_test/0, reed_boots_test/0, boot_multiple_grammars_test/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("test/boot_test.gleam", 27).
-spec setup() -> {gleam@erlang@process:subject(conversation@compiler:message()),
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary()))}.
setup() ->
    Compiler_name = gleam_erlang_ffi:new_name(<<"compiler"/utf8>>),
    Garden_name = gleam_erlang_ffi:new_name(<<"garden"/utf8>>),
    case conversation@supervisor:start(Compiler_name, Garden_name) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"setup"/utf8>>,
                        line => 33,
                        value => _assert_fail,
                        start => 749,
                        'end' => 810,
                        pattern_start => 760,
                        pattern_end => 765})
    end,
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    {Subject, Garden_name}.

-file("test/boot_test.gleam", 90).
?DOC(" Boot Reed from the actual garden files.\n").
-spec boot_reed_from_garden_test() -> {ok, integer()} |
    {error, list(gleam@dynamic@decode:decode_error())}.
boot_reed_from_garden_test() ->
    Garden = <<"/Users/alexwolf/dev/systemic.engineering/garden/public"/utf8>>,
    {Subject, Garden_name} = setup(),
    case conversation@boot:boot_from_files(
        Subject,
        Garden_name,
        [<<Garden/binary, "/@reed/reed.conv"/utf8>>,
            <<Garden/binary, "/@erlang/erlang.conv"/utf8>>]
    ) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_reed_from_garden_test"/utf8>>,
                        line => 94,
                        value => _assert_fail,
                        start => 2315,
                        'end' => 2472,
                        pattern_start => 2326,
                        pattern_end => 2338})
    end,
    gleeunit@should:be_true(domain_server:is_running(<<"reed"/utf8>>)),
    gleeunit@should:be_true(loader_ffi:is_loaded(<<"conv_reed"/utf8>>)),
    gleeunit@should:be_true(domain_server:is_running(<<"erlang"/utf8>>)),
    gleeunit@should:be_true(loader_ffi:is_loaded(<<"conv_erlang"/utf8>>)),
    Val@1 = case domain_server:exec(
        <<"erlang"/utf8>>,
        <<"erlang"/utf8>>,
        <<"abs"/utf8>>,
        [-99]
    ) of
        {ok, Val} -> Val;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_reed_from_garden_test"/utf8>>,
                        line => 109,
                        value => _assert_fail@1,
                        start => 2804,
                        'end' => 2874,
                        pattern_start => 2815,
                        pattern_end => 2822})
    end,
    _assert_subject = gleam@dynamic@decode:run(
        Val@1,
        {decoder, fun gleam@dynamic@decode:decode_int/1}
    ),
    case _assert_subject of
        {ok, 99} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_reed_from_garden_test"/utf8>>,
                        line => 111,
                        value => _assert_fail@2,
                        start => 2877,
                        'end' => 2924,
                        pattern_start => 2888,
                        pattern_end => 2894})
    end.

-file("test/boot_test.gleam", 115).
?DOC(" Boot populates lens dependencies from compiled modules.\n").
-spec boot_populates_lenses_test() -> nil.
boot_populates_lenses_test() ->
    Inner = <<"grammar @tools {
  type = hammer | wrench
}
"/utf8>>,
    Outer = <<"grammar @workshop {
  type = job
  action build {
    tool: type
  }
}
in @tools
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [Inner, Outer]) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_populates_lenses_test"/utf8>>,
                        line => 130,
                        value => _assert_fail,
                        start => 3228,
                        'end' => 3302,
                        pattern_start => 3239,
                        pattern_end => 3248})
    end,
    Domains = conversation@boot:results(Beams@1),
    Workshop@1 = case gleam@list:find(
        Domains,
        fun(D) -> erlang:element(2, D) =:= <<"workshop"/utf8>> end
    ) of
        {ok, Workshop} -> Workshop;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_populates_lenses_test"/utf8>>,
                        line => 135,
                        value => _assert_fail@1,
                        start => 3371,
                        'end' => 3453,
                        pattern_start => 3382,
                        pattern_end => 3394})
    end,
    gleeunit@should:equal(erlang:element(4, Workshop@1), [<<"tools"/utf8>>]),
    Tools@1 = case gleam@list:find(
        Domains,
        fun(D@1) -> erlang:element(2, D@1) =:= <<"tools"/utf8>> end
    ) of
        {ok, Tools} -> Tools;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_populates_lenses_test"/utf8>>,
                        line => 140,
                        value => _assert_fail@2,
                        start => 3526,
                        'end' => 3602,
                        pattern_start => 3537,
                        pattern_end => 3546})
    end,
    gleeunit@should:equal(erlang:element(4, Tools@1), []),
    gleeunit@should:be_true(conversation@boot:imports_resolved(Domains)).

-file("test/boot_test.gleam", 149).
?DOC(" Imports not resolved when dependency is missing.\n").
-spec boot_unresolved_imports_test() -> nil.
boot_unresolved_imports_test() ->
    Lonely = <<"grammar @lonely {
  type = echo
}
in @phantom
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [Lonely]) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_unresolved_imports_test"/utf8>>,
                        line => 157,
                        value => _assert_fail,
                        start => 3916,
                        'end' => 3980,
                        pattern_start => 3927,
                        pattern_end => 3936})
    end,
    Domains = conversation@boot:results(Beams@1),
    D@2 = case gleam@list:find(
        Domains,
        fun(D) -> erlang:element(2, D) =:= <<"lonely"/utf8>> end
    ) of
        {ok, D@1} -> D@1;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_unresolved_imports_test"/utf8>>,
                        line => 160,
                        value => _assert_fail@1,
                        start => 4020,
                        'end' => 4093,
                        pattern_start => 4031,
                        pattern_end => 4036})
    end,
    gleeunit@should:equal(erlang:element(4, D@2), [<<"phantom"/utf8>>]),
    gleeunit@should:be_false(conversation@boot:imports_resolved(Domains)).

-file("test/boot_test.gleam", 168).
?DOC(" Supervisor restarts crashed domain servers.\n").
-spec supervisor_restarts_domain_test() -> nil.
supervisor_restarts_domain_test() ->
    Grammar = <<"grammar @phoenix {
  type = flame
  action rise {
    from: type
  }
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    case conversation@boot:boot(Subject, Garden_name, [Grammar]) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"supervisor_restarts_domain_test"/utf8>>,
                        line => 178,
                        value => _assert_fail,
                        start => 4410,
                        'end' => 4476,
                        pattern_start => 4421,
                        pattern_end => 4431})
    end,
    gleeunit@should:be_true(domain_server:is_running(<<"phoenix"/utf8>>)),
    domain_server:kill(<<"phoenix"/utf8>>),
    gleam_erlang_ffi:sleep(50),
    gleeunit@should:be_true(domain_server:is_running(<<"phoenix"/utf8>>)).

-file("test/boot_test.gleam", 190).
?DOC(" Boot populates extends from compiled modules.\n").
-spec boot_populates_extends_test() -> nil.
boot_populates_extends_test() ->
    Parent = <<"grammar @smash {
  type = move | attack
}
"/utf8>>,
    Child = <<"grammar @fox extends @smash {
  type = dodge | counter
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [Parent, Child]) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_populates_extends_test"/utf8>>,
                        line => 201,
                        value => _assert_fail,
                        start => 4971,
                        'end' => 5046,
                        pattern_start => 4982,
                        pattern_end => 4991})
    end,
    Domains = conversation@boot:results(Beams@1),
    Fox@1 = case gleam@list:find(
        Domains,
        fun(D) -> erlang:element(2, D) =:= <<"fox"/utf8>> end
    ) of
        {ok, Fox} -> Fox;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_populates_extends_test"/utf8>>,
                        line => 205,
                        value => _assert_fail@1,
                        start => 5086,
                        'end' => 5158,
                        pattern_start => 5097,
                        pattern_end => 5104})
    end,
    gleeunit@should:equal(erlang:element(5, Fox@1), [<<"smash"/utf8>>]),
    Smash@1 = case gleam@list:find(
        Domains,
        fun(D@1) -> erlang:element(2, D@1) =:= <<"smash"/utf8>> end
    ) of
        {ok, Smash} -> Smash;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_populates_extends_test"/utf8>>,
                        line => 209,
                        value => _assert_fail@2,
                        start => 5201,
                        'end' => 5277,
                        pattern_start => 5212,
                        pattern_end => 5221})
    end,
    gleeunit@should:equal(erlang:element(5, Smash@1), []),
    gleeunit@should:be_true(conversation@boot:extends_resolved(Domains)).

-file("test/boot_test.gleam", 217).
?DOC(" Extends not resolved when parent is missing.\n").
-spec boot_unresolved_extends_test() -> nil.
boot_unresolved_extends_test() ->
    Orphan = <<"grammar @orphan extends @missing {
  type = lost
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [Orphan]) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_unresolved_extends_test"/utf8>>,
                        line => 224,
                        value => _assert_fail,
                        start => 5566,
                        'end' => 5630,
                        pattern_start => 5577,
                        pattern_end => 5586})
    end,
    Domains = conversation@boot:results(Beams@1),
    D@2 = case gleam@list:find(
        Domains,
        fun(D) -> erlang:element(2, D) =:= <<"orphan"/utf8>> end
    ) of
        {ok, D@1} -> D@1;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_unresolved_extends_test"/utf8>>,
                        line => 227,
                        value => _assert_fail@1,
                        start => 5670,
                        'end' => 5743,
                        pattern_start => 5681,
                        pattern_end => 5686})
    end,
    gleeunit@should:equal(erlang:element(5, D@2), [<<"missing"/utf8>>]),
    gleeunit@should:be_false(conversation@boot:extends_resolved(Domains)).

-file("test/boot_test.gleam", 235).
?DOC(" Boot then exec proves the full loop: grammar → module → server → reality.\n").
-spec boot_exec_reality_test() -> {ok, binary()} |
    {error, list(gleam@dynamic@decode:decode_error())}.
boot_exec_reality_test() ->
    Native_grammar = <<"grammar @boot_exec {
  type = module | function
  action exec {
    module: module
    function: function
    args: list
  }
}
in @reality
"/utf8>>,
    {Subject, Garden_name} = setup(),
    case conversation@boot:boot(Subject, Garden_name, [Native_grammar]) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_exec_reality_test"/utf8>>,
                        line => 249,
                        value => _assert_fail,
                        start => 6167,
                        'end' => 6246,
                        pattern_start => 6178,
                        pattern_end => 6190})
    end,
    Val@1 = case domain_server:exec(
        <<"boot_exec"/utf8>>,
        <<"erlang"/utf8>>,
        <<"integer_to_binary"/utf8>>,
        [42]
    ) of
        {ok, Val} -> Val;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_exec_reality_test"/utf8>>,
                        line => 252,
                        value => _assert_fail@1,
                        start => 6250,
                        'end' => 6336,
                        pattern_start => 6261,
                        pattern_end => 6268})
    end,
    _assert_subject = gleam@dynamic@decode:run(
        Val@1,
        {decoder, fun gleam@dynamic@decode:decode_string/1}
    ),
    case _assert_subject of
        {ok, <<"42"/utf8>>} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_exec_reality_test"/utf8>>,
                        line => 254,
                        value => _assert_fail@2,
                        start => 6339,
                        'end' => 6391,
                        pattern_start => 6350,
                        pattern_end => 6358})
    end.

-file("test/boot_test.gleam", 39).
?DOC(" Reed boots on the BEAM.\n").
-spec reed_boots_test() -> nil.
reed_boots_test() ->
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(
        Subject,
        Garden_name,
        [<<"grammar @reed {
  type = signal | memory | quote

  type signal = message | correction | insight

  type memory = session | pattern | position

  type quote = observation | crystallization
}

in @ai
in @actor
in @reality
"/utf8>>]
    ) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"reed_boots_test"/utf8>>,
                        line => 41,
                        value => _assert_fail,
                        start => 990,
                        'end' => 1060,
                        pattern_start => 1001,
                        pattern_end => 1010})
    end,
    Domains = conversation@boot:results(Beams@1),
    gleeunit@should:be_true(domain_server:is_running(<<"reed"/utf8>>)),
    gleeunit@should:be_true(loader_ffi:is_loaded(<<"conv_reed"/utf8>>)),
    Reed@1 = case gleam@list:find(
        Domains,
        fun(D) -> erlang:element(2, D) =:= <<"reed"/utf8>> end
    ) of
        {ok, Reed} -> Reed;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"reed_boots_test"/utf8>>,
                        line => 51,
                        value => _assert_fail@1,
                        start => 1273,
                        'end' => 1347,
                        pattern_start => 1284,
                        pattern_end => 1292})
    end,
    gleeunit@should:be_true(conversation@boot:is_alive(Reed@1)).

-file("test/boot_test.gleam", 57).
?DOC(" Boot multiple grammars at once.\n").
-spec boot_multiple_grammars_test() -> {ok, integer()} |
    {error, list(gleam@dynamic@decode:decode_error())}.
boot_multiple_grammars_test() ->
    Erlang_grammar = <<"grammar @native_boot {
  type = module | function
  type module = atom
  type function = atom

  action exec {
    module: module
    function: function
    args: list
  }
}

in @tools
in @reality
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(
        Subject,
        Garden_name,
        [<<"grammar @reed {
  type = signal | memory | quote

  type signal = message | correction | insight

  type memory = session | pattern | position

  type quote = observation | crystallization
}

in @ai
in @actor
in @reality
"/utf8>>,
            Erlang_grammar]
    ) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_multiple_grammars_test"/utf8>>,
                        line => 76,
                        value => _assert_fail,
                        start => 1734,
                        'end' => 1824,
                        pattern_start => 1745,
                        pattern_end => 1754})
    end,
    gleeunit@should:be_true(domain_server:is_running(<<"reed"/utf8>>)),
    gleeunit@should:be_true(domain_server:is_running(<<"native_boot"/utf8>>)),
    Val@1 = case domain_server:exec(
        <<"native_boot"/utf8>>,
        <<"erlang"/utf8>>,
        <<"abs"/utf8>>,
        [-7]
    ) of
        {ok, Val} -> Val;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_multiple_grammars_test"/utf8>>,
                        line => 84,
                        value => _assert_fail@1,
                        start => 1988,
                        'end' => 2062,
                        pattern_start => 1999,
                        pattern_end => 2006})
    end,
    _assert_subject = gleam@dynamic@decode:run(
        Val@1,
        {decoder, fun gleam@dynamic@decode:decode_int/1}
    ),
    case _assert_subject of
        {ok, 7} -> _assert_subject;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"boot_test"/utf8>>,
                        function => <<"boot_multiple_grammars_test"/utf8>>,
                        line => 86,
                        value => _assert_fail@2,
                        start => 2065,
                        'end' => 2111,
                        pattern_start => 2076,
                        pattern_end => 2081})
    end.
