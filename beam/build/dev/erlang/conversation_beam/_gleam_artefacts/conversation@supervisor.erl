-module(conversation@supervisor).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/supervisor.gleam").
-export([start/2, supervised/2]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Conversation supervisor — static supervision tree.\n"
    "\n"
    " RestForOne:\n"
    "   @compiler → garden (factory_supervisor)\n"
    "\n"
    " @compiler compiles grammars → loads BEAM modules → returns traces.\n"
    " Garden starts domain servers as dynamic children.\n"
    "\n"
    " If @compiler crashes, garden and all domain servers restart (clean slate).\n"
    " If a domain server crashes, the garden factory_supervisor restarts it.\n"
).

-file("src/conversation/supervisor.gleam", 46).
-spec compiler_child(gleam@erlang@process:name(conversation@compiler:message())) -> gleam@otp@supervision:child_specification(gleam@erlang@process:subject(conversation@compiler:message())).
compiler_child(Name) ->
    gleam@otp@supervision:worker(
        fun() -> conversation@compiler:start_named(Name) end
    ).

-file("src/conversation/supervisor.gleam", 52).
-spec garden_child(
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary()))
) -> gleam@otp@supervision:child_specification(gleam@otp@factory_supervisor:supervisor(binary(), binary())).
garden_child(Name) ->
    conversation@garden:supervised(Name).

-file("src/conversation/supervisor.gleam", 22).
?DOC(
    " Start the conversation supervision tree.\n"
    " Returns the supervisor handle.\n"
).
-spec start(
    gleam@erlang@process:name(conversation@compiler:message()),
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary()))
) -> {ok, gleam@otp@actor:started(gleam@otp@static_supervisor:supervisor())} |
    {error, gleam@otp@actor:start_error()}.
start(Compiler_name, Garden_name) ->
    _pipe = gleam@otp@static_supervisor:new(rest_for_one),
    _pipe@1 = gleam@otp@static_supervisor:restart_tolerance(_pipe, 3, 60),
    _pipe@2 = gleam@otp@static_supervisor:add(
        _pipe@1,
        compiler_child(Compiler_name)
    ),
    _pipe@3 = gleam@otp@static_supervisor:add(
        _pipe@2,
        garden_child(Garden_name)
    ),
    gleam@otp@static_supervisor:start(_pipe@3).

-file("src/conversation/supervisor.gleam", 37).
?DOC(
    " Create a child specification for embedding this supervisor\n"
    " in a parent supervision tree (e.g. Reed's top-level supervisor).\n"
).
-spec supervised(
    gleam@erlang@process:name(conversation@compiler:message()),
    gleam@erlang@process:name(gleam@otp@factory_supervisor:message(binary(), binary()))
) -> gleam@otp@supervision:child_specification(gleam@otp@static_supervisor:supervisor()).
supervised(Compiler_name, Garden_name) ->
    gleam@otp@supervision:supervisor(
        fun() -> start(Compiler_name, Garden_name) end
    ).
