-module(conversation@ref).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/ref.gleam").
-export([non_empty/2, from_list/1, to_list/1, scope/1, oid/1, resolve/2]).
-export_type([scoped_oid/1, ref/1, non_empty/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Ref — the @ operator in Gleam.\n"
    "\n"
    " Content-addressed references with scope typing.\n"
).

-opaque scoped_oid(ANJ) :: {scoped_oid, conversation@oid:oid()} |
    {gleam_phantom, ANJ}.

-type ref(ANK) :: {at, scoped_oid(ANK)} | {inline, ANK}.

-type non_empty(ANL) :: {non_empty, ANL, list(ANL)}.

-file("src/conversation/ref.gleam", 24).
?DOC(" Construct a NonEmpty list.\n").
-spec non_empty(ANM, list(ANM)) -> non_empty(ANM).
non_empty(First, Rest) ->
    {non_empty, First, Rest}.

-file("src/conversation/ref.gleam", 29).
?DOC(" Construct a NonEmpty from a regular list. Fails if empty.\n").
-spec from_list(list(ANP)) -> {ok, non_empty(ANP)} | {error, nil}.
from_list(Items) ->
    case Items of
        [] ->
            {error, nil};

        [First | Rest] ->
            {ok, {non_empty, First, Rest}}
    end.

-file("src/conversation/ref.gleam", 37).
?DOC(" Convert a NonEmpty back to a regular list.\n").
-spec to_list(non_empty(ANU)) -> list(ANU).
to_list(Ne) ->
    [erlang:element(2, Ne) | erlang:element(3, Ne)].

-file("src/conversation/ref.gleam", 42).
?DOC(" Create a ScopedOid from an Oid.\n").
-spec scope(conversation@oid:oid()) -> scoped_oid(any()).
scope(Oid) ->
    {scoped_oid, Oid}.

-file("src/conversation/ref.gleam", 47).
?DOC(" Extract the Oid from a ScopedOid.\n").
-spec oid(scoped_oid(any())) -> conversation@oid:oid().
oid(Scoped) ->
    erlang:element(2, Scoped).

-file("src/conversation/ref.gleam", 53).
?DOC(
    " Resolve a Ref. Inline values are returned directly.\n"
    " At references resolve through the provided hash function.\n"
).
-spec resolve(ref(AOB), fun((AOB) -> conversation@oid:oid())) -> AOB.
resolve(R, _) ->
    case R of
        {inline, Value} ->
            Value;

        {at, _} ->
            erlang:error(#{gleam_error => panic,
                    message => <<"At resolution requires a lookup function — not yet implemented"/utf8>>,
                    file => <<?FILEPATH/utf8>>,
                    module => <<"conversation/ref"/utf8>>,
                    function => <<"resolve"/utf8>>,
                    line => 56})
    end.
