-module(conversation@domain).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/domain.gleam").
-export([start/1, start_supervisor/0, start_supervised/1, stop/1, is_running/1, kill/1, call_action/3, exec/4]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Domain server — GenServer for compiled grammar modules.\n"
    "\n"
    " After @conversation compiles a grammar and loads the module,\n"
    " this starts a gen_server registered as the domain atom.\n"
    " When the compiled module's action functions call\n"
    " gen_server:call(Domain, {Action, Args}), this server receives them.\n"
    "\n"
    " Identity follows the cairn pattern: sha512(domain) → Ed25519 keypair.\n"
).

-file("src/conversation/domain.gleam", 15).
?DOC(
    " Start a domain GenServer registered as the domain atom (unsupervised).\n"
    " Returns Ok(Nil) on success, Error with reason on failure.\n"
).
-spec start(binary()) -> {ok, nil} | {error, gleam@dynamic:dynamic_()}.
start(Domain) ->
    domain_server:start(Domain).

-file("src/conversation/domain.gleam", 19).
?DOC(" Start the domain supervisor. Call once at boot.\n").
-spec start_supervisor() -> {ok, gleam@dynamic:dynamic_()} |
    {error, gleam@dynamic:dynamic_()}.
start_supervisor() ->
    conversation_sup:start_link().

-file("src/conversation/domain.gleam", 23).
?DOC(" Start a supervised domain server. Restarts on crash.\n").
-spec start_supervised(binary()) -> {ok, gleam@dynamic:dynamic_()} |
    {error, gleam@dynamic:dynamic_()}.
start_supervised(Domain) ->
    conversation_sup:start_domain(Domain).

-file("src/conversation/domain.gleam", 27).
?DOC(" Stop a running domain server.\n").
-spec stop(binary()) -> {ok, nil} | {error, gleam@dynamic:dynamic_()}.
stop(Domain) ->
    domain_server:stop(Domain).

-file("src/conversation/domain.gleam", 31).
?DOC(" Check if a domain server is running.\n").
-spec is_running(binary()) -> boolean().
is_running(Domain) ->
    domain_server:is_running(Domain).

-file("src/conversation/domain.gleam", 35).
?DOC(" Kill a domain server process (for testing supervisor restart).\n").
-spec kill(binary()) -> nil.
kill(Domain) ->
    domain_server:kill(Domain).

-file("src/conversation/domain.gleam", 40).
?DOC(
    " Call an action on a domain server directly.\n"
    " Args is any Gleam value — passed through to Erlang as-is.\n"
).
-spec call_action(binary(), binary(), any()) -> {ok, gleam@dynamic:dynamic_()} |
    {error, binary()}.
call_action(Domain, Action, Args) ->
    domain_server:call_action(Domain, Action, Args).

-file("src/conversation/domain.gleam", 50).
?DOC(
    " exec — the native primitive. Calls Module:Function(Args) on the BEAM.\n"
    " Module and Function are strings, converted to atoms by the server.\n"
    " This is what @erlang's domain server does: apply/3.\n"
).
-spec exec(binary(), binary(), binary(), list(any())) -> {ok,
        gleam@dynamic:dynamic_()} |
    {error, binary()}.
exec(Domain, Module, Function, Args) ->
    domain_server:exec(Domain, Module, Function, Args).
