-module(stage@consumer).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/consumer.gleam").
-export([new_config/2, subscribe/3, ask/3, get_state/2, start/1]).
-export_type([consumer_config/2, consumer_stage/1, consumer_msg/1, consumer_state/2]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Consumer stage implementation.\n"
    "\n"
    " A consumer subscribes to producers and receives events. It tracks\n"
    " demand and automatically requests more events when demand drops\n"
    " below the configured minimum (in automatic mode).\n"
    "\n"
    " Cancellation: the consumer monitors its producer. When the producer\n"
    " dies, the cancel mode determines whether the consumer exits:\n"
    "   Permanent  — always exit\n"
    "   Transient  — exit only on abnormal exit\n"
    "   Temporary  — never exit (remove subscription, continue)\n"
).

-type consumer_config(FPP, FPQ) :: {consumer_config,
        FPQ,
        fun((list(FPP), FPQ) -> FPQ)}.

-type consumer_stage(FPR) :: {consumer_stage,
        gleam@erlang@process:subject(consumer_msg(FPR))}.

-type consumer_msg(FPS) :: {protocol_msg,
        stage@internal@message:consumer_message(FPS)} |
    {subscribe_msg,
        gleam@erlang@process:subject(stage@internal@message:producer_message(FPS)),
        stage@subscription:subscribe_opts(),
        gleam@erlang@process:subject({ok, nil} |
            {error, stage@error:stage_error()})} |
    {ask_msg,
        gleam@erlang@process:subject(stage@internal@message:producer_message(FPS)),
        integer()} |
    {get_state_msg, gleam@erlang@process:subject(gleam@dynamic:dynamic_())} |
    {producer_down, gleam@erlang@process:down()}.

-type consumer_state(FPT, FPU) :: {consumer_state,
        FPU,
        fun((list(FPT), FPU) -> FPU),
        stage@subscription:consumer_registry(FPT),
        gleam@erlang@process:subject(stage@internal@message:consumer_message(FPT)),
        gleam@erlang@process:subject(consumer_msg(FPT))}.

-file("src/stage/consumer.gleam", 48).
?DOC(" Create a new consumer config.\n").
-spec new_config(FPX, fun((list(FPY), FPX) -> FPX)) -> consumer_config(FPY, FPX).
new_config(State, Callback) ->
    {consumer_config, State, Callback}.

-file("src/stage/consumer.gleam", 130).
?DOC(" Subscribe this consumer to a producer.\n").
-spec subscribe(
    consumer_stage(FQJ),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FQJ)),
    stage@subscription:subscribe_opts()
) -> {ok, nil} | {error, stage@error:stage_error()}.
subscribe(Stage, Producer_subject, Opts) ->
    gleam@otp@actor:call(
        erlang:element(2, Stage),
        5000,
        fun(Reply) -> {subscribe_msg, Producer_subject, Opts, Reply} end
    ).

-file("src/stage/consumer.gleam", 145).
?DOC(" Manually ask for events (used with Manual demand mode).\n").
-spec ask(
    consumer_stage(FQP),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FQP)),
    integer()
) -> nil.
ask(Stage, Producer_subject, Count) ->
    gleam@erlang@process:send(
        erlang:element(2, Stage),
        {ask_msg, Producer_subject, Count}
    ).

-file("src/stage/consumer.gleam", 158).
?DOC(
    " Get the current user state from the consumer.\n"
    " The caller must know the actual state type — this uses dynamic coercion.\n"
).
-spec get_state(consumer_stage(any()), integer()) -> any().
get_state(Stage, Timeout) ->
    Dyn_result = gleam@otp@actor:call(
        erlang:element(2, Stage),
        Timeout,
        fun(Reply) -> {get_state_msg, Reply} end
    ),
    gleam_erlang_ffi:identity(Dyn_result).

-file("src/stage/consumer.gleam", 190).
?DOC(" Handle events arriving from a producer.\n").
-spec handle_events(
    consumer_state(FRG, FRH),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FRG)),
    list(FRG)
) -> gleam@otp@actor:next(consumer_state(FRG, FRH), consumer_msg(FRG)).
handle_events(State, Demand_subject, Events) ->
    New_user_state = (erlang:element(3, State))(
        Events,
        erlang:element(2, State)
    ),
    {Registry, Ask_count} = stage@subscription:track_received_events(
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
    gleam@otp@actor:continue(
        {consumer_state,
            New_user_state,
            erlang:element(3, State),
            Registry,
            erlang:element(5, State),
            erlang:element(6, State)}
    ).

-file("src/stage/consumer.gleam", 218).
?DOC(" Handle a subscribe request.\n").
-spec handle_subscribe(
    consumer_state(FRS, FRT),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FRS)),
    stage@subscription:subscribe_opts(),
    gleam@erlang@process:subject({ok, nil} | {error, stage@error:stage_error()})
) -> gleam@otp@actor:next(consumer_state(FRS, FRT), consumer_msg(FRS)).
handle_subscribe(State, Producer_subject, Opts, Reply_to) ->
    Result = gleam@otp@actor:call(
        Producer_subject,
        5000,
        fun(Reply) ->
            {subscribe,
                erlang:element(5, State),
                Reply,
                erlang:element(4, Opts)}
        end
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
            Registry = stage@subscription:add_consumer_subscription(
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
            Monitor = gleam@erlang@process:monitor(erlang:element(4, Ref)),
            Selector = begin
                _pipe = gleam_erlang_ffi:new_selector(),
                _pipe@1 = gleam@erlang@process:select(
                    _pipe,
                    erlang:element(6, State)
                ),
                _pipe@2 = gleam@erlang@process:select_map(
                    _pipe@1,
                    erlang:element(5, State),
                    fun(Field@0) -> {protocol_msg, Field@0} end
                ),
                gleam@erlang@process:select_specific_monitor(
                    _pipe@2,
                    Monitor,
                    fun(Field@0) -> {producer_down, Field@0} end
                )
            end,
            gleam@erlang@process:send(Reply_to, {ok, nil}),
            _pipe@3 = gleam@otp@actor:continue(
                {consumer_state,
                    erlang:element(2, State),
                    erlang:element(3, State),
                    Registry,
                    erlang:element(5, State),
                    erlang:element(6, State)}
            ),
            gleam@otp@actor:with_selector(_pipe@3, Selector);

        {error, Reason} ->
            gleam@erlang@process:send(
                Reply_to,
                {error, {subscribe_failed, Reason}}
            ),
            gleam@otp@actor:continue(State)
    end.

-file("src/stage/consumer.gleam", 286).
?DOC(" Handle producer process going down — apply cancellation policy.\n").
-spec handle_producer_down(
    consumer_state(FSG, FSH),
    gleam@erlang@process:down()
) -> gleam@otp@actor:next(consumer_state(FSG, FSH), consumer_msg(FSG)).
handle_producer_down(State, Down) ->
    Subs = stage@subscription:consumer_subscriptions(erlang:element(4, State)),
    Cancel_mode = case Subs of
        [Sub | _] ->
            erlang:element(3, erlang:element(4, Sub));

        [] ->
            temporary
    end,
    Is_normal = case Down of
        {process_down, _, _, normal} ->
            true;

        {process_down, _, _, killed} ->
            false;

        {process_down, _, _, {abnormal, Reason}} ->
            stage_ffi:is_normal_exit_reason(Reason);

        {port_down, _, _, _} ->
            true
    end,
    case Cancel_mode of
        permanent ->
            gleam@otp@actor:stop();

        transient ->
            case Is_normal of
                true ->
                    gleam@otp@actor:continue(
                        {consumer_state,
                            erlang:element(2, State),
                            erlang:element(3, State),
                            stage@subscription:new_consumer_registry(),
                            erlang:element(5, State),
                            erlang:element(6, State)}
                    );

                false ->
                    gleam@otp@actor:stop()
            end;

        temporary ->
            gleam@otp@actor:continue(
                {consumer_state,
                    erlang:element(2, State),
                    erlang:element(3, State),
                    stage@subscription:new_consumer_registry(),
                    erlang:element(5, State),
                    erlang:element(6, State)}
            )
    end.

-file("src/stage/consumer.gleam", 333).
?DOC(" Handle a manual ask request.\n").
-spec handle_ask(
    consumer_state(FSP, FSQ),
    gleam@erlang@process:subject(stage@internal@message:producer_message(FSP)),
    integer()
) -> gleam@otp@actor:next(consumer_state(FSP, FSQ), consumer_msg(FSP)).
handle_ask(State, _, Count) ->
    Subs = stage@subscription:consumer_subscriptions(erlang:element(4, State)),
    gleam@list:each(
        Subs,
        fun(Sub) ->
            gleam@erlang@process:send(
                erlang:element(2, Sub),
                {ask_demand, erlang:element(2, Sub), Count}
            )
        end
    ),
    gleam@otp@actor:continue(State).

-file("src/stage/consumer.gleam", 170).
?DOC(" Handle an incoming message to the consumer.\n").
-spec handle_message(consumer_state(FQW, FQX), consumer_msg(FQW)) -> gleam@otp@actor:next(consumer_state(FQW, FQX), consumer_msg(FQW)).
handle_message(State, Msg) ->
    case Msg of
        {protocol_msg, {send_events, Demand_subject, Events}} ->
            handle_events(State, Demand_subject, Events);

        {subscribe_msg, Producer_subject, Opts, Reply_to} ->
            handle_subscribe(State, Producer_subject, Opts, Reply_to);

        {ask_msg, Producer_subject@1, Count} ->
            handle_ask(State, Producer_subject@1, Count);

        {get_state_msg, Reply_to@1} ->
            gleam@erlang@process:send(
                Reply_to@1,
                gleam_erlang_ffi:identity(erlang:element(2, State))
            ),
            gleam@otp@actor:continue(State);

        {producer_down, Down} ->
            handle_producer_down(State, Down)
    end.

-file("src/stage/consumer.gleam", 95).
?DOC(" Start a consumer stage.\n").
-spec start(consumer_config(FQC, any())) -> {ok, consumer_stage(FQC)} |
    {error, stage@error:stage_error()}.
start(Config) ->
    Builder = begin
        _pipe@5 = gleam@otp@actor:new_with_initialiser(
            5000,
            fun(Self_subject) ->
                Protocol_subject = gleam@erlang@process:new_subject(),
                Init_state = {consumer_state,
                    erlang:element(2, Config),
                    erlang:element(3, Config),
                    stage@subscription:new_consumer_registry(),
                    Protocol_subject,
                    Self_subject},
                Selector = begin
                    _pipe = gleam_erlang_ffi:new_selector(),
                    _pipe@1 = gleam@erlang@process:select(_pipe, Self_subject),
                    gleam@erlang@process:select_map(
                        _pipe@1,
                        Protocol_subject,
                        fun(Field@0) -> {protocol_msg, Field@0} end
                    )
                end,
                _pipe@2 = gleam@otp@actor:initialised(Init_state),
                _pipe@3 = gleam@otp@actor:selecting(_pipe@2, Selector),
                _pipe@4 = gleam@otp@actor:returning(_pipe@3, Self_subject),
                {ok, _pipe@4}
            end
        ),
        gleam@otp@actor:on_message(_pipe@5, fun handle_message/2)
    end,
    case gleam@otp@actor:start(Builder) of
        {ok, Started} ->
            {ok, {consumer_stage, erlang:element(3, Started)}};

        {error, _} ->
            {error, start_failed}
    end.
