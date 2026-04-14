-module(conversation@coincidence).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/coincidence.gleam").
-export([check_property/2, check_shannon_equivalence/1, check_connected/1, check_bipartite/1, check_exhaustive/1, start_server/0, stop_server/0, is_running/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Coincidence — NIF bridge to measurement functions.\n"
    "\n"
    " Exposes property-checking NIFs from the conversation crate.\n"
    " Each function takes a grammar source string, compiles it,\n"
    " and checks a specific property.\n"
    "\n"
    " Also provides start/stop/is_running for the @coincidence domain server,\n"
    " which routes action calls to these NIFs.\n"
).

-file("src/conversation/coincidence.gleam", 15).
?DOC(
    " Check a built-in property by name against a grammar source.\n"
    " Returns the pass/fail reason string.\n"
).
-spec check_property(binary(), binary()) -> {ok, binary()} | {error, binary()}.
check_property(Source, Property) ->
    conversation_nif:check_property(Source, Property).

-file("src/conversation/coincidence.gleam", 23).
?DOC(
    " Check shannon equivalence (content address uniqueness).\n"
    " Every derivation of the grammar must produce a unique content OID.\n"
).
-spec check_shannon_equivalence(binary()) -> {ok, binary()} | {error, binary()}.
check_shannon_equivalence(Source) ->
    conversation_nif:check_shannon_equivalence(Source).

-file("src/conversation/coincidence.gleam", 28).
?DOC(
    " Check type graph connectivity (spectral).\n"
    " The type reference graph must be a single connected component.\n"
).
-spec check_connected(binary()) -> {ok, binary()} | {error, binary()}.
check_connected(Source) ->
    conversation_nif:check_connected(Source).

-file("src/conversation/coincidence.gleam", 33).
?DOC(
    " Check type graph bipartiteness (spectral).\n"
    " The type reference graph must have no odd-length cycles.\n"
).
-spec check_bipartite(binary()) -> {ok, binary()} | {error, binary()}.
check_bipartite(Source) ->
    conversation_nif:check_bipartite(Source).

-file("src/conversation/coincidence.gleam", 37).
?DOC(" Check exhaustiveness — every declared type has at least one variant.\n").
-spec check_exhaustive(binary()) -> {ok, binary()} | {error, binary()}.
check_exhaustive(Source) ->
    conversation_nif:check_exhaustive(Source).

-file("src/conversation/coincidence.gleam", 43).
?DOC(" Start the @coincidence domain server (unsupervised).\n").
-spec start_server() -> {ok, nil} | {error, gleam@dynamic:dynamic_()}.
start_server() ->
    coincidence_server:start().

-file("src/conversation/coincidence.gleam", 47).
?DOC(" Stop the @coincidence domain server.\n").
-spec stop_server() -> {ok, nil} | {error, gleam@dynamic:dynamic_()}.
stop_server() ->
    coincidence_server:stop().

-file("src/conversation/coincidence.gleam", 51).
?DOC(" Check if the @coincidence domain server is running.\n").
-spec is_running() -> boolean().
is_running() ->
    coincidence_server:is_running().
