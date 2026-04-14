-module(stage@dispatcher).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/dispatcher.gleam").
-export([new_state/0, register/2, unregister/2, add_demand/3, total_demand/1, dispatch_demand/2, dispatch_broadcast/2, dispatch_partition/4]).
-export_type([dispatcher_type/1, dispatcher_state/1, dispatch_result/2]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Dispatcher strategies for routing events from producers to consumers.\n"
    "\n"
    " Dispatchers decide how events are distributed among subscribed consumers.\n"
    " Pure functions, no processes.\n"
).

-type dispatcher_type(GCJ) :: demand_dispatcher |
    broadcast_dispatcher |
    {partition_dispatcher, fun((GCJ) -> integer()), integer()}.

-type dispatcher_state(GCK) :: {dispatcher_state,
        gleam@dict:dict(GCK, integer())}.

-type dispatch_result(GCL, GCM) :: {dispatch_result,
        list({GCL, list(GCM)}),
        list(GCM),
        dispatcher_state(GCL)}.

-file("src/stage/dispatcher.gleam", 41).
?DOC(" Create a new empty dispatcher state.\n").
-spec new_state() -> dispatcher_state(any()).
new_state() ->
    {dispatcher_state, maps:new()}.

-file("src/stage/dispatcher.gleam", 46).
?DOC(" Register a subscriber with initial demand of 0.\n").
-spec register(dispatcher_state(GCP), GCP) -> dispatcher_state(GCP).
register(State, Subscriber) ->
    {dispatcher_state,
        gleam@dict:insert(erlang:element(2, State), Subscriber, 0)}.

-file("src/stage/dispatcher.gleam", 56).
?DOC(" Remove a subscriber.\n").
-spec unregister(dispatcher_state(GCS), GCS) -> dispatcher_state(GCS).
unregister(State, Subscriber) ->
    {dispatcher_state, gleam@dict:delete(erlang:element(2, State), Subscriber)}.

-file("src/stage/dispatcher.gleam", 64).
?DOC(" Add demand for a subscriber.\n").
-spec add_demand(dispatcher_state(GCV), GCV, integer()) -> dispatcher_state(GCV).
add_demand(State, Subscriber, Count) ->
    Current = case gleam_stdlib:map_get(erlang:element(2, State), Subscriber) of
        {ok, N} ->
            N;

        {error, _} ->
            0
    end,
    {dispatcher_state,
        gleam@dict:insert(erlang:element(2, State), Subscriber, Current + Count)}.

-file("src/stage/dispatcher.gleam", 79).
?DOC(" Get total demand for demand dispatcher (sum of all demands).\n").
-spec total_demand(dispatcher_state(any())) -> integer().
total_demand(State) ->
    _pipe = maps:values(erlang:element(2, State)),
    gleam@list:fold(_pipe, 0, fun(Acc, D) -> Acc + D end).

-file("src/stage/dispatcher.gleam", 86).
?DOC(
    " Get demand available for broadcast dispatcher (min across all subscribers).\n"
    " Returns 0 if no subscribers.\n"
).
-spec broadcast_demand(dispatcher_state(any())) -> integer().
broadcast_demand(State) ->
    case maps:values(erlang:element(2, State)) of
        [] ->
            0;

        Values ->
            gleam@list:fold(
                Values,
                999999999,
                fun(Acc, D) -> gleam@int:min(Acc, D) end
            )
    end.

-file("src/stage/dispatcher.gleam", 139).
?DOC(" Merge assignments that target the same key, preserving event order.\n").
-spec merge_assignments(list({GDQ, list(GDR)})) -> list({GDQ, list(GDR)}).
merge_assignments(Assignments) ->
    Ordered = lists:reverse(Assignments),
    Merged = gleam@list:fold(
        Ordered,
        maps:new(),
        fun(Acc, Pair) ->
            {K, Evts} = Pair,
            case gleam_stdlib:map_get(Acc, K) of
                {ok, Existing} ->
                    gleam@dict:insert(Acc, K, lists:append(Existing, Evts));

                {error, _} ->
                    gleam@dict:insert(Acc, K, Evts)
            end
        end
    ),
    maps:to_list(Merged).

-file("src/stage/dispatcher.gleam", 157).
?DOC(" Find the subscriber with the maximum outstanding demand.\n").
-spec find_max_demand(dispatcher_state(GDW)) -> {ok, {GDW, integer()}} |
    {error, nil}.
find_max_demand(State) ->
    Pairs = maps:to_list(erlang:element(2, State)),
    case Pairs of
        [] ->
            {error, nil};

        _ ->
            With_demand = gleam@list:filter(
                Pairs,
                fun(P) -> erlang:element(2, P) > 0 end
            ),
            case With_demand of
                [] ->
                    {error, nil};

                [First | Rest] ->
                    Max = gleam@list:fold(
                        Rest,
                        First,
                        fun(Best, Pair) ->
                            case erlang:element(2, Pair) > erlang:element(
                                2,
                                Best
                            ) of
                                true ->
                                    Pair;

                                false ->
                                    Best
                            end
                        end
                    ),
                    {ok, Max}
            end
    end.

-file("src/stage/dispatcher.gleam", 103).
-spec do_dispatch_demand(
    dispatcher_state(GDI),
    list(GDK),
    list({GDI, list(GDK)})
) -> dispatch_result(GDI, GDK).
do_dispatch_demand(State, Events, Acc) ->
    case Events of
        [] ->
            {dispatch_result, merge_assignments(Acc), [], State};

        _ ->
            case find_max_demand(State) of
                {error, _} ->
                    {dispatch_result, merge_assignments(Acc), Events, State};

                {ok, {Subscriber, Demand}} ->
                    To_send = gleam@list:take(Events, Demand),
                    Remaining = gleam@list:drop(Events, Demand),
                    Sent_count = erlang:length(To_send),
                    New_demand = Demand - Sent_count,
                    New_state = {dispatcher_state,
                        gleam@dict:insert(
                            erlang:element(2, State),
                            Subscriber,
                            New_demand
                        )},
                    New_acc = [{Subscriber, To_send} | Acc],
                    do_dispatch_demand(New_state, Remaining, New_acc)
            end
    end.

-file("src/stage/dispatcher.gleam", 96).
?DOC(
    " Dispatch events using the DemandDispatcher strategy.\n"
    " Sends events to the subscriber with the most outstanding demand.\n"
).
-spec dispatch_demand(dispatcher_state(GDC), list(GDE)) -> dispatch_result(GDC, GDE).
dispatch_demand(State, Events) ->
    do_dispatch_demand(State, Events, []).

-file("src/stage/dispatcher.gleam", 185).
?DOC(
    " Dispatch events using the BroadcastDispatcher strategy.\n"
    " Sends all events to all subscribers. Only dispatches up to the\n"
    " minimum demand across all subscribers.\n"
).
-spec dispatch_broadcast(dispatcher_state(GEA), list(GEC)) -> dispatch_result(GEA, GEC).
dispatch_broadcast(State, Events) ->
    Min_d = broadcast_demand(State),
    case Min_d of
        0 ->
            {dispatch_result, [], Events, State};

        _ ->
            To_send = gleam@list:take(Events, Min_d),
            Remaining = gleam@list:drop(Events, Min_d),
            Sent_count = erlang:length(To_send),
            Subscribers = maps:keys(erlang:element(2, State)),
            Assignments = gleam@list:map(
                Subscribers,
                fun(Sub) -> {Sub, To_send} end
            ),
            New_demands = gleam@dict:map_values(
                erlang:element(2, State),
                fun(_, Demand) -> gleam@int:max(0, Demand - Sent_count) end
            ),
            {dispatch_result,
                Assignments,
                Remaining,
                {dispatcher_state, New_demands}}
    end.

-file("src/stage/dispatcher.gleam", 218).
?DOC(
    " Dispatch events using the PartitionDispatcher strategy.\n"
    " Routes each event to the consumer assigned to its partition.\n"
).
-spec dispatch_partition(
    dispatcher_state(integer()),
    list(GEH),
    fun((GEH) -> integer()),
    integer()
) -> dispatch_result(integer(), GEH).
dispatch_partition(State, Events, Partition_fn, Partitions) ->
    Grouped = gleam@list:fold(
        Events,
        maps:new(),
        fun(Acc, Event) ->
            Partition = case Partitions of
                0 -> 0;
                Gleam@denominator -> Partition_fn(Event) rem Gleam@denominator
            end,
            case gleam_stdlib:map_get(Acc, Partition) of
                {ok, Existing} ->
                    gleam@dict:insert(
                        Acc,
                        Partition,
                        lists:append(Existing, [Event])
                    );

                {error, _} ->
                    gleam@dict:insert(Acc, Partition, [Event])
            end
        end
    ),
    Result = gleam@dict:fold(
        Grouped,
        {[], [], State},
        fun(Acc@1, Partition@1, Partition_events) ->
            {All_assignments, All_buffered, Current_state} = Acc@1,
            Demand = case gleam_stdlib:map_get(
                erlang:element(2, Current_state),
                Partition@1
            ) of
                {ok, D} ->
                    D;

                {error, _} ->
                    0
            end,
            To_send = gleam@list:take(Partition_events, Demand),
            Remaining = gleam@list:drop(Partition_events, Demand),
            Sent_count = erlang:length(To_send),
            New_demand = gleam@int:max(0, Demand - Sent_count),
            New_state = {dispatcher_state,
                gleam@dict:insert(
                    erlang:element(2, Current_state),
                    Partition@1,
                    New_demand
                )},
            Assignments = case To_send of
                [] ->
                    All_assignments;

                _ ->
                    [{Partition@1, To_send} | All_assignments]
            end,
            {Assignments, lists:append(All_buffered, Remaining), New_state}
        end
    ),
    {Assignments@1, Buffered, Final_state} = Result,
    {dispatch_result, Assignments@1, Buffered, Final_state}.
