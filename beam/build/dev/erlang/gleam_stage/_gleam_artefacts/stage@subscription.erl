-module(stage@subscription).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/subscription.gleam").
-export([default_opts/0, new_consumer_registry/0, add_consumer_subscription/2, get_consumer_subscription/2, consumer_subscriptions/1, track_received_events/3, new_producer_registry/0, add_producer_subscription/2, remove_producer_subscription/2, get_producer_subscription/2, producer_subscriptions/1, send_assignments/2]).
-export_type([demand_mode/0, cancel/0, subscribe_opts/0, consumer_subscription/1, producer_subscription/1, consumer_registry/1, producer_registry/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Subscription management for stage pipelines.\n"
    "\n"
    " Tracks active subscriptions between producers and consumers,\n"
    " including demand mode configuration and cancellation behavior.\n"
).

-type demand_mode() :: {automatic, integer(), integer()} | manual.

-type cancel() :: permanent | transient | temporary.

-type subscribe_opts() :: {subscribe_opts,
        demand_mode(),
        cancel(),
        gleam@option:option(integer())}.

-type consumer_subscription(FKH) :: {consumer_subscription,
        gleam@erlang@process:subject(stage@internal@message:producer_message(FKH)),
        gleam@erlang@process:subject(stage@internal@message:consumer_message(FKH)),
        subscribe_opts(),
        integer()}.

-type producer_subscription(FKI) :: {producer_subscription,
        gleam@erlang@process:subject(stage@internal@message:consumer_message(FKI)),
        gleam@erlang@process:subject(stage@internal@message:producer_message(FKI))}.

-type consumer_registry(FKJ) :: {consumer_registry,
        gleam@dict:dict(gleam@erlang@process:subject(stage@internal@message:producer_message(FKJ)), consumer_subscription(FKJ))}.

-type producer_registry(FKK) :: {producer_registry,
        gleam@dict:dict(gleam@erlang@process:subject(stage@internal@message:producer_message(FKK)), producer_subscription(FKK))}.

-file("src/stage/subscription.gleam", 43).
?DOC(" Default subscribe options: automatic demand with max=1000, min=500.\n").
-spec default_opts() -> subscribe_opts().
default_opts() ->
    {subscribe_opts, {automatic, 1000, 500}, temporary, none}.

-file("src/stage/subscription.gleam", 88).
?DOC(" Create a new empty consumer registry.\n").
-spec new_consumer_registry() -> consumer_registry(any()).
new_consumer_registry() ->
    {consumer_registry, maps:new()}.

-file("src/stage/subscription.gleam", 93).
?DOC(" Add a subscription to the consumer registry.\n").
-spec add_consumer_subscription(
    consumer_registry(FKN),
    consumer_subscription(FKN)
) -> consumer_registry(FKN).
add_consumer_subscription(Registry, Sub) ->
    {consumer_registry,
        gleam@dict:insert(
            erlang:element(2, Registry),
            erlang:element(2, Sub),
            Sub
        )}.

-file("src/stage/subscription.gleam", 107).
?DOC(" Get a subscription from the consumer registry.\n").
-spec get_consumer_subscription(
    consumer_registry(FKR),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FKR))
) -> {ok, consumer_subscription(FKR)} | {error, nil}.
get_consumer_subscription(Registry, Demand_subject) ->
    gleam_stdlib:map_get(erlang:element(2, Registry), Demand_subject).

-file("src/stage/subscription.gleam", 115).
?DOC(" Get all subscriptions in the consumer registry.\n").
-spec consumer_subscriptions(consumer_registry(FKY)) -> list(consumer_subscription(FKY)).
consumer_subscriptions(Registry) ->
    maps:values(erlang:element(2, Registry)).

-file("src/stage/subscription.gleam", 124).
?DOC(
    " Track received events against a consumer subscription's demand.\n"
    " Returns updated registry and the count to replenish (0 = no ask needed).\n"
    " Caller sends AskDemand(demand_subject, ask_count) when ask_count > 0.\n"
).
-spec track_received_events(
    consumer_registry(FLC),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FLC)),
    integer()
) -> {consumer_registry(FLC), integer()}.
track_received_events(Registry, Demand_subject, Event_count) ->
    case get_consumer_subscription(Registry, Demand_subject) of
        {error, _} ->
            {Registry, 0};

        {ok, Sub} ->
            New_pending = erlang:element(5, Sub) - Event_count,
            {Final_pending, Ask_count} = case erlang:element(
                2,
                erlang:element(4, Sub)
            ) of
                {automatic, Max_demand, Min_demand} ->
                    case New_pending =< Min_demand of
                        true ->
                            Ask = Max_demand - New_pending,
                            {New_pending + Ask, Ask};

                        false ->
                            {New_pending, 0}
                    end;

                manual ->
                    {New_pending, 0}
            end,
            Updated = {consumer_subscription,
                erlang:element(2, Sub),
                erlang:element(3, Sub),
                erlang:element(4, Sub),
                Final_pending},
            {{consumer_registry,
                    gleam@dict:insert(
                        erlang:element(2, Registry),
                        Demand_subject,
                        Updated
                    )},
                Ask_count}
    end.

-file("src/stage/subscription.gleam", 172).
?DOC(" Create a new empty producer registry.\n").
-spec new_producer_registry() -> producer_registry(any()).
new_producer_registry() ->
    {producer_registry, maps:new()}.

-file("src/stage/subscription.gleam", 177).
?DOC(" Add a subscription to the producer registry.\n").
-spec add_producer_subscription(
    producer_registry(FLJ),
    producer_subscription(FLJ)
) -> producer_registry(FLJ).
add_producer_subscription(Registry, Sub) ->
    {producer_registry,
        gleam@dict:insert(
            erlang:element(2, Registry),
            erlang:element(3, Sub),
            Sub
        )}.

-file("src/stage/subscription.gleam", 191).
?DOC(" Remove a subscription from the producer registry.\n").
-spec remove_producer_subscription(
    producer_registry(FLN),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FLN))
) -> producer_registry(FLN).
remove_producer_subscription(Registry, Demand_subject) ->
    {producer_registry,
        gleam@dict:delete(erlang:element(2, Registry), Demand_subject)}.

-file("src/stage/subscription.gleam", 201).
?DOC(" Get a subscription from the producer registry by demand subject.\n").
-spec get_producer_subscription(
    producer_registry(FLS),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FLS))
) -> {ok, producer_subscription(FLS)} | {error, nil}.
get_producer_subscription(Registry, Demand_subject) ->
    gleam_stdlib:map_get(erlang:element(2, Registry), Demand_subject).

-file("src/stage/subscription.gleam", 209).
?DOC(" Get all subscriptions in the producer registry.\n").
-spec producer_subscriptions(producer_registry(FLZ)) -> list(producer_subscription(FLZ)).
producer_subscriptions(Registry) ->
    maps:values(erlang:element(2, Registry)).

-file("src/stage/subscription.gleam", 216).
?DOC(" Send dispatched event assignments to downstream consumers.\n").
-spec send_assignments(
    producer_registry(FMD),
    list({gleam@erlang@process:subject(stage@internal@message:producer_message(FMD)),
        list(FMD)})
) -> nil.
send_assignments(Registry, Assignments) ->
    gleam@list:each(
        Assignments,
        fun(Assignment) ->
            {Demand_subject, Events} = Assignment,
            case get_producer_subscription(Registry, Demand_subject) of
                {ok, Sub} ->
                    gleam@erlang@process:send(
                        erlang:element(2, Sub),
                        {send_events, Demand_subject, Events}
                    );

                {error, _} ->
                    nil
            end
        end
    ).
