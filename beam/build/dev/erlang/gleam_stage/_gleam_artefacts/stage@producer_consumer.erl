-module(stage@producer_consumer).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/producer_consumer.gleam").
-export([new_config/2, producer_subject/1, subscribe/3, start/1]).
-export_type([producer_consumer_config/3, producer_consumer_stage/2, p_c_message/2, p_c_state/3]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " ProducerConsumer stage implementation.\n"
    "\n"
    " A producer-consumer receives events from upstream producers, transforms\n"
    " them, and emits new events to downstream consumers. It implements both\n"
    " the producer and consumer protocols.\n"
).

-type producer_consumer_config(HGC, HGD, HGE) :: {producer_consumer_config,
        HGE,
        fun((list(HGC), HGE) -> {list(HGD), HGE}),
        stage@dispatcher:dispatcher_type(HGD),
        integer(),
        stage@buffer:overflow_strategy()}.

-type producer_consumer_stage(HGF, HGG) :: {producer_consumer_stage,
        gleam@erlang@process:subject(p_c_message(HGF, HGG)),
        gleam@erlang@process:subject(stage@internal@message:producer_message(HGG))}.

-type p_c_message(HGH, HGI) :: {upstream_msg,
        stage@internal@message:consumer_message(HGH)} |
    {downstream_msg, stage@internal@message:producer_message(HGI)} |
    {p_c_subscribe_msg,
        gleam@erlang@process:subject(stage@internal@message:producer_message(HGH)),
        stage@subscription:subscribe_opts(),
        gleam@erlang@process:subject({ok, nil} |
            {error, stage@error:stage_error()})}.

-type p_c_state(HGJ, HGK, HGL) :: {p_c_state,
        HGL,
        fun((list(HGJ), HGL) -> {list(HGK), HGL}),
        stage@subscription:consumer_registry(HGJ),
        stage@subscription:producer_registry(HGK),
        stage@dispatcher:dispatcher_type(HGK),
        stage@dispatcher:dispatcher_state(gleam@erlang@process:subject(stage@internal@message:producer_message(HGK))),
        stage@buffer:buffer(HGK),
        gleam@erlang@process:subject(stage@internal@message:consumer_message(HGJ)),
        gleam@erlang@process:subject(stage@internal@message:producer_message(HGK))}.

-file("src/stage/producer_consumer.gleam", 42).
?DOC(" Create a new producer-consumer config.\n").
-spec new_config(HGM, fun((list(HGN), HGM) -> {list(HGP), HGM})) -> producer_consumer_config(HGN, HGP, HGM).
new_config(State, Callback) ->
    {producer_consumer_config,
        State,
        Callback,
        demand_dispatcher,
        10000,
        drop_oldest}.

-file("src/stage/producer_consumer.gleam", 101).
?DOC(" Get the producer subject from a producer-consumer.\n").
-spec producer_subject(producer_consumer_stage(any(), HGV)) -> gleam@erlang@process:subject(stage@internal@message:producer_message(HGV)).
producer_subject(Stage) ->
    erlang:element(3, Stage).

-file("src/stage/producer_consumer.gleam", 156).
?DOC(" Subscribe this producer-consumer to an upstream producer.\n").
-spec subscribe(
    producer_consumer_stage(HHK, any()),
    gleam@erlang@process:subject(stage@internal@message:producer_message(HHK)),
    stage@subscription:subscribe_opts()
) -> {ok, nil} | {error, stage@error:stage_error()}.
subscribe(Stage, Producer_subject, Opts) ->
    gleam@otp@actor:call(
        erlang:element(2, Stage),
        5000,
        fun(Reply) -> {p_c_subscribe_msg, Producer_subject, Opts, Reply} end
    ).

-file("src/stage/producer_consumer.gleam", 328).
?DOC(" Build a selector that handles all message sources.\n").
-spec build_selector(
    p_c_state(HJJ, HJK, any()),
    stage@subscription:producer_registry(HJK)
) -> gleam@erlang@process:selector(p_c_message(HJJ, HJK)).
build_selector(State, Producer_registry) ->
    Subs = stage@subscription:producer_subscriptions(Producer_registry),
    Base_selector = begin
        _pipe = gleam_erlang_ffi:new_selector(),
        _pipe@1 = gleam@erlang@process:select_map(
            _pipe,
            erlang:element(9, State),
            fun(Field@0) -> {upstream_msg, Field@0} end
        ),
        gleam@erlang@process:select_map(
            _pipe@1,
            erlang:element(10, State),
            fun(Field@0) -> {downstream_msg, Field@0} end
        )
    end,
    gleam@list:fold(
        Subs,
        Base_selector,
        fun(Sel, Sub) ->
            gleam@erlang@process:select_map(
                Sel,
                erlang:element(3, Sub),
                fun(Field@0) -> {downstream_msg, Field@0} end
            )
        end
    ).

-file("src/stage/producer_consumer.gleam", 344).
?DOC(" Handle subscribing to an upstream producer.\n").
-spec handle_subscribe_upstream(
    p_c_state(HJT, HJU, HJV),
    gleam@erlang@process:subject(stage@internal@message:producer_message(HJT)),
    stage@subscription:subscribe_opts(),
    gleam@erlang@process:subject({ok, nil} | {error, stage@error:stage_error()})
) -> gleam@otp@actor:next(p_c_state(HJT, HJU, HJV), p_c_message(HJT, HJU)).
handle_subscribe_upstream(State, Producer_subject, Opts, Reply_to) ->
    Result = gleam@otp@actor:call(
        Producer_subject,
        5000,
        fun(Reply) -> {subscribe, erlang:element(9, State), Reply, none} end
    ),
    case Result of
        {ok, Ref} ->
            Initial_demand = case erlang:element(2, Opts) of
                {automatic, Max_demand, _} ->
                    Max_demand;

                manual ->
                    0
            end,
            Sub = {consumer_subscription,
                erlang:element(3, Ref),
                erlang:element(2, Ref),
                Opts,
                Initial_demand},
            Consumer_registry = stage@subscription:add_consumer_subscription(
                erlang:element(4, State),
                Sub
            ),
            case erlang:element(2, Opts) of
                {automatic, Max_demand@1, _} ->
                    gleam@erlang@process:send(
                        erlang:element(3, Ref),
                        {ask_demand, erlang:element(3, Ref), Max_demand@1}
                    );

                manual ->
                    nil
            end,
            gleam@erlang@process:send(Reply_to, {ok, nil}),
            gleam@otp@actor:continue(
                {p_c_state,
                    erlang:element(2, State),
                    erlang:element(3, State),
                    Consumer_registry,
                    erlang:element(5, State),
                    erlang:element(6, State),
                    erlang:element(7, State),
                    erlang:element(8, State),
                    erlang:element(9, State),
                    erlang:element(10, State)}
            );

        {error, Reason} ->
            gleam@erlang@process:send(
                Reply_to,
                {error, {subscribe_failed, Reason}}
            ),
            gleam@otp@actor:continue(State)
    end.

-file("src/stage/producer_consumer.gleam", 404).
?DOC(" Dispatch events downstream and buffer any that can't be sent.\n").
-spec dispatch_downstream(
    stage@dispatcher:dispatcher_type(HKL),
    stage@dispatcher:dispatcher_state(gleam@erlang@process:subject(stage@internal@message:producer_message(HKL))),
    list(HKL),
    stage@subscription:producer_registry(HKL),
    stage@buffer:buffer(HKL)
) -> {stage@dispatcher:dispatcher_state(gleam@erlang@process:subject(stage@internal@message:producer_message(HKL))),
    stage@buffer:buffer(HKL)}.
dispatch_downstream(
    Dispatcher_type,
    Dispatcher_state,
    Events,
    Producer_registry,
    Event_buffer
) ->
    Result = case Dispatcher_type of
        demand_dispatcher ->
            stage@dispatcher:dispatch_demand(Dispatcher_state, Events);

        broadcast_dispatcher ->
            stage@dispatcher:dispatch_broadcast(Dispatcher_state, Events);

        {partition_dispatcher, _, _} ->
            stage@dispatcher:dispatch_demand(Dispatcher_state, Events)
    end,
    stage@subscription:send_assignments(
        Producer_registry,
        erlang:element(2, Result)
    ),
    New_buffer = case erlang:element(3, Result) of
        [] ->
            Event_buffer;

        Remaining ->
            Buf@1 = case stage@buffer:add(Event_buffer, Remaining) of
                {buffer_ok, Buf, _} -> Buf;
                _assert_fail ->
                    erlang:error(#{gleam_error => let_assert,
                                message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                                file => <<?FILEPATH/utf8>>,
                                module => <<"stage/producer_consumer"/utf8>>,
                                function => <<"dispatch_downstream"/utf8>>,
                                line => 425,
                                value => _assert_fail,
                                start => 13771,
                                'end' => 13843,
                                pattern_start => 13782,
                                pattern_end => 13805})
            end,
            Buf@1
    end,
    {erlang:element(4, Result), New_buffer}.

-file("src/stage/producer_consumer.gleam", 187).
?DOC(" Handle events from upstream.\n").
-spec handle_upstream(
    p_c_state(HIH, HII, HIJ),
    stage@internal@message:consumer_message(HIH)
) -> gleam@otp@actor:next(p_c_state(HIH, HII, HIJ), p_c_message(HIH, HII)).
handle_upstream(State, Msg) ->
    case Msg of
        {send_events, Demand_subject, Events} ->
            {Out_events, New_user_state} = (erlang:element(3, State))(
                Events,
                erlang:element(2, State)
            ),
            {Consumer_registry, Ask_count} = stage@subscription:track_received_events(
                erlang:element(4, State),
                Demand_subject,
                erlang:length(Events)
            ),
            case Ask_count > 0 of
                true ->
                    gleam@erlang@process:send(
                        Demand_subject,
                        {ask_demand, Demand_subject, Ask_count}
                    );

                false ->
                    nil
            end,
            {Dispatcher_state, Event_buffer} = case Out_events of
                [] ->
                    {erlang:element(7, State), erlang:element(8, State)};

                _ ->
                    dispatch_downstream(
                        erlang:element(6, State),
                        erlang:element(7, State),
                        Out_events,
                        erlang:element(5, State),
                        erlang:element(8, State)
                    )
            end,
            gleam@otp@actor:continue(
                {p_c_state,
                    New_user_state,
                    erlang:element(3, State),
                    Consumer_registry,
                    erlang:element(5, State),
                    erlang:element(6, State),
                    Dispatcher_state,
                    Event_buffer,
                    erlang:element(9, State),
                    erlang:element(10, State)}
            )
    end.

-file("src/stage/producer_consumer.gleam", 237).
?DOC(" Handle downstream messages (subscribe, demand).\n").
-spec handle_downstream(
    p_c_state(HIV, HIW, HIX),
    stage@internal@message:producer_message(HIW)
) -> gleam@otp@actor:next(p_c_state(HIV, HIW, HIX), p_c_message(HIV, HIW)).
handle_downstream(State, Msg) ->
    case Msg of
        {subscribe, Consumer_subject, Reply_to, _} ->
            Demand_subject = gleam@erlang@process:new_subject(),
            Ref = {subscription_ref,
                Consumer_subject,
                Demand_subject,
                erlang:self()},
            Sub = {producer_subscription, Consumer_subject, Demand_subject},
            Producer_registry = stage@subscription:add_producer_subscription(
                erlang:element(5, State),
                Sub
            ),
            Dispatcher_state = stage@dispatcher:register(
                erlang:element(7, State),
                Demand_subject
            ),
            gleam@erlang@process:send(Reply_to, {ok, Ref}),
            Selector = build_selector(State, Producer_registry),
            New_state = {p_c_state,
                erlang:element(2, State),
                erlang:element(3, State),
                erlang:element(4, State),
                Producer_registry,
                erlang:element(6, State),
                Dispatcher_state,
                erlang:element(8, State),
                erlang:element(9, State),
                erlang:element(10, State)},
            _pipe = gleam@otp@actor:continue(New_state),
            gleam@otp@actor:with_selector(_pipe, Selector);

        {unsubscribe, Demand_subject@1} ->
            Producer_registry@1 = stage@subscription:remove_producer_subscription(
                erlang:element(5, State),
                Demand_subject@1
            ),
            Dispatcher_state@1 = stage@dispatcher:unregister(
                erlang:element(7, State),
                Demand_subject@1
            ),
            Selector@1 = build_selector(State, Producer_registry@1),
            New_state@1 = {p_c_state,
                erlang:element(2, State),
                erlang:element(3, State),
                erlang:element(4, State),
                Producer_registry@1,
                erlang:element(6, State),
                Dispatcher_state@1,
                erlang:element(8, State),
                erlang:element(9, State),
                erlang:element(10, State)},
            _pipe@1 = gleam@otp@actor:continue(New_state@1),
            gleam@otp@actor:with_selector(_pipe@1, Selector@1);

        {emit, _} ->
            gleam@otp@actor:continue(State);

        {ask_demand, Demand_subject@2, Count} ->
            Dispatcher_state@2 = stage@dispatcher:add_demand(
                erlang:element(7, State),
                Demand_subject@2,
                Count
            ),
            {Dispatcher_state@3, Event_buffer} = case stage@buffer:is_empty(
                erlang:element(8, State)
            ) of
                true ->
                    {Dispatcher_state@2, erlang:element(8, State)};

                false ->
                    Buffered_events = stage@buffer:to_list(
                        erlang:element(8, State)
                    ),
                    Cleared_buffer = stage@buffer:clear(
                        erlang:element(8, State)
                    ),
                    dispatch_downstream(
                        erlang:element(6, State),
                        Dispatcher_state@2,
                        Buffered_events,
                        erlang:element(5, State),
                        Cleared_buffer
                    )
            end,
            gleam@otp@actor:continue(
                {p_c_state,
                    erlang:element(2, State),
                    erlang:element(3, State),
                    erlang:element(4, State),
                    erlang:element(5, State),
                    erlang:element(6, State),
                    Dispatcher_state@3,
                    Event_buffer,
                    erlang:element(9, State),
                    erlang:element(10, State)}
            )
    end.

-file("src/stage/producer_consumer.gleam", 171).
?DOC(" Handle incoming messages.\n").
-spec handle_message(p_c_state(HHS, HHT, HHU), p_c_message(HHS, HHT)) -> gleam@otp@actor:next(p_c_state(HHS, HHT, HHU), p_c_message(HHS, HHT)).
handle_message(State, Msg) ->
    case Msg of
        {upstream_msg, Protocol_msg} ->
            handle_upstream(State, Protocol_msg);

        {downstream_msg, Producer_msg} ->
            handle_downstream(State, Producer_msg);

        {p_c_subscribe_msg, Producer_subject, Opts, Reply_to} ->
            handle_subscribe_upstream(State, Producer_subject, Opts, Reply_to)
    end.

-file("src/stage/producer_consumer.gleam", 108).
?DOC(" Start a producer-consumer stage.\n").
-spec start(producer_consumer_config(HHA, HHB, any())) -> {ok,
        producer_consumer_stage(HHA, HHB)} |
    {error, stage@error:stage_error()}.
start(Config) ->
    Builder = begin
        _pipe@6 = gleam@otp@actor:new_with_initialiser(
            5000,
            fun(Self_subject) ->
                Upstream_subject = gleam@erlang@process:new_subject(),
                Downstream_subject = gleam@erlang@process:new_subject(),
                Init_state = {p_c_state,
                    erlang:element(2, Config),
                    erlang:element(3, Config),
                    stage@subscription:new_consumer_registry(),
                    stage@subscription:new_producer_registry(),
                    erlang:element(4, Config),
                    stage@dispatcher:new_state(),
                    stage@buffer:new(
                        erlang:element(5, Config),
                        erlang:element(6, Config)
                    ),
                    Upstream_subject,
                    Downstream_subject},
                Selector = begin
                    _pipe = gleam_erlang_ffi:new_selector(),
                    _pipe@1 = gleam@erlang@process:select(_pipe, Self_subject),
                    _pipe@2 = gleam@erlang@process:select_map(
                        _pipe@1,
                        Upstream_subject,
                        fun(Field@0) -> {upstream_msg, Field@0} end
                    ),
                    gleam@erlang@process:select_map(
                        _pipe@2,
                        Downstream_subject,
                        fun(Field@0) -> {downstream_msg, Field@0} end
                    )
                end,
                _pipe@3 = gleam@otp@actor:initialised(Init_state),
                _pipe@4 = gleam@otp@actor:selecting(_pipe@3, Selector),
                _pipe@5 = gleam@otp@actor:returning(
                    _pipe@4,
                    {Self_subject, Downstream_subject}
                ),
                {ok, _pipe@5}
            end
        ),
        gleam@otp@actor:on_message(_pipe@6, fun handle_message/2)
    end,
    case gleam@otp@actor:start(Builder) of
        {ok, Started} ->
            {Self_subject@1, Ds_subject} = erlang:element(3, Started),
            {ok, {producer_consumer_stage, Self_subject@1, Ds_subject}};

        {error, _} ->
            {error, start_failed}
    end.
