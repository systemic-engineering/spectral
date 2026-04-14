%% @doc Oracle: orchestrator that wires up gen_stage pipelines and runs
%% deterministic scenarios exercising all GenStage features.
%%
%% Each scenario starts stages, runs them to completion, collects sorted
%% results, and stops all stages. Results are deterministic -- the Gleam
%% implementation must produce identical output.
%%
%% Scenarios:
%%   1. basic_pipeline   -- Producer -> Filter(evens) -> Transformer(2x) -> Consumer
%%   2. broadcast        -- Producer with BroadcastDispatcher -> 2 consumers
%%   3. partition        -- Producer with PartitionDispatcher(even/odd) -> 2 consumers
%%   4. manual_demand    -- Consumer with manual demand, explicit ask
%%   5. multi_consumer   -- One producer, 2 consumers, DemandDispatcher
%%   6. cancellation     -- permanent, transient, temporary cancel modes
%%   7. buffer_overflow  -- Producer buffers events when no consumer demand
%% @end
-module(oracle).

-export([run_all/0,
         run_basic_pipeline/0,
         run_broadcast/0,
         run_partition/0,
         run_manual_demand/0,
         run_multi_consumer/0,
         run_cancellation/0,
         run_buffer_overflow/0]).

%% ---------------------------------------------------------------------------
%% run_all/0
%% ---------------------------------------------------------------------------

-spec run_all() -> [{atom(), term()}].
run_all() ->
    [{basic_pipeline,  run_basic_pipeline()},
     {broadcast,       run_broadcast()},
     {partition,       run_partition()},
     {manual_demand,   run_manual_demand()},
     {multi_consumer,  run_multi_consumer()},
     {cancellation,    run_cancellation()},
     {buffer_overflow, run_buffer_overflow()}].

%% ---------------------------------------------------------------------------
%% Scenario 1: Basic Pipeline
%%
%%   Producer(0..9) -> Filter(keep evens) -> Transformer(double) -> Consumer
%%
%%   Input sequence: 0,1,2,3,4,5,6,7,8,9
%%   After filter:   0,2,4,6,8
%%   After double:   0,4,8,12,16
%%
%%   Expected: [0, 4, 8, 12, 16]
%% ---------------------------------------------------------------------------

-spec run_basic_pipeline() -> [integer()].
run_basic_pipeline() ->
    Self = self(),

    {ok, Producer} = oracle_producer:start_link(0),

    {ok, Filter} = oracle_filter:start_link(
        [{subscribe_to, [{Producer, [{max_demand, 10}, {min_demand, 0}]}]}]
    ),

    {ok, Transformer} = oracle_transformer:start_link(
        [{subscribe_to, [{Filter, [{max_demand, 10}, {min_demand, 0}]}]}]
    ),

    {ok, Consumer} = oracle_consumer:start_link(Self,
        [{subscribe_to, [{Transformer, [{max_demand, 5}, {min_demand, 0}]}]}]
    ),

    wait_for_events(Consumer, 5, 5000),
    Result = oracle_consumer:get_events(Consumer),
    stop_all([Consumer, Transformer, Filter, Producer]),
    Result.

%% ---------------------------------------------------------------------------
%% Scenario 2: Broadcast
%%
%%   Producer(0..9) with BroadcastDispatcher -> Consumer1, Consumer2
%%   Both consumers receive ALL events.
%%
%%   Expected: {[0,1,2,3,4,5,6,7,8,9], [0,1,2,3,4,5,6,7,8,9]}
%% ---------------------------------------------------------------------------

-spec run_broadcast() -> {[integer()], [integer()]}.
run_broadcast() ->
    Self = self(),

    %% Producer in accumulate mode with BroadcastDispatcher
    {ok, Producer} = oracle_producer:start_link(0,
        [{dispatcher, gen_stage_broadcast_dispatcher},
         {demand, accumulate}]),

    %% Both consumers use manual demand for precise control
    {ok, C1} = oracle_consumer:start_link(manual, Self, []),
    {ok, C2} = oracle_consumer:start_link(manual, Self, []),

    %% Subscribe both before any demand flows
    {ok, Sub1} = gen_stage:sync_subscribe(C1,
        [{to, Producer}, {max_demand, 10}, {min_demand, 0}]),
    {ok, Sub2} = gen_stage:sync_subscribe(C2,
        [{to, Producer}, {max_demand, 10}, {min_demand, 0}]),

    %% Switch to forward mode
    gen_stage:demand(Producer, forward),

    %% Both consumers ask for exactly 10 events (from within their process)
    oracle_consumer:ask(C1, {Producer, Sub1}, 10),
    oracle_consumer:ask(C2, {Producer, Sub2}, 10),

    wait_for_events(C1, 10, 5000),
    wait_for_events(C2, 10, 5000),

    R1 = oracle_consumer:get_events(C1),
    R2 = oracle_consumer:get_events(C2),
    stop_all([C1, C2, Producer]),
    {R1, R2}.

%% ---------------------------------------------------------------------------
%% Scenario 3: Partition
%%
%%   Producer(0..9) with PartitionDispatcher(even/odd) -> CEven, COdd
%%
%%   Hash: Event rem 2 == 0 -> partition 0, else -> partition 1
%%
%%   Expected: {[0,2,4,6,8], [1,3,5,7,9]}
%% ---------------------------------------------------------------------------

-spec run_partition() -> {[integer()], [integer()]}.
run_partition() ->
    Self = self(),

    Hash = fun(Event) -> {Event, Event rem 2} end,

    %% Producer in accumulate mode with PartitionDispatcher
    {ok, Producer} = oracle_producer:start_link(0,
        [{dispatcher, {gen_stage_partition_dispatcher,
                       [{partitions, 2}, {hash, Hash}]}},
         {demand, accumulate}]),

    %% Manual demand consumers for deterministic results
    {ok, CEven} = oracle_consumer:start_link(manual, Self, []),
    {ok, COdd}  = oracle_consumer:start_link(manual, Self, []),

    {ok, SubEven} = gen_stage:sync_subscribe(CEven,
        [{to, Producer}, {partition, 0}, {max_demand, 10}, {min_demand, 0}]),
    {ok, SubOdd} = gen_stage:sync_subscribe(COdd,
        [{to, Producer}, {partition, 1}, {max_demand, 10}, {min_demand, 0}]),

    %% Switch to forward, then ask for events.
    %% Ask 10 from each partition consumer. The producer will get demand=10
    %% from each, and generate 20 integers (0..19).
    %% Partition 0 (even): 0,2,4,6,8,10,12,14,16,18 -> 10 events
    %% Partition 1 (odd):  1,3,5,7,9,11,13,15,17,19 -> 10 events
    %% But we only want 5 per consumer for cleaner output.
    gen_stage:demand(Producer, forward),

    oracle_consumer:ask(CEven, {Producer, SubEven}, 5),
    oracle_consumer:ask(COdd, {Producer, SubOdd}, 5),

    wait_for_events(CEven, 5, 5000),
    wait_for_events(COdd,  5, 5000),

    REven = oracle_consumer:get_events(CEven),
    ROdd  = oracle_consumer:get_events(COdd),
    stop_all([CEven, COdd, Producer]),
    {REven, ROdd}.

%% ---------------------------------------------------------------------------
%% Scenario 4: Manual Demand
%%
%%   Producer(0..N) -> Consumer with manual demand.
%%   Consumer explicitly asks for 5 events, then 3 more.
%%
%%   Expected: [0,1,2,3,4,5,6,7]
%% ---------------------------------------------------------------------------

-spec run_manual_demand() -> [integer()].
run_manual_demand() ->
    Self = self(),

    {ok, Producer} = oracle_producer:start_link(0),
    {ok, Consumer} = oracle_consumer:start_link(manual, Self, []),

    {ok, SubTag} = gen_stage:sync_subscribe(Consumer,
        [{to, Producer}, {max_demand, 10}, {min_demand, 0}]),

    %% Ask for 5, wait, ask for 3 more (from within the consumer process)
    oracle_consumer:ask(Consumer, {Producer, SubTag}, 5),
    wait_for_events(Consumer, 5, 5000),

    oracle_consumer:ask(Consumer, {Producer, SubTag}, 3),
    wait_for_events(Consumer, 8, 5000),

    Result = oracle_consumer:get_events(Consumer),
    stop_all([Consumer, Producer]),
    Result.

%% ---------------------------------------------------------------------------
%% Scenario 5: Multi-Consumer (DemandDispatcher)
%%
%%   Producer(0..N) with default DemandDispatcher -> C1, C2.
%%   Events dispatched to consumer with most outstanding demand.
%%
%%   Both subscribe with max_demand=5, min_demand=0.
%%   We collect until total >= 10 across both.
%%   Return sorted union -- deterministic regardless of dispatch order.
%%
%%   Expected: [0,1,2,3,4,5,6,7,8,9]  (as sorted union)
%% ---------------------------------------------------------------------------

-spec run_multi_consumer() -> {[integer()], [integer()]}.
run_multi_consumer() ->
    Self = self(),

    %% Producer in accumulate mode with default DemandDispatcher
    {ok, Producer} = oracle_producer:start_link(0,
        [{demand, accumulate}]),

    %% Manual demand consumers
    {ok, C1} = oracle_consumer:start_link(manual, Self, []),
    {ok, C2} = oracle_consumer:start_link(manual, Self, []),

    {ok, Sub1} = gen_stage:sync_subscribe(C1,
        [{to, Producer}, {max_demand, 5}, {min_demand, 0}]),
    {ok, Sub2} = gen_stage:sync_subscribe(C2,
        [{to, Producer}, {max_demand, 5}, {min_demand, 0}]),

    %% Switch to forward, then ask for 5 events from each consumer.
    %% DemandDispatcher sends events to consumer with most outstanding demand.
    %% With both asking for 5 simultaneously, the dispatcher will interleave.
    gen_stage:demand(Producer, forward),

    oracle_consumer:ask(C1, {Producer, Sub1}, 5),
    oracle_consumer:ask(C2, {Producer, Sub2}, 5),

    wait_for_total_events([C1, C2], 10, 5000),

    R1 = oracle_consumer:get_events(C1),
    R2 = oracle_consumer:get_events(C2),
    stop_all([C1, C2, Producer]),

    %% Sorted union is deterministic regardless of dispatch ordering.
    %% Individual consumer results may vary, but the union is always [0..9].
    Union = lists:sort(R1 ++ R2),
    {Union, Union}.

%% ---------------------------------------------------------------------------
%% Scenario 6: Cancellation Modes
%%
%%   Tests permanent, transient, temporary cancel modes by killing
%%   the producer and checking whether the consumer survives.
%%
%%   - permanent: consumer exits when producer exits (any reason)
%%   - transient + normal exit: consumer stays alive
%%   - transient + abnormal exit: consumer exits
%%   - temporary: consumer stays alive (any reason)
%%
%%   Returns [{mode, consumer_alive :: boolean()}]
%% ---------------------------------------------------------------------------

-spec run_cancellation() -> [{atom(), boolean()}].
run_cancellation() ->
    Permanent        = test_cancel_mode(permanent, shutdown),
    TransientNormal  = test_cancel_mode(transient, shutdown),
    TransientAbnorm  = test_cancel_mode(transient, kill),
    Temporary        = test_cancel_mode(temporary, shutdown),

    [{permanent,          Permanent},
     {transient_normal,   TransientNormal},
     {transient_abnormal, TransientAbnorm},
     {temporary,          Temporary}].

%% ---------------------------------------------------------------------------
%% Scenario 7: Buffer Overflow
%%
%%   Producer with buffer_size=5 and buffer_keep=last.
%%   Events emitted while no consumer is subscribed go into the buffer.
%%   We emit 8 events; buffer holds last 5, dropping first 3.
%%   Then subscribe consumer and collect.
%%
%%   Steps:
%%   1. Start producer in forward mode with buffer_size=5
%%   2. Push 8 events via {emit, Events} info message (no consumer = buffered)
%%   3. Wait for buffering
%%   4. Subscribe consumer
%%   5. Consumer gets buffered events + demand-generated events
%%
%%   Expected: first 5 events received are [3,4,5,6,7] (the surviving buffer)
%% ---------------------------------------------------------------------------

-spec run_buffer_overflow() -> [integer()].
run_buffer_overflow() ->
    Self = self(),

    {ok, Producer} = oracle_producer:start_link(1000,
        [{buffer_size, 5}, {buffer_keep, last}]),

    %% Emit 8 events with no consumer -- they go into the buffer.
    %% Buffer keeps last 5: [103,104,105,106,107]
    %% We use values starting at 100 so they are distinguishable from
    %% counter-generated values (which start at 1000).
    Producer ! {emit, lists:seq(100, 107)},
    timer:sleep(100),

    %% Now subscribe consumer. It receives buffered events first,
    %% then demand-generated events from counter (1000+).
    {ok, Consumer} = oracle_consumer:start_link(Self, []),
    {ok, _} = gen_stage:sync_subscribe(Consumer,
        [{to, Producer}, {max_demand, 5}, {min_demand, 0}]),

    wait_for_events(Consumer, 5, 5000),
    AllEvents = oracle_consumer:get_events(Consumer),
    stop_all([Consumer, Producer]),

    %% Extract only the buffered events (values < 1000).
    %% These are the ones that survived the buffer overflow.
    Buffered = lists:sort([E || E <- AllEvents, E < 1000]),
    Buffered.

%% ---------------------------------------------------------------------------
%% Internal helpers
%% ---------------------------------------------------------------------------

%% @doc Wait until a consumer has accumulated at least N events.
wait_for_events(Consumer, N, Timeout) ->
    Deadline = erlang:monotonic_time(millisecond) + Timeout,
    wait_for_events_loop(Consumer, N, Deadline).

wait_for_events_loop(Consumer, N, Deadline) ->
    Now = erlang:monotonic_time(millisecond),
    case Now >= Deadline of
        true ->
            error({timeout_waiting_for_events, N});
        false ->
            Events = oracle_consumer:get_events(Consumer),
            case length(Events) >= N of
                true  -> ok;
                false ->
                    timer:sleep(10),
                    wait_for_events_loop(Consumer, N, Deadline)
            end
    end.

%% @doc Wait until total events across all consumers >= N.
wait_for_total_events(Consumers, N, Timeout) ->
    Deadline = erlang:monotonic_time(millisecond) + Timeout,
    wait_for_total_events_loop(Consumers, N, Deadline).

wait_for_total_events_loop(Consumers, N, Deadline) ->
    Now = erlang:monotonic_time(millisecond),
    case Now >= Deadline of
        true ->
            error({timeout_waiting_for_total_events, N});
        false ->
            Total = lists:sum([length(oracle_consumer:get_events(C))
                               || C <- Consumers]),
            case Total >= N of
                true  -> ok;
                false ->
                    timer:sleep(10),
                    wait_for_total_events_loop(Consumers, N, Deadline)
                end
    end.

%% @doc Stop all stages, ignoring errors from already-dead processes.
stop_all(Pids) ->
    lists:foreach(fun(Pid) ->
        try gen_stage:stop(Pid, shutdown, 1000)
        catch _:_ -> ok
        end
    end, Pids).

%% @doc Test a cancellation mode. Returns whether the consumer is alive
%% after the producer exits with the given reason.
test_cancel_mode(CancelMode, ExitReason) ->
    Self = self(),

    {ok, Producer} = oracle_producer:start_link(0),
    {ok, Consumer} = oracle_consumer:start_link(Self, []),

    {ok, _} = gen_stage:sync_subscribe(Consumer,
        [{to, Producer}, {cancel, CancelMode},
         {max_demand, 5}, {min_demand, 0}]),

    %% Let some events flow so the subscription is fully established
    wait_for_events(Consumer, 5, 5000),

    %% Monitor consumer to detect exit
    MonRef = erlang:monitor(process, Consumer),

    %% Stop the producer with the specified reason
    case ExitReason of
        kill     -> exit(Producer, kill);
        shutdown -> gen_stage:stop(Producer, shutdown, 1000)
    end,

    %% Wait for cancellation to propagate
    timer:sleep(200),

    Alive = erlang:is_process_alive(Consumer),

    %% Cleanup
    erlang:demonitor(MonRef, [flush]),
    case Alive of
        true ->
            try gen_stage:stop(Consumer, shutdown, 1000)
            catch _:_ -> ok
            end;
        false -> ok
    end,

    Alive.
