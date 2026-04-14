-module(conversation@compiler).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/compiler.gleam").
-export([public_key_from/1, public_key/0, start/0, start_with_root/1, start_named/1, start_named_with_root/2]).
-export_type([compiled_domain/0, phase/0, message/0, state/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Compiler — @compiler actor.\n"
    "\n"
    " The @compiler receives .conv source, compiles the grammar block via\n"
    " the Rust NIF, loads the compiled module onto the BEAM, and returns a\n"
    " witnessed Trace(CompiledDomain).\n"
    "\n"
    " Identity is deterministic: sha512(\"compiler\") → Ed25519 keypair.\n"
    "\n"
    " Two start modes:\n"
    " - start()       — imperative path. Starts domain supervisor, manages\n"
    "                    domain server lifecycle on compile. Backwards compatible.\n"
    " - start_named() — supervised path. Pure compilation only. The garden\n"
    "                    factory supervisor handles domain server lifecycle.\n"
).

-type compiled_domain() :: {compiled_domain,
        binary(),
        conversation@oid:oid(),
        binary()}.

-type phase() :: {parse_phase, conversation@oid:oid()} |
    {resolve_phase, conversation@oid:oid()} |
    {compile_phase, conversation@oid:oid()}.

-type message() :: {compile_grammar,
        binary(),
        gleam@erlang@process:subject({ok,
                conversation@trace:trace(compiled_domain())} |
            {error, binary()})} |
    shutdown.

-type state() :: {state,
        conversation@key:key_pair(),
        conversation@ref:scoped_oid(conversation@key:key()),
        boolean()}.

-file("src/conversation/compiler.gleam", 65).
?DOC(" The @compiler actor's public key derived from a root key (hierarchical).\n").
-spec public_key_from(conversation@key:key()) -> conversation@key:key().
public_key_from(Root) ->
    _pipe = conversation@key:derive_child(Root, <<"compiler"/utf8>>),
    conversation@key:public_key(_pipe).

-file("src/conversation/compiler.gleam", 236).
?DOC(" SHA-512 hash, first 32 bytes — Ed25519 seed for the cairn pattern.\n").
-spec domain_seed(bitstring()) -> bitstring().
domain_seed(Name) ->
    Seed@1 = case crypto_ffi:sha512(Name) of
        <<Seed:32/binary, _/binary>> -> Seed;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"conversation/compiler"/utf8>>,
                        function => <<"domain_seed"/utf8>>,
                        line => 237,
                        value => _assert_fail,
                        start => 7940,
                        'end' => 8005,
                        pattern_start => 7951,
                        pattern_end => 7987})
    end,
    Seed@1.

-file("src/conversation/compiler.gleam", 59).
?DOC(" The @compiler actor's deterministic public key (flat derivation).\n").
-spec public_key() -> conversation@key:key().
public_key() ->
    _pipe = conversation@key:from_seed(domain_seed(<<"compiler"/utf8>>)),
    conversation@key:public_key(_pipe).

-file("src/conversation/compiler.gleam", 246).
?DOC(
    " Check required properties through @coincidence.\n"
    " Returns Error if any required property fails.\n"
).
-spec check_requires(binary(), binary()) -> {ok, nil} | {error, binary()}.
check_requires(Beam_module, Source) ->
    case loader_ffi:get_requires(Beam_module) of
        {ok, Requires} ->
            Failures = gleam@list:filter_map(
                Requires,
                fun(Name) ->
                    case conversation_nif:check_property(Source, Name) of
                        {ok, _} ->
                            {error, nil};

                        {error, Reason} ->
                            {ok,
                                <<<<<<"required property '"/utf8, Name/binary>>/binary,
                                        "' failed: "/utf8>>/binary,
                                    Reason/binary>>}
                    end
                end
            ),
            case Failures of
                [] ->
                    {ok, nil};

                [First | _] ->
                    {error, First}
            end;

        {error, _} ->
            {ok, nil}
    end.

-file("src/conversation/compiler.gleam", 271).
?DOC(
    " Check invariant properties through @coincidence.\n"
    " Returns Error if any invariant property fails.\n"
).
-spec check_invariants(binary(), binary()) -> {ok, nil} | {error, binary()}.
check_invariants(Beam_module, Source) ->
    case loader_ffi:get_invariants(Beam_module) of
        {ok, Invariants} ->
            Failures = gleam@list:filter_map(
                Invariants,
                fun(Name) ->
                    case conversation_nif:check_property(Source, Name) of
                        {ok, _} ->
                            {error, nil};

                        {error, Reason} ->
                            {ok,
                                <<<<<<"invariant '"/utf8, Name/binary>>/binary,
                                        "' failed: "/utf8>>/binary,
                                    Reason/binary>>}
                    end
                end
            ),
            case Failures of
                [] ->
                    {ok, nil};

                [First | _] ->
                    {error, First}
            end;

        {error, _} ->
            {ok, nil}
    end.

-file("src/conversation/compiler.gleam", 132).
-spec handle_message(state(), message()) -> gleam@otp@actor:next(state(), message()).
handle_message(State, Msg) ->
    case Msg of
        {compile_grammar, Source, Reply} ->
            Source_oid = conversation@oid:from_bytes(<<Source/binary>>),
            case conversation_nif:compile_grammar_traced(Source) of
                {ok, {Etf, Parse_oid_str, Resolve_oid_str, Compile_oid_str}} ->
                    Domain_name = case conversation@grammar:from_source(Source) of
                        {ok, G} ->
                            conversation@grammar:domain(G);

                        {error, _} ->
                            <<"unknown"/utf8>>
                    end,
                    case loader_ffi:load_etf_module(Etf) of
                        {ok, Module} ->
                            Domain_was_started = case erlang:element(4, State) of
                                true ->
                                    case domain_server:is_running(Domain_name) of
                                        false ->
                                            _ = conversation_sup:start_domain(
                                                Domain_name
                                            ),
                                            true;

                                        true ->
                                            false
                                    end;

                                false ->
                                    false
                            end,
                            Parse_trace = conversation@trace:new(
                                erlang:element(3, State),
                                erlang:element(2, State),
                                {parse_phase,
                                    conversation@oid:from_string(Parse_oid_str)},
                                none
                            ),
                            Resolve_trace = conversation@trace:new(
                                erlang:element(3, State),
                                erlang:element(2, State),
                                {resolve_phase,
                                    conversation@oid:from_string(
                                        Resolve_oid_str
                                    )},
                                {some, conversation@trace:oid(Parse_trace)}
                            ),
                            Compile_trace = conversation@trace:new(
                                erlang:element(3, State),
                                erlang:element(2, State),
                                {compile_phase,
                                    conversation@oid:from_string(
                                        Compile_oid_str
                                    )},
                                {some, conversation@trace:oid(Resolve_trace)}
                            ),
                            Enforcement_result = case check_requires(
                                Module,
                                Source
                            ) of
                                {error, Reason} ->
                                    {error,
                                        <<"property enforcement: "/utf8,
                                            Reason/binary>>};

                                {ok, nil} ->
                                    case check_invariants(Module, Source) of
                                        {error, Reason@1} ->
                                            {error,
                                                <<"property enforcement: "/utf8,
                                                    Reason@1/binary>>};

                                        {ok, nil} ->
                                            {ok, nil}
                                    end
                            end,
                            case Enforcement_result of
                                {error, Reason@2} ->
                                    case Domain_was_started of
                                        true ->
                                            _ = domain_server:stop(Domain_name),
                                            nil;

                                        false ->
                                            nil
                                    end,
                                    _ = loader_ffi:purge_module(Module),
                                    gleam@erlang@process:send(
                                        Reply,
                                        {error, Reason@2}
                                    );

                                {ok, nil} ->
                                    Compiled = {compiled_domain,
                                        Domain_name,
                                        Source_oid,
                                        Module},
                                    T = conversation@trace:new(
                                        erlang:element(3, State),
                                        erlang:element(2, State),
                                        Compiled,
                                        {some,
                                            conversation@trace:oid(
                                                Compile_trace
                                            )}
                                    ),
                                    gleam@erlang@process:send(Reply, {ok, T})
                            end;

                        {error, E} ->
                            gleam@erlang@process:send(Reply, {error, E})
                    end;

                {error, E@1} ->
                    gleam@erlang@process:send(Reply, {error, E@1})
            end,
            gleam@otp@actor:continue(State);

        shutdown ->
            gleam@otp@actor:stop()
    end.

-file("src/conversation/compiler.gleam", 108).
-spec do_start(conversation@key:key_pair(), boolean()) -> {ok,
        gleam@otp@actor:started(gleam@erlang@process:subject(message()))} |
    {error, gleam@otp@actor:start_error()}.
do_start(Kp, Manage_domains) ->
    Actor_oid = conversation@key:oid(conversation@key:public_key(Kp)),
    State = {state, Kp, Actor_oid, Manage_domains},
    _pipe = gleam@otp@actor:new(State),
    _pipe@1 = gleam@otp@actor:on_message(_pipe, fun handle_message/2),
    gleam@otp@actor:start(_pipe@1).

-file("src/conversation/compiler.gleam", 74).
?DOC(
    " Start the @compiler actor (imperative path, flat derivation).\n"
    " Starts the domain supervisor and manages domain server lifecycle\n"
    " on each compile. Use this for backwards compatibility with the\n"
    " existing boot path.\n"
).
-spec start() -> {ok,
        gleam@otp@actor:started(gleam@erlang@process:subject(message()))} |
    {error, gleam@otp@actor:start_error()}.
start() ->
    _ = conversation_sup:start_link(),
    Kp = conversation@key:from_seed(domain_seed(<<"compiler"/utf8>>)),
    do_start(Kp, true).

-file("src/conversation/compiler.gleam", 82).
?DOC(
    " Start the @compiler actor with hierarchical key derivation.\n"
    " Derives the compiler's identity from the root key.\n"
).
-spec start_with_root(conversation@key:key()) -> {ok,
        gleam@otp@actor:started(gleam@erlang@process:subject(message()))} |
    {error, gleam@otp@actor:start_error()}.
start_with_root(Root) ->
    _ = conversation_sup:start_link(),
    Kp = conversation@key:derive_child(Root, <<"compiler"/utf8>>),
    do_start(Kp, true).

-file("src/conversation/compiler.gleam", 120).
-spec do_start_named(
    conversation@key:key_pair(),
    gleam@erlang@process:name(message())
) -> {ok, gleam@otp@actor:started(gleam@erlang@process:subject(message()))} |
    {error, gleam@otp@actor:start_error()}.
do_start_named(Kp, Name) ->
    Actor_oid = conversation@key:oid(conversation@key:public_key(Kp)),
    State = {state, Kp, Actor_oid, false},
    _pipe = gleam@otp@actor:new(State),
    _pipe@1 = gleam@otp@actor:on_message(_pipe, fun handle_message/2),
    _pipe@2 = gleam@otp@actor:named(_pipe@1, Name),
    gleam@otp@actor:start(_pipe@2).

-file("src/conversation/compiler.gleam", 92).
?DOC(
    " Start the @compiler actor with a registered name (supervised path, flat).\n"
    " Does NOT start a domain supervisor or manage domain servers.\n"
    " Pure compilation: grammar → NIF → ETF → BEAM module → trace.\n"
    " The garden factory supervisor handles domain server lifecycle.\n"
).
-spec start_named(gleam@erlang@process:name(message())) -> {ok,
        gleam@otp@actor:started(gleam@erlang@process:subject(message()))} |
    {error, gleam@otp@actor:start_error()}.
start_named(Name) ->
    Kp = conversation@key:from_seed(domain_seed(<<"compiler"/utf8>>)),
    do_start_named(Kp, Name).

-file("src/conversation/compiler.gleam", 100).
?DOC(" Start named with hierarchical key derivation.\n").
-spec start_named_with_root(
    gleam@erlang@process:name(message()),
    conversation@key:key()
) -> {ok, gleam@otp@actor:started(gleam@erlang@process:subject(message()))} |
    {error, gleam@otp@actor:start_error()}.
start_named_with_root(Name, Root) ->
    Kp = conversation@key:derive_child(Root, <<"compiler"/utf8>>),
    do_start_named(Kp, Name).
