%% FFI helper for litmus tests.
%% Oracle stages use start_link — their shutdown exit signals kill
%% the untrapped test process. This wrapper traps exits for the duration
%% of the oracle call.
-module(litmus_ffi).
-export([run_oracle/1]).

-spec run_oracle(fun(() -> T)) -> T.
run_oracle(Fun) ->
    Old = process_flag(trap_exit, true),
    Result = Fun(),
    flush_exits(),
    process_flag(trap_exit, Old),
    Result.

flush_exits() ->
    receive
        {'EXIT', _, _} -> flush_exits()
    after 0 ->
        ok
    end.
