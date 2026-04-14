-module(conversation@grammar).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/grammar.gleam").
-export([domain/1, has_variant/3, from_source/1]).
-export_type([grammar/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Grammar — type vocabulary extraction from .conv source.\n"
    "\n"
    " Lightweight string parsing to extract grammar blocks and their type\n"
    " definitions. No full parser — just enough to build a type vocabulary\n"
    " for assertion evaluation.\n"
).

-type grammar() :: {grammar,
        binary(),
        gleam@dict:dict(binary(), gleam@set:set(binary()))}.

-file("src/conversation/grammar.gleam", 18).
?DOC(" Extract the domain name from a grammar.\n").
-spec domain(grammar()) -> binary().
domain(Grammar) ->
    erlang:element(2, Grammar).

-file("src/conversation/grammar.gleam", 24).
?DOC(
    " Check if a grammar contains a specific variant under a type path.\n"
    " Empty type_path checks the default (unnamed) type.\n"
).
-spec has_variant(grammar(), binary(), binary()) -> boolean().
has_variant(Grammar, Type_path, Variant) ->
    case gleam_stdlib:map_get(erlang:element(3, Grammar), Type_path) of
        {ok, Variants} ->
            gleam@set:contains(Variants, Variant);

        {error, _} ->
            false
    end.

-file("src/conversation/grammar.gleam", 47).
?DOC(
    " Find the `grammar @name {` or `abstract grammar @name {` line\n"
    " and return the domain name + remaining lines.\n"
).
-spec find_grammar_start(list(binary())) -> {ok, {binary(), list(binary())}} |
    {error, binary()}.
find_grammar_start(Lines) ->
    case Lines of
        [] ->
            {error, <<"No grammar block found"/utf8>>};

        [Line | Rest] ->
            Trimmed = gleam@string:trim(Line),
            Grammar_rest = case gleam_stdlib:string_starts_with(
                Trimmed,
                <<"abstract grammar @"/utf8>>
            ) of
                true ->
                    {ok, gleam@string:drop_start(Trimmed, 18)};

                false ->
                    case gleam_stdlib:string_starts_with(
                        Trimmed,
                        <<"grammar @"/utf8>>
                    ) of
                        true ->
                            {ok, gleam@string:drop_start(Trimmed, 9)};

                        false ->
                            {error, nil}
                    end
            end,
            case Grammar_rest of
                {ok, After_at} ->
                    Without_brace = begin
                        _pipe = After_at,
                        _pipe@1 = gleam@string:replace(
                            _pipe,
                            <<" {"/utf8>>,
                            <<""/utf8>>
                        ),
                        _pipe@2 = gleam@string:replace(
                            _pipe@1,
                            <<"{"/utf8>>,
                            <<""/utf8>>
                        ),
                        gleam@string:trim(_pipe@2)
                    end,
                    Domain_name = case gleam@string:split_once(
                        Without_brace,
                        <<" extends "/utf8>>
                    ) of
                        {ok, {Name, _}} ->
                            gleam@string:trim(Name);

                        {error, _} ->
                            Without_brace
                    end,
                    {ok, {Domain_name, Rest}};

                {error, _} ->
                    find_grammar_start(Rest)
            end
    end.

-file("src/conversation/grammar.gleam", 99).
?DOC(" Count occurrences of a character in a string.\n").
-spec count_char(binary(), binary()) -> integer().
count_char(S, Char) ->
    Parts = gleam@string:split(S, Char),
    erlang:length(Parts) - 1.

-file("src/conversation/grammar.gleam", 82).
?DOC(" Collect lines inside the grammar block, tracking brace depth.\n").
-spec collect_grammar_body(list(binary()), integer()) -> list(binary()).
collect_grammar_body(Lines, Depth) ->
    case Lines of
        [] ->
            [];

        _ when Depth =< 0 ->
            [];

        [Line | Rest] ->
            Opens = count_char(Line, <<"{"/utf8>>),
            Closes = count_char(Line, <<"}"/utf8>>),
            New_depth = (Depth + Opens) - Closes,
            case New_depth =< 0 of
                true ->
                    [];

                false ->
                    [Line | collect_grammar_body(Rest, New_depth)]
            end
    end.

-file("src/conversation/grammar.gleam", 123).
?DOC(
    " Parse a single type line.\n"
    " `type = a | b | c` → (\"\", {\"a\", \"b\", \"c\"})\n"
    " `type name = a | b` → (\"name\", {\"a\", \"b\"})\n"
).
-spec parse_type_line(binary()) -> {ok, {binary(), gleam@set:set(binary())}} |
    {error, nil}.
parse_type_line(Line) ->
    After_type = begin
        _pipe = gleam@string:drop_start(Line, 4),
        gleam@string:trim_start(_pipe)
    end,
    case gleam@string:split_once(After_type, <<"="/utf8>>) of
        {ok, {Before_eq, After_eq}} ->
            Name = gleam@string:trim(Before_eq),
            Variants = begin
                _pipe@1 = After_eq,
                _pipe@2 = gleam@string:split(_pipe@1, <<"|"/utf8>>),
                _pipe@3 = gleam@list:map(_pipe@2, fun gleam@string:trim/1),
                _pipe@4 = gleam@list:filter(
                    _pipe@3,
                    fun(S) -> S /= <<""/utf8>> end
                ),
                gleam@set:from_list(_pipe@4)
            end,
            {ok, {Name, Variants}};

        {error, _} ->
            {error, nil}
    end.

-file("src/conversation/grammar.gleam", 105).
?DOC(" Parse type definition lines into a Dict(String, Set(String)).\n").
-spec parse_type_lines(list(binary())) -> gleam@dict:dict(binary(), gleam@set:set(binary())).
parse_type_lines(Lines) ->
    gleam@list:fold(
        Lines,
        maps:new(),
        fun(Acc, Line) ->
            Trimmed = gleam@string:trim(Line),
            case gleam_stdlib:string_starts_with(Trimmed, <<"type"/utf8>>) of
                true ->
                    case parse_type_line(Trimmed) of
                        {ok, {Name, Variants}} ->
                            gleam@dict:insert(Acc, Name, Variants);

                        {error, _} ->
                            Acc
                    end;

                false ->
                    Acc
            end
        end
    ).

-file("src/conversation/grammar.gleam", 33).
?DOC(
    " Parse a grammar block from .conv source text.\n"
    " Finds `grammar @name { ... }`, extracts type definitions.\n"
).
-spec from_source(binary()) -> {ok, grammar()} | {error, binary()}.
from_source(Source) ->
    Lines = gleam@string:split(Source, <<"\n"/utf8>>),
    case find_grammar_start(Lines) of
        {ok, {Domain_name, Rest}} ->
            Type_lines = collect_grammar_body(Rest, 1),
            Types = parse_type_lines(Type_lines),
            {ok, {grammar, Domain_name, Types}};

        {error, Msg} ->
            {error, Msg}
    end.
