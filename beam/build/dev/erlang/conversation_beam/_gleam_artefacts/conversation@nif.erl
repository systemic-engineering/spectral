-module(conversation@nif).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/nif.gleam").
-export([parse_conv/1, compile_grammar/1, compile_grammar_traced/1]).
-export_type([traced_compile/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " NIF — bridge to the Rust conversation crate.\n"
    "\n"
    " Calls the Rustler NIF compiled from `beam/native/conversation_nif`.\n"
    " On load, the NIF is loaded from `priv/conversation_nif.so`.\n"
    "\n"
    " Build the NIF before running tests:\n"
    "   just build-nif\n"
).

-type traced_compile() :: {traced_compile,
        bitstring(),
        binary(),
        binary(),
        binary()}.

-file("src/conversation/nif.gleam", 13).
?DOC(
    " Parse a .conv source string.\n"
    " Returns the content OID of the parsed tree on success,\n"
    " or an error message string on failure.\n"
).
-spec parse_conv(binary()) -> {ok, binary()} | {error, binary()}.
parse_conv(Source) ->
    conversation_nif:parse_conv(Source).

-file("src/conversation/nif.gleam", 19).
?DOC(
    " Compile a .conv grammar source into ETF bytes.\n"
    " Returns ETF-encoded EAF ready for `compile:forms/1` on success,\n"
    " or an error message string on failure.\n"
).
-spec compile_grammar(binary()) -> {ok, bitstring()} | {error, binary()}.
compile_grammar(Source) ->
    conversation_nif:compile_grammar(Source).

-file("src/conversation/nif.gleam", 34).
?DOC(
    " Compile with per-phase OIDs for traced compilation chain.\n"
    " Returns ETF bytes and content OIDs for parse, resolve, and compile phases.\n"
).
-spec compile_grammar_traced(binary()) -> {ok,
        {bitstring(), binary(), binary(), binary()}} |
    {error, binary()}.
compile_grammar_traced(Source) ->
    conversation_nif:compile_grammar_traced(Source).
