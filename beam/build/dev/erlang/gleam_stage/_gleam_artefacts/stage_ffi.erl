%% FFI helpers for gleam_stage internals.
-module(stage_ffi).
-export([subject_owner/1,
         send_shutdown/1,
         send_kill/1,
         is_normal_exit_reason/1,
         is_process_alive/1]).

%% @doc Extract the owning PID from a Gleam Subject.
%% A Subject is represented as {subject, Pid, Ref} at the BEAM level.
-spec subject_owner({subject, pid(), reference()}) -> pid().
subject_owner({subject, Pid, _Ref}) -> Pid.

%% @doc Send a shutdown exit signal to a process.
-spec send_shutdown(pid()) -> nil.
send_shutdown(Pid) ->
    exit(Pid, shutdown),
    nil.

%% @doc Send a kill exit signal to a process (untrappable).
-spec send_kill(pid()) -> nil.
send_kill(Pid) ->
    exit(Pid, kill),
    nil.

%% @doc Check if an exit reason is "normal" per OTP conventions.
%% Normal: normal, shutdown, {shutdown, _}
-spec is_normal_exit_reason(term()) -> boolean().
is_normal_exit_reason(normal) -> true;
is_normal_exit_reason(shutdown) -> true;
is_normal_exit_reason({shutdown, _}) -> true;
is_normal_exit_reason(_) -> false.

%% @doc Check if a process is alive.
-spec is_process_alive(pid()) -> boolean().
is_process_alive(Pid) -> erlang:is_process_alive(Pid).
