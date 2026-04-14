%% @doc Oracle transformer: producer_consumer that doubles each value.
%%
%% Implements the gen_stage behaviour as a producer_consumer that transforms
%% each incoming event by multiplying it by 2. Used as the second
%% transformation stage in the oracle pipeline.
%% @end
-module(oracle_transformer).

-behaviour(gen_stage).

-export([start_link/0,
         start_link/1,
         stop/1]).

-export([init/1,
         handle_events/3,
         handle_subscribe/4,
         handle_cancel/3,
         handle_info/2,
         terminate/2]).

%% --- API ---

start_link() ->
    start_link([]).

start_link(Opts) ->
    gen_stage:start_link(?MODULE, Opts, []).

stop(Pid) ->
    gen_stage:stop(Pid).

%% --- Callbacks ---

init(Opts) ->
    {producer_consumer, #{}, Opts}.

handle_events(Events, _From, State) ->
    Doubled = [E * 2 || E <- Events],
    {noreply, Doubled, State}.

handle_subscribe(_Kind, _Opts, _From, State) ->
    {automatic, State}.

handle_cancel(_Reason, _From, State) ->
    {noreply, [], State}.

handle_info(_Msg, State) ->
    {noreply, [], State}.

terminate(_Reason, _State) ->
    ok.
