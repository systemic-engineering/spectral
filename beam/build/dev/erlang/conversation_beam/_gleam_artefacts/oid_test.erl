-module(oid_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/oid_test.gleam").
-export([from_bytes_deterministic_test/0, different_input_different_oid_test/0, to_string_roundtrip_test/0, to_string_is_hex_test/0]).

-file("test/oid_test.gleam", 4).
-spec from_bytes_deterministic_test() -> nil.
from_bytes_deterministic_test() ->
    A = conversation@oid:from_bytes(<<"hello"/utf8>>),
    B = conversation@oid:from_bytes(<<"hello"/utf8>>),
    _pipe = conversation@oid:equals(A, B),
    gleeunit@should:be_true(_pipe).

-file("test/oid_test.gleam", 11).
-spec different_input_different_oid_test() -> nil.
different_input_different_oid_test() ->
    A = conversation@oid:from_bytes(<<"hello"/utf8>>),
    B = conversation@oid:from_bytes(<<"world"/utf8>>),
    _pipe = conversation@oid:equals(A, B),
    gleeunit@should:be_false(_pipe).

-file("test/oid_test.gleam", 17).
-spec to_string_roundtrip_test() -> nil.
to_string_roundtrip_test() ->
    A = conversation@oid:from_bytes(<<"test"/utf8>>),
    S = conversation@oid:to_string(A),
    B = conversation@oid:from_string(S),
    _pipe = conversation@oid:equals(A, B),
    gleeunit@should:be_true(_pipe).

-file("test/oid_test.gleam", 31).
-spec string_length(binary()) -> integer().
string_length(S) ->
    string:length(S).

-file("test/oid_test.gleam", 24).
-spec to_string_is_hex_test() -> nil.
to_string_is_hex_test() ->
    A = conversation@oid:from_bytes(<<"abc"/utf8>>),
    S = conversation@oid:to_string(A),
    gleeunit@should:equal(128, string_length(S)).
