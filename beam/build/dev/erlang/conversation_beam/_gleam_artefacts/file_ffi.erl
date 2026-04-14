-module(file_ffi).
-export([read_file/1]).

%% Read a UTF-8 text file from disk.
%% Validates that contents are valid UTF-8 before returning as a Gleam String.
%% Returns {ok, String} or {error, Reason}.
read_file(Path) when is_binary(Path) ->
    case file:read_file(Path) of
        {ok, Bytes} ->
            case unicode:characters_to_binary(Bytes, utf8, utf8) of
                {error, _, _} -> {error, <<"not valid UTF-8">>};
                {incomplete, _, _} -> {error, <<"incomplete UTF-8">>};
                Valid when is_binary(Valid) -> {ok, Valid}
            end;
        {error, Reason} ->
            {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.
