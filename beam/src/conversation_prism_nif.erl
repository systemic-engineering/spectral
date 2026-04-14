-module(conversation_prism_nif).
-export([prism_preview/3, prism_review/3, prism_modify/4, prism_compose/3]).
-on_load(init/0).

init() ->
    NifPath = filename:join(code:priv_dir(conversation_beam), "conversation_prism_nif"),
    erlang:load_nif(NifPath, 0).

prism_preview(_N, _Projection, _Source) -> erlang:nif_error(nif_not_loaded).
prism_review(_N, _Projection, _Focus) -> erlang:nif_error(nif_not_loaded).
prism_modify(_N, _Projection, _Source, _Transform) -> erlang:nif_error(nif_not_loaded).
prism_compose(_N, _P1, _P2) -> erlang:nif_error(nif_not_loaded).
