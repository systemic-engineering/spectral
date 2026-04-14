-module(conversation_beam).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation_beam.gleam").
-export([start_compiler/0, main/0]).

-file("src/conversation_beam.gleam", 11).
-spec start_compiler() -> {ok,
        gleam@otp@actor:started(gleam@erlang@process:subject(conversation@compiler:message()))} |
    {error, gleam@otp@actor:start_error()}.
start_compiler() ->
    conversation@compiler:start().

-file("src/conversation_beam.gleam", 15).
-spec main() -> list(conversation@runtime:delta()).
main() ->
    Spec = {'case',
        <<"error.rate"/utf8>>,
        [{arm,
                {cmp, gt, <<"0.1"/utf8>>},
                {desired_state, <<"health_monitor"/utf8>>, <<"critical"/utf8>>}},
            {arm, wildcard, pass}]},
    Deltas = conversation@runtime:converge(Spec),
    Deltas.
