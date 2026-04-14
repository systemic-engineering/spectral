-module(coincidence_server).
-behaviour(gen_server).

%% @coincidence domain server — routes measurement action calls to NIFs.
%%
%% Unlike the generic domain_server, this server has custom handlers
%% that dispatch directly to conversation_nif property-checking functions.

-export([start/0, start_link/0, stop/0, is_running/0]).
-export([init/1, handle_call/3, handle_cast/2, handle_info/2, terminate/2]).

%% ── Public API ──────────────────────────────────────────────────────────────

start() ->
    case gen_server:start({local, coincidence}, ?MODULE, #{}, []) of
        {ok, _Pid} -> {ok, nil};
        {error, Reason} -> {error, Reason}
    end.

start_link() ->
    gen_server:start_link({local, coincidence}, ?MODULE, #{}, []).

stop() ->
    try
        gen_server:stop(coincidence),
        {ok, nil}
    catch
        _:Reason -> {error, Reason}
    end.

is_running() ->
    case whereis(coincidence) of
        undefined -> false;
        _Pid -> true
    end.

%% ── gen_server callbacks ────────────────────────────────────────────────────

init(_Args) ->
    {ok, #{domain => <<"coincidence">>}}.

%% The check action: dispatches by property name to the appropriate NIF.
%% Args = [PropertyName :: binary(), GrammarSource :: binary()]
handle_call({check, [Name, Source]}, _From, State) ->
    Result = conversation_nif:check_property(Source, Name),
    {reply, Result, State};

%% Named measurement actions — each calls its specific NIF.
handle_call({shannon_equivalence, [Source]}, _From, State) ->
    Result = conversation_nif:check_shannon_equivalence(Source),
    {reply, Result, State};

handle_call({connected, [Source]}, _From, State) ->
    Result = conversation_nif:check_connected(Source),
    {reply, Result, State};

handle_call({bipartite, [Source]}, _From, State) ->
    Result = conversation_nif:check_bipartite(Source),
    {reply, Result, State};

handle_call({exhaustive, [Source]}, _From, State) ->
    Result = conversation_nif:check_exhaustive(Source),
    {reply, Result, State};

%% Fallback for unknown actions.
handle_call({Action, Args}, _From, State) ->
    Reply = {ok, {<<"coincidence">>, Action, Args}},
    {reply, Reply, State};

handle_call(Msg, _From, State) ->
    Err = iolist_to_binary(io_lib:format("unknown call: ~p", [Msg])),
    {reply, {error, Err}, State}.

handle_cast(_Msg, State) -> {noreply, State}.
handle_info(_Msg, State) -> {noreply, State}.
terminate(_Reason, _State) -> ok.
