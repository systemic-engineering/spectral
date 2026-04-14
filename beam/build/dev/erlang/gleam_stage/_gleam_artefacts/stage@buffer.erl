-module(stage@buffer).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/buffer.gleam").
-export([new/2, add/2, take/2, size/1, is_empty/1, to_list/1, clear/1]).
-export_type([overflow_strategy/0, buffer/1, buffer_result/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Event buffer with overflow strategies for stage producers.\n"
    "\n"
    " Buffers events when there is no downstream demand. Supports configurable\n"
    " maximum size and overflow strategies. Pure functions, no processes.\n"
).

-type overflow_strategy() :: drop_oldest | drop_newest | error.

-type buffer(FGF) :: {buffer,
        list(FGF),
        integer(),
        integer(),
        overflow_strategy()}.

-type buffer_result(FGG) :: {buffer_ok, buffer(FGG), integer()} |
    {buffer_overflow, buffer(FGG), list(FGG)}.

-file("src/stage/buffer.gleam", 37).
?DOC(" Create a new empty buffer with the given maximum size and overflow strategy.\n").
-spec new(integer(), overflow_strategy()) -> buffer(any()).
new(Max_size, Overflow) ->
    {buffer, [], 0, Max_size, Overflow}.

-file("src/stage/buffer.gleam", 42).
?DOC(" Add events to the buffer, applying the overflow strategy if necessary.\n").
-spec add(buffer(FGJ), list(FGJ)) -> buffer_result(FGJ).
add(Buffer, Events) ->
    Incoming_count = erlang:length(Events),
    New_size = erlang:element(3, Buffer) + Incoming_count,
    case New_size =< erlang:element(4, Buffer) of
        true ->
            {buffer_ok,
                {buffer,
                    lists:append(erlang:element(2, Buffer), Events),
                    New_size,
                    erlang:element(4, Buffer),
                    erlang:element(5, Buffer)},
                0};

        false ->
            Overflow_count = New_size - erlang:element(4, Buffer),
            case erlang:element(5, Buffer) of
                drop_oldest ->
                    All_events = lists:append(erlang:element(2, Buffer), Events),
                    Kept = gleam@list:drop(All_events, Overflow_count),
                    {buffer_ok,
                        {buffer,
                            Kept,
                            erlang:element(4, Buffer),
                            erlang:element(4, Buffer),
                            erlang:element(5, Buffer)},
                        Overflow_count};

                drop_newest ->
                    Space = erlang:element(4, Buffer) - erlang:element(
                        3,
                        Buffer
                    ),
                    Kept_new = gleam@list:take(Events, Space),
                    Dropped = Incoming_count - Space,
                    {buffer_ok,
                        {buffer,
                            lists:append(erlang:element(2, Buffer), Kept_new),
                            erlang:element(4, Buffer),
                            erlang:element(4, Buffer),
                            erlang:element(5, Buffer)},
                        Dropped};

                error ->
                    {buffer_overflow, Buffer, Events}
            end
    end.

-file("src/stage/buffer.gleam", 95).
?DOC(
    " Take up to `count` events from the front of the buffer.\n"
    " Returns the taken events and the updated buffer.\n"
).
-spec take(buffer(FGN), integer()) -> {list(FGN), buffer(FGN)}.
take(Buffer, Count) ->
    Taken = gleam@list:take(erlang:element(2, Buffer), Count),
    Taken_count = erlang:length(Taken),
    Remaining = gleam@list:drop(erlang:element(2, Buffer), Taken_count),
    New_size = erlang:element(3, Buffer) - Taken_count,
    {Taken,
        {buffer,
            Remaining,
            New_size,
            erlang:element(4, Buffer),
            erlang:element(5, Buffer)}}.

-file("src/stage/buffer.gleam", 104).
?DOC(" Get the current number of events in the buffer.\n").
-spec size(buffer(any())) -> integer().
size(Buffer) ->
    erlang:element(3, Buffer).

-file("src/stage/buffer.gleam", 109).
?DOC(" Check if the buffer is empty.\n").
-spec is_empty(buffer(any())) -> boolean().
is_empty(Buffer) ->
    erlang:element(3, Buffer) =:= 0.

-file("src/stage/buffer.gleam", 114).
?DOC(" Get all events in the buffer without removing them.\n").
-spec to_list(buffer(FGV)) -> list(FGV).
to_list(Buffer) ->
    erlang:element(2, Buffer).

-file("src/stage/buffer.gleam", 119).
?DOC(" Clear all events from the buffer.\n").
-spec clear(buffer(FGY)) -> buffer(FGY).
clear(Buffer) ->
    {buffer, [], 0, erlang:element(4, Buffer), erlang:element(5, Buffer)}.
