%% @doc Oracle producer: generates a sequence of integers on demand.
%%
%% Implements the gen_stage behaviour as a producer that emits sequential
%% integers starting from a configurable offset. Used as the source stage
%% in the oracle pipeline.
%%
%% Options passed through init:
%%   {Counter, Opts} where Counter is the starting integer and Opts are
%%   gen_stage producer options (dispatcher, buffer_size, etc.)
%% @end
-module(oracle_producer).

-behaviour(gen_stage).

-export([start_link/1,
         start_link/2,
         stop/1]).

-export([init/1,
         handle_demand/2,
         handle_subscribe/4,
         handle_cancel/3,
         handle_info/2,
         terminate/2]).

%% --- API ---

start_link(Counter) ->
    start_link(Counter, []).

start_link(Counter, Opts) ->
    gen_stage:start_link(?MODULE, {Counter, Opts}, []).

stop(Pid) ->
    gen_stage:stop(Pid).

%% --- Callbacks ---

init({Counter, Opts}) ->
    {producer, Counter, Opts}.

handle_demand(Demand, Counter) when Demand > 0 ->
    Events = lists:seq(Counter, Counter + Demand - 1),
    {noreply, Events, Counter + Demand}.

handle_subscribe(consumer, _Opts, _From, State) ->
    {automatic, State}.

handle_cancel(_Reason, _From, State) ->
    {noreply, [], State}.

handle_info({emit, Events}, State) ->
    %% Emit events directly — used for buffer testing.
    %% When in accumulate mode or with no consumers, these get buffered.
    {noreply, Events, State};
handle_info(_Msg, State) ->
    {noreply, [], State}.

terminate(_Reason, _State) ->
    ok.
