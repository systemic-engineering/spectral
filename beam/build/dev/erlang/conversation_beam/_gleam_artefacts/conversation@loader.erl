-module(conversation@loader).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/loader.gleam").
-export([load_etf_module/1, is_loaded/1, get_lenses/1, get_extends/1, get_requires/1, get_invariants/1, get_ensures/1, purge_module/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Loader — compiles ETF to BEAM modules and loads them.\n"
    "\n"
    " Takes the ETF bytes from the Rust NIF (Erlang Abstract Format),\n"
    " compiles them via compile:forms/1, and loads with code:load_binary/3.\n"
).

-file("src/conversation/loader.gleam", 8).
?DOC(" Load ETF bytes as a BEAM module. Returns the module name on success.\n").
-spec load_etf_module(bitstring()) -> {ok, binary()} | {error, binary()}.
load_etf_module(Etf) ->
    loader_ffi:load_etf_module(Etf).

-file("src/conversation/loader.gleam", 12).
?DOC(" Check if a module is loaded on the BEAM.\n").
-spec is_loaded(binary()) -> boolean().
is_loaded(Module) ->
    loader_ffi:is_loaded(Module).

-file("src/conversation/loader.gleam", 16).
?DOC(" Get a loaded module's Lens dependencies.\n").
-spec get_lenses(binary()) -> {ok, list(binary())} | {error, binary()}.
get_lenses(Module) ->
    loader_ffi:get_lenses(Module).

-file("src/conversation/loader.gleam", 20).
?DOC(" Get a loaded module's extends (parent domains).\n").
-spec get_extends(binary()) -> {ok, list(binary())} | {error, binary()}.
get_extends(Module) ->
    loader_ffi:get_extends(Module).

-file("src/conversation/loader.gleam", 24).
?DOC(" Get a loaded module's required properties.\n").
-spec get_requires(binary()) -> {ok, list(binary())} | {error, binary()}.
get_requires(Module) ->
    loader_ffi:get_requires(Module).

-file("src/conversation/loader.gleam", 28).
?DOC(" Get a loaded module's invariant properties.\n").
-spec get_invariants(binary()) -> {ok, list(binary())} | {error, binary()}.
get_invariants(Module) ->
    loader_ffi:get_invariants(Module).

-file("src/conversation/loader.gleam", 32).
?DOC(" Get a loaded module's ensures (postcondition) properties.\n").
-spec get_ensures(binary()) -> {ok, list(binary())} | {error, binary()}.
get_ensures(Module) ->
    loader_ffi:get_ensures(Module).

-file("src/conversation/loader.gleam", 38).
?DOC(
    " Purge and delete a loaded module from the BEAM.\n"
    " Idempotent — safe to call even if the module was never loaded.\n"
    " Use this to clean up after an enforcement failure.\n"
).
-spec purge_module(binary()) -> nil.
purge_module(Module) ->
    loader_ffi:purge_module(Module).
