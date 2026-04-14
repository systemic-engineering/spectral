-module(conversation@boot).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/boot.gleam").
-export([read_file/1, results/1, is_alive/1, imports_resolved/1, extends_resolved/1, boot/3, boot_from_files/3]).
-export_type([booted_domain/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Boot — compile grammars through a supervised @compiler.\n"
    "\n"
    " One boot path: supervised. The caller provides the compiler subject\n"
    " and garden name from the supervision tree. This module orchestrates\n"
    " compilation + domain startup through them.\n"
).

-type booted_domain() :: {booted_domain,
        binary(),
        binary(),
        list(binary()),
        list(binary())}.

-file("src/conversation/boot.gleam", 18).
?DOC(" Read a file from disk.\n").
-spec read_file(binary()) -> {ok, binary()} | {error, binary()}.
read_file(Path) ->
    file_ffi:read_file(Path).

-file("src/conversation/boot.gleam", 58).
?DOC(" Extract all BootedDomains from a list of Beams.\n").
-spec results(list(prism_beam:beam(booted_domain()))) -> list(booted_domain()).
results(Beams) ->
    gleam@list:map(Beams, fun(B) -> erlang:element(2, B) end).

-file("src/conversation/boot.gleam", 63).
?DOC(" Check if a booted domain is alive.\n").
-spec is_alive(booted_domain()) -> boolean().
is_alive(Booted) ->
    conversation@garden:is_running(erlang:element(2, Booted)) andalso loader_ffi:is_loaded(
        erlang:element(3, Booted)
    ).

-file("src/conversation/boot.gleam", 69).
?DOC(" Check if all lens imports are satisfied.\n").
-spec imports_resolved(list(booted_domain())) -> boolean().
imports_resolved(Domains) ->
    Booted_names = gleam@list:map(Domains, fun(D) -> erlang:element(2, D) end),
    gleam@list:all(
        Domains,
        fun(D@1) ->
            gleam@list:all(
                erlang:element(4, D@1),
                fun(Lens) -> gleam@list:contains(Booted_names, Lens) end
            )
        end
    ).

-file("src/conversation/boot.gleam", 77).
?DOC(" Check if all extends parents are satisfied.\n").
-spec extends_resolved(list(booted_domain())) -> boolean().
extends_resolved(Domains) ->
    Booted_names = gleam@list:map(Domains, fun(D) -> erlang:element(2, D) end),
    gleam@list:all(
        Domains,
        fun(D@1) ->
            gleam@list:all(
                erlang:element(5, D@1),
                fun(Parent) -> gleam@list:contains(Booted_names, Parent) end
            )
        end
    ).

-file("src/conversation/boot.gleam", 109).
-spec compile_one(
    gleam@erlang@process:subject(conversation@compiler:message()),
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary())),
    binary()
) -> {ok, prism_beam:beam(booted_domain())} | {error, binary()}.
compile_one(Compiler_subject, Garden_name, Source) ->
    Reply = gleam@erlang@process:new_subject(),
    gleam@erlang@process:send(
        Compiler_subject,
        {compile_grammar, Source, Reply}
    ),
    case gleam@erlang@process:'receive'(Reply, 10000) of
        {error, _} ->
            {error, <<"timeout compiling grammar"/utf8>>};

        {ok, {error, E}} ->
            {error, E};

        {ok, {ok, T}} ->
            Compiled = conversation@trace:value(T),
            Beam_module = erlang:element(4, Compiled),
            case conversation@garden:is_running(erlang:element(2, Compiled)) of
                true ->
                    nil;

                false ->
                    _ = conversation@garden:start_domain(
                        Garden_name,
                        erlang:element(2, Compiled)
                    ),
                    nil
            end,
            Lenses = case loader_ffi:get_lenses(Beam_module) of
                {ok, L} ->
                    L;

                {error, _} ->
                    []
            end,
            Extends = case loader_ffi:get_extends(Beam_module) of
                {ok, E@1} ->
                    E@1;

                {error, _} ->
                    []
            end,
            {ok,
                prism_beam:new(
                    {booted_domain,
                        erlang:element(2, Compiled),
                        Beam_module,
                        Lenses,
                        Extends}
                )}
    end.

-file("src/conversation/boot.gleam", 86).
-spec compile_loop(
    gleam@erlang@process:subject(conversation@compiler:message()),
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary())),
    list(binary()),
    list(prism_beam:beam(booted_domain()))
) -> {ok, list(prism_beam:beam(booted_domain()))} | {error, binary()}.
compile_loop(Compiler_subject, Garden_name, Remaining, Acc) ->
    case Remaining of
        [] ->
            {ok, lists:reverse(Acc)};

        [Source | Rest] ->
            case compile_one(Compiler_subject, Garden_name, Source) of
                {ok, Beam} ->
                    compile_loop(
                        Compiler_subject,
                        Garden_name,
                        Rest,
                        [Beam | Acc]
                    );

                {error, E} ->
                    {error, E}
            end
    end.

-file("src/conversation/boot.gleam", 33).
?DOC(
    " Compile grammars through a supervised @compiler and start domain\n"
    " servers through the garden factory supervisor.\n"
    " Each compiled domain is wrapped in a Beam — the compilation trace.\n"
).
-spec boot(
    gleam@erlang@process:subject(conversation@compiler:message()),
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary())),
    list(binary())
) -> {ok, list(prism_beam:beam(booted_domain()))} | {error, binary()}.
boot(Compiler_subject, Garden_name, Grammars) ->
    compile_loop(Compiler_subject, Garden_name, Grammars, []).

-file("src/conversation/boot.gleam", 152).
-spec read_all_files(list(binary()), list(binary())) -> {ok, list(binary())} |
    {error, binary()}.
read_all_files(Paths, Acc) ->
    case Paths of
        [] ->
            {ok, lists:reverse(Acc)};

        [Path | Rest] ->
            case file_ffi:read_file(Path) of
                {ok, Contents} ->
                    read_all_files(Rest, [Contents | Acc]);

                {error, E} ->
                    {error,
                        <<<<<<"reading "/utf8, Path/binary>>/binary, ": "/utf8>>/binary,
                            E/binary>>}
            end
    end.

-file("src/conversation/boot.gleam", 44).
?DOC(" Read .conv files from disk, then boot.\n").
-spec boot_from_files(
    gleam@erlang@process:subject(conversation@compiler:message()),
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary())),
    list(binary())
) -> {ok, list(prism_beam:beam(booted_domain()))} | {error, binary()}.
boot_from_files(Compiler_subject, Garden_name, Paths) ->
    case read_all_files(Paths, []) of
        {ok, Sources} ->
            boot(Compiler_subject, Garden_name, Sources);

        {error, E} ->
            {error, E}
    end.
