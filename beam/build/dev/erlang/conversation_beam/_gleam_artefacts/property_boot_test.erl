-module(property_boot_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/property_boot_test.gleam").
-export([inline_grammar_with_requires_test/0, inline_grammar_with_requires_and_invariant_test/0, full_property_pipeline_test/0, enforcement_unknown_requires_fails_test/0, enforcement_valid_requires_passes_test/0, enforcement_unknown_invariant_fails_test/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

-file("test/property_boot_test.gleam", 13).
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
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"setup"/utf8>>,
                        line => 19,
                        value => _assert_fail,
                        start => 527,
                        'end' => 588,
                        pattern_start => 538,
                        pattern_end => 543})
    end,
    _ = coincidence_server:start(),
    Subject = gleam@erlang@process:named_subject(Compiler_name),
    {Subject, Garden_name}.

-file("test/property_boot_test.gleam", 26).
?DOC(" Inline grammar with requires — proves the full pipeline without garden files.\n").
-spec inline_grammar_with_requires_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
inline_grammar_with_requires_test() ->
    Source = <<"grammar @inline_prop {
  type = a | b | c

  requires shannon_equivalence
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [Source]) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"inline_grammar_with_requires_test"/utf8>>,
                        line => 36,
                        value => _assert_fail,
                        start => 978,
                        'end' => 1042,
                        pattern_start => 989,
                        pattern_end => 998})
    end,
    Domains = conversation@boot:results(Beams@1),
    gleeunit@should:be_true(domain_server:is_running(<<"inline_prop"/utf8>>)),
    gleeunit@should:be_true(loader_ffi:is_loaded(<<"conv_inline_prop"/utf8>>)),
    Requires@1 = case loader_ffi:get_requires(<<"conv_inline_prop"/utf8>>) of
        {ok, Requires} -> Requires;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"inline_grammar_with_requires_test"/utf8>>,
                        line => 42,
                        value => _assert_fail@1,
                        start => 1189,
                        'end' => 1254,
                        pattern_start => 1200,
                        pattern_end => 1212})
    end,
    gleeunit@should:equal(Requires@1, [<<"shannon_equivalence"/utf8>>]),
    _ = coincidence_server:stop().

-file("test/property_boot_test.gleam", 49).
?DOC(" Inline grammar with both requires and invariant.\n").
-spec inline_grammar_with_requires_and_invariant_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
inline_grammar_with_requires_and_invariant_test() ->
    Source = <<"grammar @dual_prop {
  type = x | y | z

  requires shannon_equivalence
  invariant connected
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Beams@1 = case conversation@boot:boot(Subject, Garden_name, [Source]) of
        {ok, Beams} -> Beams;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"inline_grammar_with_requires_and_invariant_test"/utf8>>,
                        line => 60,
                        value => _assert_fail,
                        start => 1618,
                        'end' => 1682,
                        pattern_start => 1629,
                        pattern_end => 1638})
    end,
    Domains = conversation@boot:results(Beams@1),
    gleeunit@should:be_true(domain_server:is_running(<<"dual_prop"/utf8>>)),
    Requires@1 = case loader_ffi:get_requires(<<"conv_dual_prop"/utf8>>) of
        {ok, Requires} -> Requires;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"inline_grammar_with_requires_and_invariant_test"/utf8>>,
                        line => 65,
                        value => _assert_fail@1,
                        start => 1772,
                        'end' => 1835,
                        pattern_start => 1783,
                        pattern_end => 1795})
    end,
    Invariants@1 = case loader_ffi:get_invariants(<<"conv_dual_prop"/utf8>>) of
        {ok, Invariants} -> Invariants;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"inline_grammar_with_requires_and_invariant_test"/utf8>>,
                        line => 66,
                        value => _assert_fail@2,
                        start => 1838,
                        'end' => 1905,
                        pattern_start => 1849,
                        pattern_end => 1863})
    end,
    gleeunit@should:equal(Requires@1, [<<"shannon_equivalence"/utf8>>]),
    gleeunit@should:equal(Invariants@1, [<<"connected"/utf8>>]),
    _ = coincidence_server:stop().

-file("test/property_boot_test.gleam", 75).
?DOC(
    " Full pipeline: boot infrastructure domains, then @training with\n"
    " requires/invariant. Properties enforced through @coincidence.\n"
).
-spec full_property_pipeline_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
full_property_pipeline_test() ->
    Conv = <<"/Users/alexwolf/dev/projects/conversation/conv"/utf8>>,
    Garden = <<"/Users/alexwolf/dev/systemic.engineering/garden/public"/utf8>>,
    Property_source@1 = case file_ffi:read_file(
        <<Conv/binary, "/property.conv"/utf8>>
    ) of
        {ok, Property_source} -> Property_source;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"full_property_pipeline_test"/utf8>>,
                        line => 79,
                        value => _assert_fail,
                        start => 2351,
                        'end' => 2428,
                        pattern_start => 2362,
                        pattern_end => 2381})
    end,
    Topology_source@1 = case file_ffi:read_file(
        <<Conv/binary, "/topology.conv"/utf8>>
    ) of
        {ok, Topology_source} -> Topology_source;
        _assert_fail@1 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"full_property_pipeline_test"/utf8>>,
                        line => 81,
                        value => _assert_fail@1,
                        start => 2431,
                        'end' => 2508,
                        pattern_start => 2442,
                        pattern_end => 2461})
    end,
    Training_source@1 = case file_ffi:read_file(
        <<Garden/binary, "/@training/training.conv"/utf8>>
    ) of
        {ok, Training_source} -> Training_source;
        _assert_fail@2 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"full_property_pipeline_test"/utf8>>,
                        line => 83,
                        value => _assert_fail@2,
                        start => 2511,
                        'end' => 2600,
                        pattern_start => 2522,
                        pattern_end => 2541})
    end,
    {Subject, Garden_name} = setup(),
    case conversation@boot:boot(
        Subject,
        Garden_name,
        [Property_source@1, Topology_source@1]
    ) of
        {ok, _} -> nil;
        _assert_fail@3 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"full_property_pipeline_test"/utf8>>,
                        line => 89,
                        value => _assert_fail@3,
                        start => 2694,
                        'end' => 2789,
                        pattern_start => 2705,
                        pattern_end => 2715})
    end,
    Result = conversation@boot:boot(Subject, Garden_name, [Training_source@1]),
    gleeunit@should:be_error(Result),
    Reason@1 = case Result of
        {error, Reason} -> Reason;
        _assert_fail@4 ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"full_property_pipeline_test"/utf8>>,
                        line => 95,
                        value => _assert_fail@4,
                        start => 2963,
                        'end' => 2996,
                        pattern_start => 2974,
                        pattern_end => 2987})
    end,
    gleeunit@should:be_true(
        gleam_stdlib:contains_string(Reason@1, <<"property enforcement"/utf8>>)
    ),
    gleeunit@should:be_true(
        gleam_stdlib:contains_string(Reason@1, <<"connected"/utf8>>)
    ),
    gleeunit@should:be_true(domain_server:is_running(<<"property"/utf8>>)),
    gleeunit@should:be_true(domain_server:is_running(<<"topology"/utf8>>)),
    _ = coincidence_server:stop().

-file("test/property_boot_test.gleam", 107).
?DOC(" Compilation fails when a required property is unknown.\n").
-spec enforcement_unknown_requires_fails_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
enforcement_unknown_requires_fails_test() ->
    Source = <<"grammar @bad_req {
  type = a | b

  requires nonexistent_property
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Result = conversation@boot:boot(Subject, Garden_name, [Source]),
    gleeunit@should:be_error(Result),
    _ = coincidence_server:stop().

-file("test/property_boot_test.gleam", 122).
?DOC(" Compilation succeeds when all required properties pass.\n").
-spec enforcement_valid_requires_passes_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
enforcement_valid_requires_passes_test() ->
    Source = <<"grammar @good_req {
  type = a | b | c

  requires shannon_equivalence
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    case conversation@boot:boot(Subject, Garden_name, [Source]) of
        {ok, _} -> nil;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"property_boot_test"/utf8>>,
                        function => <<"enforcement_valid_requires_passes_test"/utf8>>,
                        line => 131,
                        value => _assert_fail,
                        start => 3921,
                        'end' => 3988,
                        pattern_start => 3932,
                        pattern_end => 3944})
    end,
    _ = coincidence_server:stop().

-file("test/property_boot_test.gleam", 136).
?DOC(" Compilation fails when an invariant property is unknown.\n").
-spec enforcement_unknown_invariant_fails_test() -> {ok, nil} |
    {error, gleam@dynamic:dynamic_()}.
enforcement_unknown_invariant_fails_test() ->
    Source = <<"grammar @bad_inv {
  type = a | b

  invariant nonexistent_invariant
}
"/utf8>>,
    {Subject, Garden_name} = setup(),
    Result = conversation@boot:boot(Subject, Garden_name, [Source]),
    gleeunit@should:be_error(Result),
    _ = coincidence_server:stop().
