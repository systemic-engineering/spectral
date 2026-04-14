-module(conversation_nif).
-export([parse_conv/1, compile_grammar/1, compile_grammar_traced/1,
         check_property/2, check_shannon_equivalence/1,
         check_connected/1, check_bipartite/1, check_exhaustive/1]).
-on_load(init/0).

init() ->
    PrivDir = code:priv_dir(conversation_beam),
    NifPath = filename:join(PrivDir, "conversation_nif"),
    erlang:load_nif(NifPath, 0).

parse_conv(_Source) ->
    erlang:nif_error(nif_not_loaded).

compile_grammar(_Source) ->
    erlang:nif_error(nif_not_loaded).

compile_grammar_traced(_Source) ->
    erlang:nif_error(nif_not_loaded).

check_property(_Source, _Property) ->
    erlang:nif_error(nif_not_loaded).

check_shannon_equivalence(_Source) ->
    erlang:nif_error(nif_not_loaded).

check_connected(_Source) ->
    erlang:nif_error(nif_not_loaded).

check_bipartite(_Source) ->
    erlang:nif_error(nif_not_loaded).

check_exhaustive(_Source) ->
    erlang:nif_error(nif_not_loaded).
