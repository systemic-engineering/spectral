-module(conversation@prism).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/prism.gleam").
-export([dimension/1, preview/2, review/2, modify/3, compose/2, new/1]).
-export_type([prism/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Prism — root optics primitive. A projection matrix.\n"
    "\n"
    " A Prism selects which variant you're working with.\n"
    " preview = project + check nonzero.\n"
    " review  = embed via transpose.\n"
    " modify  = complement + transform.\n"
    " compose = matmul.\n"
    "\n"
    " The routing IS the computation. Fortran handles the math.\n"
    "\n"
    " Build the NIF before running tests:\n"
    "   just build-prism-nif\n"
).

-opaque prism() :: {prism, integer(), list(list(float()))}.

-file("src/conversation/prism.gleam", 26).
?DOC(" The dimension of this prism's space.\n").
-spec dimension(prism()) -> integer().
dimension(P) ->
    erlang:element(2, P).

-file("src/conversation/prism.gleam", 32).
?DOC(
    " Project source into the prism's subspace.\n"
    " Returns Ok(focus) if the projection is nonzero, Error(Nil) otherwise.\n"
).
-spec preview(prism(), list(float())) -> {ok, list(float())} | {error, nil}.
preview(P, Source) ->
    case conversation_prism_nif:prism_preview(
        erlang:element(2, P),
        erlang:element(3, P),
        Source
    ) of
        {ok, Focus} ->
            {ok, Focus};

        {error, _} ->
            {error, nil}
    end.

-file("src/conversation/prism.gleam", 40).
?DOC(" Embed a focus value back into the full space via P^T.\n").
-spec review(prism(), list(float())) -> list(float()).
review(P, Focus) ->
    conversation_prism_nif:prism_review(
        erlang:element(2, P),
        erlang:element(3, P),
        Focus
    ).

-file("src/conversation/prism.gleam", 46).
?DOC(
    " Transform the matched part, leave the complement unchanged.\n"
    " result = (I - P) * source + transform * (P * source)\n"
).
-spec modify(prism(), list(float()), list(list(float()))) -> list(float()).
modify(P, Source, Transform) ->
    conversation_prism_nif:prism_modify(
        erlang:element(2, P),
        erlang:element(3, P),
        Source,
        Transform
    ).

-file("src/conversation/prism.gleam", 56).
?DOC(
    " Compose two prisms. The result selects the intersection of subspaces.\n"
    " composed = p2 * p1\n"
).
-spec compose(prism(), prism()) -> prism().
compose(P1, P2) ->
    Composed = conversation_prism_nif:prism_compose(
        erlang:element(2, P1),
        erlang:element(3, P1),
        erlang:element(3, P2)
    ),
    {prism, erlang:element(2, P1), Composed}.

-file("src/conversation/prism.gleam", 98).
-spec do_list_length(list(any()), integer()) -> integer().
do_list_length(L, Acc) ->
    case L of
        [] ->
            Acc;

        [_ | Rest] ->
            do_list_length(Rest, Acc + 1)
    end.

-file("src/conversation/prism.gleam", 94).
-spec list_length(list(any())) -> integer().
list_length(L) ->
    do_list_length(L, 0).

-file("src/conversation/prism.gleam", 20).
?DOC(" Construct a Prism from a projection matrix (list of rows).\n").
-spec new(list(list(float()))) -> prism().
new(Projection) ->
    Dimension = list_length(Projection),
    {prism, Dimension, Projection}.
