-module(stage@producer).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/producer.gleam").
-export([new_config/2, with_dispatcher/2, with_buffer_size/2, subject/1, start/1]).
-export_type([producer_config/2, producer_stage/1, producer_state/2]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Producer stage implementation.\n"
    "\n"
    " A producer generates events in response to demand from downstream consumers.\n"
    " It is an actor that tracks demand per subscriber and dispatches events\n"
    " according to the configured dispatcher strategy.\n"
).

-type producer_config(GLL, GLM) :: {producer_config,
        GLM,
        fun((integer(), GLM) -> {list(GLL), GLM}),
        stage@dispatcher:dispatcher_type(GLL),
        integer(),
        stage@buffer:overflow_strategy()}.

-type producer_stage(GLN) :: {producer_stage,
        gleam@erlang@process:subject(stage@internal@message:producer_message(GLN))}.

-type producer_state(GLO, GLP) :: {producer_state,
        GLP,
        fun((integer(), GLP) -> {list(GLO), GLP}),
        stage@dispatcher:dispatcher_type(GLO),
        stage@dispatcher:dispatcher_state(gleam@erlang@process:subject(stage@internal@message:producer_message(GLO))),
        stage@subscription:producer_registry(GLO),
        stage@buffer:buffer(GLO),
        gleam@erlang@process:subject(stage@internal@message:producer_message(GLO)),
        gleam@dict:dict(integer(), gleam@erlang@process:subject(stage@internal@message:producer_message(GLO)))}.

-file("src/stage/producer.gleam", 43).
?DOC(" Create a new producer config with sensible defaults.\n").
-spec new_config(GLQ, fun((integer(), GLQ) -> {list(GLR), GLQ})) -> producer_config(GLR, GLQ).
new_config(State, Callback) ->
    {producer_config, State, Callback, demand_dispatcher, 10000, drop_oldest}.

-file("src/stage/producer.gleam", 57).
?DOC(" Set the dispatcher strategy.\n").
-spec with_dispatcher(
    producer_config(GLV, GLW),
    stage@dispatcher:dispatcher_type(GLV)
) -> producer_config(GLV, GLW).
with_dispatcher(Config, D) ->
    {producer_config,
        erlang:element(2, Config),
        erlang:element(3, Config),
        D,
        erlang:element(5, Config),
        erlang:element(6, Config)}.

-file("src/stage/producer.gleam", 65).
?DOC(" Set the buffer size.\n").
-spec with_buffer_size(producer_config(GMC, GMD), integer()) -> producer_config(GMC, GMD).
with_buffer_size(Config, Size) ->
    {producer_config,
        erlang:element(2, Config),
        erlang:element(3, Config),
        erlang:element(4, Config),
        Size,
        erlang:element(6, Config)}.

-file("src/stage/producer.gleam", 78).
?DOC(" Get the subject for this producer (used by consumers to subscribe).\n").
-spec subject(producer_stage(GMI)) -> gleam@erlang@process:subject(stage@internal@message:producer_message(GMI)).
subject(Stage) ->
    erlang:element(2, Stage).

-file("src/stage/producer.gleam", 197).
?DOC(" Build a selector that listens on the main subject plus all demand subjects.\n").
-spec build_selector(
    gleam@erlang@process:subject(stage@internal@message:producer_message(GNT)),
    stage@subscription:producer_registry(GNT)
) -> gleam@erlang@process:selector(stage@internal@message:producer_message(GNT)).
build_selector(Self_subject, Registry) ->
    Subs = stage@subscription:producer_subscriptions(Registry),
    Base = begin
        _pipe = gleam_erlang_ffi:new_selector(),
        gleam@erlang@process:select(_pipe, Self_subject)
    end,
    gleam@list:fold(
        Subs,
        Base,
        fun(Sel, Sub) ->
            gleam@erlang@process:select(Sel, erlang:element(3, Sub))
        end
    ).

-file("src/stage/producer.gleam", 151).
?DOC(" Handle a subscribe request from a consumer.\n").
-spec handle_subscribe(
    producer_state(GND, GNE),
    gleam@erlang@process:subject(stage@internal@message:consumer_message(GND)),
    gleam@erlang@process:subject({ok,
            stage@internal@message:subscription_ref(GND)} |
        {error, binary()}),
    gleam@option:option(integer())
) -> gleam@otp@actor:next(producer_state(GND, GNE), stage@internal@message:producer_message(GND)).
handle_subscribe(State, Consumer_subject, Reply_to, Partition) ->
    Demand_subject = gleam@erlang@process:new_subject(),
    Ref = {subscription_ref, Consumer_subject, Demand_subject, erlang:self()},
    Sub = {producer_subscription, Consumer_subject, Demand_subject},
    Registry = stage@subscription:add_producer_subscription(
        erlang:element(6, State),
        Sub
    ),
    Dispatcher_state = stage@dispatcher:register(
        erlang:element(5, State),
        Demand_subject
    ),
    Partition_map = case Partition of
        {some, P} ->
            gleam@dict:insert(erlang:element(9, State), P, Demand_subject);

        none ->
            erlang:element(9, State)
    end,
    gleam@erlang@process:send(Reply_to, {ok, Ref}),
    Selector = build_selector(erlang:element(8, State), Registry),
    New_state = {producer_state,
        erlang:element(2, State),
        erlang:element(3, State),
        erlang:element(4, State),
        Dispatcher_state,
        Registry,
        erlang:element(7, State),
        erlang:element(8, State),
        Partition_map},
    _pipe = gleam@otp@actor:continue(New_state),
    gleam@otp@actor:with_selector(_pipe, Selector).

-file("src/stage/producer.gleam", 211).
?DOC(" Handle an unsubscribe request.\n").
-spec handle_unsubscribe(
    producer_state(GNZ, GOA),
    gleam@erlang@process:subject(stage@internal@message:producer_message(GNZ))
) -> gleam@otp@actor:next(producer_state(GNZ, GOA), stage@internal@message:producer_message(GNZ)).
handle_unsubscribe(State, Demand_subject) ->
    Registry = stage@subscription:remove_producer_subscription(
        erlang:element(6, State),
        Demand_subject
    ),
    Dispatcher_state = stage@dispatcher:unregister(
        erlang:element(5, State),
        Demand_subject
    ),
    Partition_map = gleam@dict:filter(
        erlang:element(9, State),
        fun(_, V) -> V /= Demand_subject end
    ),
    Selector = build_selector(erlang:element(8, State), Registry),
    New_state = {producer_state,
        erlang:element(2, State),
        erlang:element(3, State),
        erlang:element(4, State),
        Dispatcher_state,
        Registry,
        erlang:element(7, State),
        erlang:element(8, State),
        Partition_map},
    _pipe = gleam@otp@actor:continue(New_state),
    gleam@otp@actor:with_selector(_pipe, Selector).

-file("src/stage/producer.gleam", 372).
?DOC(
    " Partition dispatch: route events by partition function, mapping partition\n"
    " numbers to the appropriate demand subjects.\n"
).
-spec dispatch_partition(
    stage@dispatcher:dispatcher_state(gleam@erlang@process:subject(stage@internal@message:producer_message(GPU))),
    list(GPU),
    fun((GPU) -> integer()),
    integer(),
    gleam@dict:dict(integer(), gleam@erlang@process:subject(stage@internal@message:producer_message(GPU)))
) -> stage@dispatcher:dispatch_result(gleam@erlang@process:subject(stage@internal@message:producer_message(GPU)), GPU).
dispatch_partition(
    Dispatcher_state,
    Events,
    Partition_fn,
    Partitions,
    Partition_map
) ->
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
        {[], [], Dispatcher_state},
        fun(Acc@1, Partition@1, Partition_events) ->
            {All_assignments, All_buffered, Current_state} = Acc@1,
            case gleam_stdlib:map_get(Partition_map, Partition@1) of
                {error, _} ->
                    {All_assignments,
                        lists:append(All_buffered, Partition_events),
                        Current_state};

                {ok, Demand_subject} ->
                    Demand = case gleam_stdlib:map_get(
                        erlang:element(2, Current_state),
                        Demand_subject
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
                            Demand_subject,
                            New_demand
                        )},
                    Assignments = case To_send of
                        [] ->
                            All_assignments;

                        _ ->
                            [{Demand_subject, To_send} | All_assignments]
                    end,
                    {Assignments,
                        lists:append(All_buffered, Remaining),
                        New_state}
            end
        end
    ),
    {Assignments@1, Buffered, Final_state} = Result,
    {dispatch_result, Assignments@1, Buffered, Final_state}.

-file("src/stage/producer.gleam", 344).
?DOC(" Dispatch events using the configured dispatcher strategy and send to consumers.\n").
-spec dispatch_events(
    stage@dispatcher:dispatcher_type(GPF),
    stage@dispatcher:dispatcher_state(gleam@erlang@process:subject(stage@internal@message:producer_message(GPF))),
    list(GPF),
    stage@subscription:producer_registry(GPF),
    gleam@dict:dict(integer(), gleam@erlang@process:subject(stage@internal@message:producer_message(GPF)))
) -> stage@dispatcher:dispatch_result(gleam@erlang@process:subject(stage@internal@message:producer_message(GPF)), GPF).
dispatch_events(
    Dispatcher_type,
    Dispatcher_state,
    Events,
    Registry,
    Partition_map
) ->
    Result = case Dispatcher_type of
        demand_dispatcher ->
            stage@dispatcher:dispatch_demand(Dispatcher_state, Events);

        broadcast_dispatcher ->
            stage@dispatcher:dispatch_broadcast(Dispatcher_state, Events);

        {partition_dispatcher, Partition_fn, Partitions} ->
            dispatch_partition(
                Dispatcher_state,
                Events,
                Partition_fn,
                Partitions,
                Partition_map
            )
    end,
    stage@subscription:send_assignments(Registry, erlang:element(2, Result)),
    Result.

-file("src/stage/producer.gleam", 236).
?DOC(" Handle a demand request from a consumer.\n").
-spec handle_ask_demand(
    producer_state(GOK, GOL),
    gleam@erlang@process:subject(stage@internal@message:producer_message(GOK)),
    integer()
) -> gleam@otp@actor:next(producer_state(GOK, GOL), stage@internal@message:producer_message(GOK)).
handle_ask_demand(State, Demand_subject, Count) ->
    Dispatcher_state = stage@dispatcher:add_demand(
        erlang:element(5, State),
        Demand_subject,
        Count
    ),
    {State_after_buffer, Dispatcher_state@1} = case stage@buffer:is_empty(
        erlang:element(7, State)
    ) of
        true ->
            {State, Dispatcher_state};

        false ->
            Buffered_events = stage@buffer:to_list(erlang:element(7, State)),
            Dispatch_result = dispatch_events(
                erlang:element(4, State),
                Dispatcher_state,
                Buffered_events,
                erlang:element(6, State),
                erlang:element(9, State)
            ),
            New_buffer = stage@buffer:clear(erlang:element(7, State)),
            New_buffer@1 = case erlang:element(3, Dispatch_result) of
                [] ->
                    New_buffer;

                Remaining ->
                    Buf@1 = case stage@buffer:add(New_buffer, Remaining) of
                        {buffer_ok, Buf, _} -> Buf;
                        _assert_fail ->
                            erlang:error(#{gleam_error => let_assert,
                                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                                        file => <<?FILEPATH/utf8>>,
                                        module => <<"stage/producer"/utf8>>,
                                        function => <<"handle_ask_demand"/utf8>>,
                                        line => 262,
                                        value => _assert_fail,
                                        start => 8259,
                                        'end' => 8329,
                                        pattern_start => 8270,
                                        pattern_end => 8293})
                    end,
                    Buf@1
            end,
            {{producer_state,
                    erlang:element(2, State),
                    erlang:element(3, State),
                    erlang:element(4, State),
                    erlang:element(5, State),
                    erlang:element(6, State),
                    New_buffer@1,
                    erlang:element(8, State),
                    erlang:element(9, State)},
                erlang:element(4, Dispatch_result)}
    end,
    Remaining_demand = stage@dispatcher:total_demand(Dispatcher_state@1),
    case Remaining_demand > 0 of
        false ->
            gleam@otp@actor:continue(
                {producer_state,
                    erlang:element(2, State_after_buffer),
                    erlang:element(3, State_after_buffer),
                    erlang:element(4, State_after_buffer),
                    Dispatcher_state@1,
                    erlang:element(6, State_after_buffer),
                    erlang:element(7, State_after_buffer),
                    erlang:element(8, State_after_buffer),
                    erlang:element(9, State_after_buffer)}
            );

        true ->
            {Events, New_user_state} = (erlang:element(3, State))(
                Remaining_demand,
                erlang:element(2, State_after_buffer)
            ),
            Dispatch_result@1 = dispatch_events(
                erlang:element(4, State),
                Dispatcher_state@1,
                Events,
                erlang:element(6, State_after_buffer),
                erlang:element(9, State_after_buffer)
            ),
            New_buffer@2 = case erlang:element(3, Dispatch_result@1) of
                [] ->
                    erlang:element(7, State_after_buffer);

                Remaining@1 ->
                    Buf@3 = case stage@buffer:add(
                        erlang:element(7, State_after_buffer),
                        Remaining@1
                    ) of
                        {buffer_ok, Buf@2, _} -> Buf@2;
                        _assert_fail@1 ->
                            erlang:error(#{gleam_error => let_assert,
                                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                                        file => <<?FILEPATH/utf8>>,
                                        module => <<"stage/producer"/utf8>>,
                                        function => <<"handle_ask_demand"/utf8>>,
                                        line => 296,
                                        value => _assert_fail@1,
                                        start => 9244,
                                        'end' => 9347,
                                        pattern_start => 9255,
                                        pattern_end => 9278})
                    end,
                    Buf@3
            end,
            gleam@otp@actor:continue(
                {producer_state,
                    New_user_state,
                    erlang:element(3, State_after_buffer),
                    erlang:element(4, State_after_buffer),
                    erlang:element(4, Dispatch_result@1),
                    erlang:element(6, State_after_buffer),
                    New_buffer@2,
                    erlang:element(8, State_after_buffer),
                    erlang:element(9, State_after_buffer)}
            )
    end.

-file("src/stage/producer.gleam", 313).
?DOC(" Handle externally emitted events — dispatch if demand, otherwise buffer.\n").
-spec handle_emit(producer_state(GOV, GOW), list(GOV)) -> gleam@otp@actor:next(producer_state(GOV, GOW), stage@internal@message:producer_message(GOV)).
handle_emit(State, Events) ->
    Dispatch_result = dispatch_events(
        erlang:element(4, State),
        erlang:element(5, State),
        Events,
        erlang:element(6, State),
        erlang:element(9, State)
    ),
    New_buffer = case erlang:element(3, Dispatch_result) of
        [] ->
            erlang:element(7, State);

        Remaining ->
            case stage@buffer:add(erlang:element(7, State), Remaining) of
                {buffer_ok, Buf, _} ->
                    Buf;

                {buffer_overflow, Buf@1, _} ->
                    Buf@1
            end
    end,
    gleam@otp@actor:continue(
        {producer_state,
            erlang:element(2, State),
            erlang:element(3, State),
            erlang:element(4, State),
            erlang:element(4, Dispatch_result),
            erlang:element(6, State),
            New_buffer,
            erlang:element(8, State),
            erlang:element(9, State)}
    ).

-file("src/stage/producer.gleam", 133).
?DOC(" Handle an incoming message to the producer.\n").
-spec handle_message(
    producer_state(GMT, GMU),
    stage@internal@message:producer_message(GMT)
) -> gleam@otp@actor:next(producer_state(GMT, GMU), stage@internal@message:producer_message(GMT)).
handle_message(State, Msg) ->
    case Msg of
        {subscribe, Consumer_subject, Reply_to, Partition} ->
            handle_subscribe(State, Consumer_subject, Reply_to, Partition);

        {unsubscribe, Demand_subject} ->
            handle_unsubscribe(State, Demand_subject);

        {ask_demand, Demand_subject@1, Count} ->
            handle_ask_demand(State, Demand_subject@1, Count);

        {emit, Events} ->
            handle_emit(State, Events)
    end.

-file("src/stage/producer.gleam", 98).
?DOC(" Start a producer stage.\n").
-spec start(producer_config(GMM, any())) -> {ok, producer_stage(GMM)} |
    {error, stage@error:stage_error()}.
start(Config) ->
    Builder = begin
        _pipe@4 = gleam@otp@actor:new_with_initialiser(
            5000,
            fun(Self_subject) ->
                Init_state = {producer_state,
                    erlang:element(2, Config),
                    erlang:element(3, Config),
                    erlang:element(4, Config),
                    stage@dispatcher:new_state(),
                    stage@subscription:new_producer_registry(),
                    stage@buffer:new(
                        erlang:element(5, Config),
                        erlang:element(6, Config)
                    ),
                    Self_subject,
                    maps:new()},
                Selector = begin
                    _pipe = gleam_erlang_ffi:new_selector(),
                    gleam@erlang@process:select(_pipe, Self_subject)
                end,
                _pipe@1 = gleam@otp@actor:initialised(Init_state),
                _pipe@2 = gleam@otp@actor:selecting(_pipe@1, Selector),
                _pipe@3 = gleam@otp@actor:returning(_pipe@2, Self_subject),
                {ok, _pipe@3}
            end
        ),
        gleam@otp@actor:on_message(_pipe@4, fun handle_message/2)
    end,
    case gleam@otp@actor:start(Builder) of
        {ok, Started} ->
            {ok, {producer_stage, erlang:element(3, Started)}};

        {error, _} ->
            {error, start_failed}
    end.
