-module(conversation@garden).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/garden.gleam").
-export([start_domain/2, start/1, supervised/1, stop_domain/1, is_running/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Garden — factory supervisor for domain servers.\n"
    "\n"
    " The package manager for the language. When you install a grammar,\n"
    " the garden starts its domain server as a dynamic child. Domain\n"
    " servers that crash are restarted by the factory supervisor (transient).\n"
    "\n"
    " The garden is embedded in a static supervisor alongside @compiler:\n"
    "   conversation_supervisor (RestForOne)\n"
    "   ├── @compiler\n"
    "   └── garden (factory_supervisor)\n"
    "\n"
    " @compiler crash → garden + all domain servers restart (clean slate).\n"
    " Domain server crash → factory supervisor restarts that one domain.\n"
).

-file("src/conversation/garden.gleam", 41).
?DOC(" Start a domain server under the garden factory supervisor.\n").
-spec start_domain(
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary())),
    binary()
) -> {ok, gleam@otp@actor:started(binary())} |
    {error, gleam@otp@actor:start_error()}.
start_domain(Name, Domain) ->
    Sup = gleam@otp@factory_supervisor:get_by_name(Name),
    gleam@otp@factory_supervisor:start_child(Sup, Domain).

-file("src/conversation/garden.gleam", 71).
?DOC(
    " The template function for the factory supervisor.\n"
    " Takes a domain name, starts a domain_server linked to the caller.\n"
).
-spec start_domain_server(binary()) -> {ok, gleam@otp@actor:started(binary())} |
    {error, gleam@otp@actor:start_error()}.
start_domain_server(Domain) ->
    case domain_server:start_link(Domain) of
        {ok, Pid} ->
            {ok, {started, Pid, Domain}};

        {error, _} ->
            {error,
                {init_failed,
                    <<"failed to start domain server: "/utf8, Domain/binary>>}}
    end.

-file("src/conversation/garden.gleam", 61).
-spec builder(
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary()))
) -> gleam@otp@factory_supervisor:builder(binary(), binary()).
builder(Name) ->
    _pipe = gleam@otp@factory_supervisor:worker_child(fun start_domain_server/1),
    _pipe@1 = gleam@otp@factory_supervisor:named(_pipe, Name),
    gleam@otp@factory_supervisor:restart_tolerance(_pipe@1, 5, 60).

-file("src/conversation/garden.gleam", 23).
?DOC(
    " Start the garden factory supervisor directly.\n"
    " Use this for standalone testing; prefer `supervised` for production.\n"
).
-spec start(
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary()))
) -> {ok,
        gleam@otp@actor:started(gleam@otp@factory_supervisor:supervisor(binary(), binary()))} |
    {error, gleam@otp@actor:start_error()}.
start(Name) ->
    gleam@otp@factory_supervisor:start(builder(Name)).

-file("src/conversation/garden.gleam", 32).
?DOC(
    " Create a child specification for embedding the garden in a static supervisor.\n"
    " The name is used to register the factory supervisor so other processes\n"
    " can start/stop domain servers through it.\n"
).
-spec supervised(
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary()))
) -> gleam@otp@supervision:child_specification(gleam@otp@factory_supervisor:supervisor(binary(), binary())).
supervised(Name) ->
    gleam@otp@factory_supervisor:supervised(builder(Name)).

-file("src/conversation/garden.gleam", 52).
?DOC(
    " Stop a running domain server.\n"
    " Delegates directly to domain_server — the factory supervisor\n"
    " tracks the process lifecycle via the linked pid.\n"
).
-spec stop_domain(binary()) -> {ok, nil} | {error, gleam@dynamic:dynamic_()}.
stop_domain(Domain) ->
    domain_server:stop(Domain).

-file("src/conversation/garden.gleam", 57).
?DOC(" Check if a domain server is running.\n").
-spec is_running(binary()) -> boolean().
is_running(Domain) ->
    domain_server:is_running(Domain).
