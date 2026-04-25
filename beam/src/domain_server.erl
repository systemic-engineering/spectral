-module(domain_server).
-behaviour(gen_server).

%% Domain GenServer — receives action calls from compiled grammar modules.
%%
%% When a grammar compiles to EAF, each action becomes:
%%   action(Args) -> gen_server:call(Domain, {action, Args}).
%%
%% This module is the process registered as the domain atom that
%% receives those calls.
%%
%% Start with domain_server:start(<<"filesystem">>).
%% The process registers as the atom 'filesystem'.

-export([start/1, start_link/1, stop/1, kill/1, is_running/1, call_action/3, exec/4]).
-export([init/1, handle_call/3, handle_cast/2, handle_info/2, terminate/2]).

%% ── Public API ──────────────────────────────────────────────────────────────

%% Start a domain GenServer registered as the domain atom.
%% Returns {ok, nil} or {error, Reason} (Gleam-compatible).
start(Domain) when is_binary(Domain) ->
    Atom = binary_to_atom(Domain, utf8),
    case gen_server:start({local, Atom}, ?MODULE, #{domain => Domain}, []) of
        {ok, _Pid} -> {ok, nil};
        {error, Reason} -> {error, Reason}
    end.

%% Start linked — for supervisor.
start_link(Domain) when is_binary(Domain) ->
    Atom = binary_to_atom(Domain, utf8),
    gen_server:start_link({local, Atom}, ?MODULE, #{domain => Domain}, []).

%% Stop a running domain server.
%% Returns {ok, nil} or {error, Reason}.
stop(Domain) when is_binary(Domain) ->
    Atom = binary_to_atom(Domain, utf8),
    try
        gen_server:stop(Atom),
        {ok, nil}
    catch
        _:Reason -> {error, Reason}
    end.

%% Kill a domain server (for testing supervisor restart).
kill(Domain) when is_binary(Domain) ->
    Atom = binary_to_atom(Domain, utf8),
    case whereis(Atom) of
        undefined -> nil;
        Pid -> exit(Pid, kill), nil
    end.

%% Check if a domain server is running.
is_running(Domain) when is_binary(Domain) ->
    Atom = binary_to_atom(Domain, utf8),
    case whereis(Atom) of
        undefined -> false;
        _Pid -> true
    end.

%% Call an action on a domain server directly (for testing).
%% Returns {ok, Response} or {error, Reason}.
call_action(Domain, Action, Args) when is_binary(Domain), is_binary(Action) ->
    Atom = binary_to_atom(Domain, utf8),
    ActionAtom = binary_to_atom(Action, utf8),
    try
        gen_server:call(Atom, {ActionAtom, Args})
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.

%% Call exec on a domain server — the native primitive.
%% Module and Function are binaries, converted to atoms here.
%% Args is a list passed through to apply/3.
exec(Domain, Module, Function, Args)
  when is_binary(Domain), is_binary(Module), is_binary(Function), is_list(Args) ->
    Atom = binary_to_atom(Domain, utf8),
    M = binary_to_atom(Module, utf8),
    F = binary_to_atom(Function, utf8),
    try
        gen_server:call(Atom, {exec, {M, F, Args}})
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.

%% ── gen_server callbacks ────────────────────────────────────────────────────

init(#{domain := Domain} = _Args) ->
    {ok, #{domain => Domain, loaded_at => erlang:system_time(millisecond)}}.

%% exec — the one native primitive. Calls Erlang directly.
%% Args is {Module, Function, FunctionArgs} — an MFA.
handle_call({exec, {M, F, A}}, _From, State) when is_atom(M), is_atom(F), is_list(A) ->
    try
        Result = apply(M, F, A),
        {reply, {ok, Result}, State}
    catch
        Class:Reason ->
            Err = iolist_to_binary(io_lib:format("~p:~p", [Class, Reason])),
            {reply, {error, Err}, State}
    end;

%% Action dispatch: {ActionAtom, Args} from compiled module.
%% For non-exec actions, acknowledge. These are branch domains —
%% their implementation is grammar composition that chains down to exec.
handle_call({Action, Args}, _From, #{domain := Domain} = State) ->
    Reply = {ok, {Domain, Action, Args}},
    {reply, Reply, State};

handle_call(Msg, _From, State) ->
    Err = iolist_to_binary(io_lib:format("unknown call: ~p", [Msg])),
    {reply, {error, Err}, State}.

handle_cast(_Msg, State) ->
    {noreply, State}.

handle_info(_Msg, State) ->
    {noreply, State}.

terminate(_Reason, _State) ->
    ok.
