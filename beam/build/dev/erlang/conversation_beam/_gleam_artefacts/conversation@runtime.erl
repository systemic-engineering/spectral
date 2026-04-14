-module(conversation@runtime).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/runtime.gleam").
-export([converge/1]).
-export_type([delta/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Runtime — convergence engine.\n"
    "\n"
    " Evaluates a conversation spec against current BEAM state.\n"
    " Returns the delta: what needs to change to reach desired state.\n"
).

-type delta() :: {start_process, binary(), binary()} |
    {update_state, binary(), binary(), binary()} |
    {stop_process, binary()}.

-file("src/conversation/runtime.gleam", 57).
?DOC(
    " Check if a pattern matches. Stub: wildcards always match,\n"
    " comparisons require runtime context (not yet wired).\n"
).
-spec matches(conversation@protocol:pattern()) -> boolean().
matches(Pattern) ->
    case Pattern of
        wildcard ->
            true;

        {cmp, _, _} ->
            false
    end.

-file("src/conversation/runtime.gleam", 33).
?DOC(" Try each arm in order. First match wins.\n").
-spec dispatch(list(conversation@protocol:arm())) -> list(delta()).
dispatch(Arms) ->
    case Arms of
        [] ->
            [];

        [{arm, Pattern, Body} | Rest] ->
            case matches(Pattern) of
                true ->
                    converge(Body);

                false ->
                    dispatch(Rest)
            end
    end.

-file("src/conversation/runtime.gleam", 22).
?DOC(" Evaluate a conversation spec. Returns the list of deltas needed.\n").
-spec converge(conversation@protocol:spec()) -> list(delta()).
converge(Spec) ->
    case Spec of
        {'case', _, Arms} ->
            dispatch(Arms);

        {branch, Arms@1} ->
            branch_dispatch(Arms@1);

        {'when', Op, _, Literal, Then} ->
            guard(Op, Literal, Then);

        {desired_state, Process, State} ->
            [{start_process, Process, State}];

        pass ->
            []
    end.

-file("src/conversation/runtime.gleam", 45).
?DOC(" Try all arms. Collect deltas from every arm that matches.\n").
-spec branch_dispatch(list(conversation@protocol:arm())) -> list(delta()).
branch_dispatch(Arms) ->
    gleam@list:flat_map(
        Arms,
        fun(Arm) ->
            {arm, Pattern, Body} = Arm,
            case matches(Pattern) of
                true ->
                    converge(Body);

                false ->
                    []
            end
        end
    ).

-file("src/conversation/runtime.gleam", 65).
?DOC(" Evaluate a guard. Stub: always applies (predicate eval not yet wired).\n").
-spec guard(conversation@protocol:op(), binary(), conversation@protocol:spec()) -> list(delta()).
guard(_, _, Then) ->
    converge(Then).
