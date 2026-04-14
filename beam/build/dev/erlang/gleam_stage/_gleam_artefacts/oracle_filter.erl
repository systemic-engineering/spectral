%% @doc Oracle filter: producer_consumer that keeps only even numbers.
%%
%% Implements the gen_stage behaviour as a producer_consumer that filters
%% incoming events, passing through only even integers. Used as the first
%% transformation stage in the oracle pipeline.
%% @end
-module(oracle_filter).

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
    Filtered = [E || E <- Events, E rem 2 =:= 0],
    {noreply, Filtered, State}.

handle_subscribe(_Kind, _Opts, _From, State) ->
    {automatic, State}.

handle_cancel(_Reason, _From, State) ->
    {noreply, [], State}.

handle_info(_Msg, State) ->
    {noreply, [], State}.

terminate(_Reason, _State) ->
    ok.
