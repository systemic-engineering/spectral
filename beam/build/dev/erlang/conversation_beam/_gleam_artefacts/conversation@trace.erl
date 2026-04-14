-module(conversation@trace).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/trace.gleam").
-export([value/1, oid/1, new/4, verify/2]).
-export_type([trace/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Trace — witnessed record.\n"
    "\n"
    " A signed, content-addressed record of an actor's action.\n"
).

-type trace(APY) :: {trace,
        conversation@oid:oid(),
        conversation@ref:scoped_oid(conversation@key:key()),
        gleam@option:option(conversation@oid:oid()),
        APY,
        bitstring(),
        integer()}.

-file("src/conversation/trace.gleam", 51).
?DOC(" Get the value from a trace.\n").
-spec value(trace(AQF)) -> AQF.
value(T) ->
    erlang:element(5, T).

-file("src/conversation/trace.gleam", 56).
?DOC(" Get the content address of a trace.\n").
-spec oid(trace(any())) -> conversation@oid:oid().
oid(T) ->
    erlang:element(2, T).

-file("src/conversation/trace.gleam", 23).
?DOC(" Create a new trace. Signs the value with the actor's keypair.\n").
-spec new(
    conversation@ref:scoped_oid(conversation@key:key()),
    conversation@key:key_pair(),
    AQA,
    gleam@option:option(conversation@oid:oid())
) -> trace(AQA).
new(Actor_oid, Kp, Value, Parent) ->
    Timestamp = crypto_ffi:system_time_ms(),
    Payload = crypto_ffi:term_to_binary({Value, Parent, Timestamp}),
    Signature = conversation@key:sign(Kp, Payload),
    Trace_oid = conversation@oid:from_bytes(
        crypto_ffi:term_to_binary({Value, Parent, Timestamp, Signature})
    ),
    {trace, Trace_oid, Actor_oid, Parent, Value, Signature, Timestamp}.

-file("src/conversation/trace.gleam", 45).
?DOC(" Verify a trace's signature against a public key.\n").
-spec verify(trace(any()), conversation@key:key()) -> boolean().
verify(T, K) ->
    Payload = crypto_ffi:term_to_binary(
        {erlang:element(5, T), erlang:element(4, T), erlang:element(7, T)}
    ),
    conversation@key:verify(K, Payload, erlang:element(6, T)).
