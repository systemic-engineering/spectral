-module(trace_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/trace_test.gleam").
-export([new_trace_test/0, trace_verify_test/0, trace_verify_wrong_key_fails_test/0, trace_oid_deterministic_test/0, trace_with_parent_test/0, different_values_different_oids_test/0]).

-file("test/trace_test.gleam", 7).
-spec new_trace_test() -> nil.
new_trace_test() ->
    Kp = conversation@key:generate(),
    Actor_oid = conversation@key:oid(conversation@key:public_key(Kp)),
    T = conversation@trace:new(Actor_oid, Kp, <<"hello"/utf8>>, none),
    gleeunit@should:equal(conversation@trace:value(T), <<"hello"/utf8>>).

-file("test/trace_test.gleam", 15).
-spec trace_verify_test() -> nil.
trace_verify_test() ->
    Kp = conversation@key:generate(),
    Pub_key = conversation@key:public_key(Kp),
    Actor_oid = conversation@key:oid(Pub_key),
    T = conversation@trace:new(Actor_oid, Kp, <<"signed message"/utf8>>, none),
    _pipe = conversation@trace:verify(T, Pub_key),
    gleeunit@should:be_true(_pipe).

-file("test/trace_test.gleam", 23).
-spec trace_verify_wrong_key_fails_test() -> nil.
trace_verify_wrong_key_fails_test() ->
    Kp1 = conversation@key:generate(),
    Kp2 = conversation@key:generate(),
    Actor_oid = conversation@key:oid(conversation@key:public_key(Kp1)),
    T = conversation@trace:new(Actor_oid, Kp1, <<"message"/utf8>>, none),
    _pipe = conversation@trace:verify(T, conversation@key:public_key(Kp2)),
    gleeunit@should:be_false(_pipe).

-file("test/trace_test.gleam", 32).
-spec trace_oid_deterministic_test() -> nil.
trace_oid_deterministic_test() ->
    Kp = conversation@key:generate(),
    Actor_oid = conversation@key:oid(conversation@key:public_key(Kp)),
    T = conversation@trace:new(Actor_oid, Kp, <<"data"/utf8>>, none),
    Oid1 = conversation@trace:oid(T),
    Oid2 = conversation@trace:oid(T),
    _pipe = conversation@oid:equals(Oid1, Oid2),
    gleeunit@should:be_true(_pipe).

-file("test/trace_test.gleam", 41).
-spec trace_with_parent_test() -> nil.
trace_with_parent_test() ->
    Kp = conversation@key:generate(),
    Actor_oid = conversation@key:oid(conversation@key:public_key(Kp)),
    Parent = conversation@oid:from_bytes(<<"parent"/utf8>>),
    T = conversation@trace:new(Actor_oid, Kp, <<"child"/utf8>>, {some, Parent}),
    gleeunit@should:equal(conversation@trace:value(T), <<"child"/utf8>>),
    _pipe = conversation@trace:verify(T, conversation@key:public_key(Kp)),
    gleeunit@should:be_true(_pipe).

-file("test/trace_test.gleam", 50).
-spec different_values_different_oids_test() -> nil.
different_values_different_oids_test() ->
    Kp = conversation@key:generate(),
    Actor_oid = conversation@key:oid(conversation@key:public_key(Kp)),
    T1 = conversation@trace:new(Actor_oid, Kp, <<"value1"/utf8>>, none),
    T2 = conversation@trace:new(Actor_oid, Kp, <<"value2"/utf8>>, none),
    _pipe = conversation@oid:equals(
        conversation@trace:oid(T1),
        conversation@trace:oid(T2)
    ),
    gleeunit@should:be_false(_pipe).
