-module(stage).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage.gleam").
-export([start_producer/1, producer_subject/1, start_consumer/1, subscribe/3, ask/3, consumer_state/2, start_producer_consumer/1, producer_consumer_producer_subject/1, subscribe_producer_consumer/3, default_subscribe_opts/0, auto_subscribe_opts/2, manual_subscribe_opts/0, partition_subscribe_opts/3, emit/2]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " stage — Demand-driven backpressure pipelines for Gleam.\n"
    "\n"
    " A pure Gleam implementation of GenStage. Each stage is a process that\n"
    " exchanges events with back-pressure. Producers generate events, consumers\n"
    " consume them, and producer-consumers do both.\n"
    "\n"
    " Demand flows upstream. Events flow downstream. The protocol guarantees\n"
    " that producers never emit more than demanded.\n"
).

-file("src/stage.gleam", 75).
?DOC(
    " Start a producer stage.\n"
    "\n"
    " The `on_demand` callback is called when consumers request events.\n"
    " It receives the demanded count and current state, and returns\n"
    " events to send downstream plus the new state.\n"
    "\n"
    " ## Example\n"
    "\n"
    " ```gleam\n"
    " let config = producer.new_config(\n"
    "   init_state: 0,\n"
    "   on_demand: fn(demand, counter) {\n"
    "     let events = list.range(counter, counter + demand - 1)\n"
    "     #(events, counter + demand)\n"
    "   },\n"
    " )\n"
    " let assert Ok(stage) = stage.start_producer(config)\n"
    " ```\n"
).
-spec start_producer(stage@producer:producer_config(IDG, any())) -> {ok,
        stage@producer:producer_stage(IDG)} |
    {error, stage@error:stage_error()}.
start_producer(Config) ->
    stage@producer:start(Config).

-file("src/stage.gleam", 82).
?DOC(" Get the subject for subscribing to a producer.\n").
-spec producer_subject(stage@producer:producer_stage(IDN)) -> gleam@erlang@process:subject(stage@internal@message:producer_message(IDN)).
producer_subject(Stage) ->
    stage@producer:subject(Stage).

-file("src/stage.gleam", 115).
?DOC(
    " Start a consumer stage.\n"
    "\n"
    " The `on_events` callback is called when events arrive from upstream.\n"
    " It receives the list of events and current state, and returns the\n"
    " new state.\n"
    "\n"
    " ## Example\n"
    "\n"
    " ```gleam\n"
    " let config = consumer.new_config(\n"
    "   init_state: [],\n"
    "   on_events: fn(events, collected) {\n"
    "     list.append(collected, events)\n"
    "   },\n"
    " )\n"
    " let assert Ok(stage) = stage.start_consumer(config)\n"
    " ```\n"
).
-spec start_consumer(stage@consumer:consumer_config(IDR, any())) -> {ok,
        stage@consumer:consumer_stage(IDR)} |
    {error, stage@error:stage_error()}.
start_consumer(Config) ->
    stage@consumer:start(Config).

-file("src/stage.gleam", 126).
?DOC(
    " Subscribe a consumer to a producer.\n"
    "\n"
    " This wires the demand protocol between the two stages. The consumer\n"
    " will begin requesting events from the producer according to the\n"
    " demand mode in the subscribe options.\n"
).
-spec subscribe(
    stage@consumer:consumer_stage(IDY),
    stage@producer:producer_stage(IDY),
    stage@subscription:subscribe_opts()
) -> {ok, nil} | {error, stage@error:stage_error()}.
subscribe(Consumer_stage, Producer_stage, Opts) ->
    stage@consumer:subscribe(
        Consumer_stage,
        stage@producer:subject(Producer_stage),
        Opts
    ).

-file("src/stage.gleam", 135).
?DOC(" Manually ask for events (when using Manual demand mode).\n").
-spec ask(
    stage@consumer:consumer_stage(IED),
    gleam@erlang@process:subject(stage@internal@message:producer_message(IED)),
    integer()
) -> nil.
ask(Consumer_stage, Producer_subject, Count) ->
    stage@consumer:ask(Consumer_stage, Producer_subject, Count).

-file("src/stage.gleam", 145).
?DOC(
    " Get the current state from a consumer (useful in tests).\n"
    " The caller must know the actual state type.\n"
).
-spec consumer_state(stage@consumer:consumer_stage(any()), integer()) -> any().
consumer_state(Consumer_stage, Timeout) ->
    stage@consumer:get_state(Consumer_stage, Timeout).

-file("src/stage.gleam", 167).
?DOC(
    " Start a producer-consumer stage.\n"
    "\n"
    " The `on_events` callback transforms incoming events into outgoing events.\n"
    " It receives events from upstream and the current state, returning\n"
    " new events to send downstream plus the new state.\n"
).
-spec start_producer_consumer(
    stage@producer_consumer:producer_consumer_config(IEK, IEL, any())
) -> {ok, stage@producer_consumer:producer_consumer_stage(IEK, IEL)} |
    {error, stage@error:stage_error()}.
start_producer_consumer(Config) ->
    stage@producer_consumer:start(Config).

-file("src/stage.gleam", 175).
?DOC(
    " Get the producer subject from a producer-consumer (for downstream consumers\n"
    " to subscribe to).\n"
).
-spec producer_consumer_producer_subject(
    stage@producer_consumer:producer_consumer_stage(any(), IEV)
) -> gleam@erlang@process:subject(stage@internal@message:producer_message(IEV)).
producer_consumer_producer_subject(Stage) ->
    stage@producer_consumer:producer_subject(Stage).

-file("src/stage.gleam", 182).
?DOC(" Subscribe a producer-consumer to an upstream producer.\n").
-spec subscribe_producer_consumer(
    stage@producer_consumer:producer_consumer_stage(IFA, any()),
    stage@producer:producer_stage(IFA),
    stage@subscription:subscribe_opts()
) -> {ok, nil} | {error, stage@error:stage_error()}.
subscribe_producer_consumer(Pc_stage, Producer_stage, Opts) ->
    stage@producer_consumer:subscribe(
        Pc_stage,
        stage@producer:subject(Producer_stage),
        Opts
    ).

-file("src/stage.gleam", 194).
?DOC(
    " Create default subscribe options.\n"
    " Automatic demand with max_demand=1000, min_demand=500.\n"
).
-spec default_subscribe_opts() -> stage@subscription:subscribe_opts().
default_subscribe_opts() ->
    stage@subscription:default_opts().

-file("src/stage.gleam", 199).
?DOC(" Create subscribe options with custom demand parameters.\n").
-spec auto_subscribe_opts(integer(), integer()) -> stage@subscription:subscribe_opts().
auto_subscribe_opts(Max, Min) ->
    {subscribe_opts, {automatic, Max, Min}, temporary, none}.

-file("src/stage.gleam", 208).
?DOC(" Create subscribe options for manual demand mode.\n").
-spec manual_subscribe_opts() -> stage@subscription:subscribe_opts().
manual_subscribe_opts() ->
    {subscribe_opts, manual, temporary, none}.

-file("src/stage.gleam", 217).
?DOC(" Create subscribe options for a specific partition (used with PartitionDispatcher).\n").
-spec partition_subscribe_opts(integer(), integer(), integer()) -> stage@subscription:subscribe_opts().
partition_subscribe_opts(P, Max, Min) ->
    {subscribe_opts, {automatic, Max, Min}, temporary, {some, P}}.

-file("src/stage.gleam", 231).
?DOC(
    " Push events directly into a producer (bypasses on_demand callback).\n"
    " Events are dispatched to consumers if demand exists, otherwise buffered.\n"
).
-spec emit(stage@producer:producer_stage(IFH), list(IFH)) -> nil.
emit(Producer_stage, Events) ->
    gleam@erlang@process:send(
        stage@producer:subject(Producer_stage),
        {emit, Events}
    ).
