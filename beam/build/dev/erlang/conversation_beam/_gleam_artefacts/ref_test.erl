-module(ref_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/ref_test.gleam").
-export([non_empty_construction_test/0, non_empty_single_test/0, from_list_ok_test/0, from_list_empty_errors_test/0, scope_creates_scoped_oid_test/0, resolve_inline_test/0, ref_at_construction_test/0]).

-file("test/ref_test.gleam", 5).
-spec non_empty_construction_test() -> nil.
non_empty_construction_test() ->
    Ne = conversation@ref:non_empty(1, [2, 3]),
    _pipe = conversation@ref:to_list(Ne),
    gleeunit@should:equal(_pipe, [1, 2, 3]).

-file("test/ref_test.gleam", 10).
-spec non_empty_single_test() -> nil.
non_empty_single_test() ->
    Ne = conversation@ref:non_empty(<<"a"/utf8>>, []),
    _pipe = conversation@ref:to_list(Ne),
    gleeunit@should:equal(_pipe, [<<"a"/utf8>>]).

-file("test/ref_test.gleam", 15).
-spec from_list_ok_test() -> nil.
from_list_ok_test() ->
    Ne@1 = case conversation@ref:from_list([1, 2, 3]) of
        {ok, Ne} -> Ne;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"ref_test"/utf8>>,
                        function => <<"from_list_ok_test"/utf8>>,
                        line => 16,
                        value => _assert_fail,
                        start => 360,
                        'end' => 404,
                        pattern_start => 371,
                        pattern_end => 377})
    end,
    _pipe = conversation@ref:to_list(Ne@1),
    gleeunit@should:equal(_pipe, [1, 2, 3]).

-file("test/ref_test.gleam", 20).
-spec from_list_empty_errors_test() -> nil.
from_list_empty_errors_test() ->
    _pipe = conversation@ref:from_list([]),
    gleeunit@should:be_error(_pipe).

-file("test/ref_test.gleam", 24).
-spec scope_creates_scoped_oid_test() -> nil.
scope_creates_scoped_oid_test() ->
    O = conversation@oid:from_bytes(<<"test"/utf8>>),
    Scoped = conversation@ref:scope(O),
    Retrieved = conversation@ref:oid(Scoped),
    _pipe = conversation@oid:equals(O, Retrieved),
    gleeunit@should:be_true(_pipe).

-file("test/ref_test.gleam", 31).
-spec resolve_inline_test() -> nil.
resolve_inline_test() ->
    R = {inline, 42},
    Resolved = conversation@ref:resolve(
        R,
        fun(X) -> conversation@oid:from_bytes(<<X/integer>>) end
    ),
    gleeunit@should:equal(Resolved, 42).

-file("test/ref_test.gleam", 38).
-spec ref_at_construction_test() -> nil.
ref_at_construction_test() ->
    O = conversation@oid:from_bytes(<<"actor"/utf8>>),
    Scoped = conversation@ref:scope(O),
    R = {at, Scoped},
    case R of
        {at, _} ->
            gleeunit@should:be_true(true);

        {inline, _} ->
            gleeunit@should:be_true(false)
    end.
