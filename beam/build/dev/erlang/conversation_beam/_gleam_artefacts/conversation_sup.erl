-module(conversation_sup).
-behaviour(supervisor).

%% DEPRECATED: Use conversation/supervisor.gleam + conversation/garden.gleam instead.
%%
%% This Erlang supervisor is retained for backwards compatibility with the
%% imperative boot path (compiler.start() + boot.boot_from_files()).
%% New code should use the supervised boot path:
%%   conv_sup.start(compiler_name, garden_name) + boot.supervised_boot_from_files()
%%
%% Original: simple_one_for_one supervisor for domain servers.
%% Each domain server is a child started dynamically via start_domain/1.
%% If a domain server crashes, the supervisor restarts it.

-export([start_link/0, start_domain/1, init/1]).

%% Start the supervisor. Registered as 'conversation_sup'.
start_link() ->
    supervisor:start_link({local, ?MODULE}, ?MODULE, []).

%% Dynamically start a supervised domain server.
%% Returns {ok, Pid} or {error, Reason}.
start_domain(Domain) when is_binary(Domain) ->
    supervisor:start_child(?MODULE, [Domain]).

init([]) ->
    ChildSpec = #{
        id => domain_server,
        start => {domain_server, start_link, []},
        restart => transient,
        type => worker
    },
    {ok, {#{strategy => simple_one_for_one, intensity => 5, period => 60}, [ChildSpec]}}.
