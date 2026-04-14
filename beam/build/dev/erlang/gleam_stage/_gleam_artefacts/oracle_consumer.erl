%% @doc Oracle consumer: collects events into an ordered list.
%%
%% Implements the gen_stage behaviour as a consumer that accumulates all
%% received events. Supports both automatic and manual demand modes.
%% The collected results can be retrieved via get_events/1.
%%
%% Init argument:
%%   {OwnerPid, Opts} — OwnerPid receives a message when events arrive
%%                       (used for synchronization). Opts are gen_stage
%%                       consumer options.
%%   {manual, OwnerPid, Opts} — Same but with manual demand.
%% @end
-module(oracle_consumer).

-behaviour(gen_stage).

-export([start_link/2,
         start_link/3,
         get_events/1,
         ask/3,
         stop/1]).

-export([init/1,
         handle_events/3,
         handle_subscribe/4,
         handle_cancel/3,
         handle_call/3,
         handle_info/2,
         terminate/2]).

%% --- API ---

start_link(Owner, Opts) ->
    gen_stage:start_link(?MODULE, {automatic, Owner, Opts}, []).

start_link(DemandMode, Owner, Opts) ->
    gen_stage:start_link(?MODULE, {DemandMode, Owner, Opts}, []).

get_events(Pid) ->
    gen_stage:call(Pid, get_events).

%% @doc Ask for N events from a producer. Must be called externally;
%% the ask is executed inside the consumer's process context.
ask(Pid, ProducerFrom, N) ->
    gen_stage:call(Pid, {ask, ProducerFrom, N}).

stop(Pid) ->
    gen_stage:stop(Pid).

%% --- Callbacks ---

init({DemandMode, Owner, Opts}) ->
    {consumer, #{owner => Owner, events => [], demand_mode => DemandMode}, Opts}.

handle_events(Events, _From, #{events := Acc, owner := Owner} = State) ->
    NewAcc = Acc ++ Events,
    Owner ! {oracle_consumer_batch, self(), Events},
    {noreply, [], State#{events := NewAcc}}.

handle_subscribe(producer, _Opts, _From, #{demand_mode := manual} = State) ->
    {manual, State};
handle_subscribe(producer, _Opts, _From, State) ->
    {automatic, State}.

handle_cancel(_Reason, _From, State) ->
    {noreply, [], State}.

handle_call(get_events, _From, #{events := Events} = State) ->
    {reply, lists:sort(Events), [], State};
handle_call({ask, ProducerFrom, N}, _From, State) ->
    gen_stage:ask(ProducerFrom, N),
    {reply, ok, [], State}.

handle_info(_Msg, State) ->
    {noreply, [], State}.

terminate(_Reason, _State) ->
    ok.
