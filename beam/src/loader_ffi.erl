-module(loader_ffi).
-export([load_etf_module/1, is_loaded/1, purge_module/1, get_lenses/1, get_extends/1,
         get_requires/1, get_invariants/1, get_ensures/1]).

%% Decode ETF → EAF, compile to BEAM, load module.
%% Returns {ok, ModuleName} or {error, Reason}.
load_etf_module(EtfBinary) ->
    try
        Forms = binary_to_term(EtfBinary),
        case compile:forms(Forms) of
            {ok, Module, Binary} ->
                case code:load_binary(Module, "conversation", Binary) of
                    {module, Module} -> {ok, atom_to_binary(Module, utf8)};
                    Err -> {error, iolist_to_binary(io_lib:format("~p", [Err]))}
                end;
            Err -> {error, iolist_to_binary(io_lib:format("~p", [Err]))}
        end
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.

%% Check if a module is loaded on the BEAM.
is_loaded(ModuleBinary) ->
    Module = binary_to_atom(ModuleBinary, utf8),
    case code:is_loaded(Module) of
        {file, _} -> true;
        false -> false
    end.

%% Purge and delete a loaded module from the BEAM.
%% Called on enforcement failure to clean up after a rejected compile.
%% code:purge/1 terminates processes running old code; code:delete/1 removes it.
%% Returns ok regardless of whether the module was loaded (idempotent).
purge_module(ModuleBinary) ->
    Module = binary_to_atom(ModuleBinary, utf8),
    code:purge(Module),
    code:delete(Module),
    ok.

%% Call Module:lenses() → List(String).
get_lenses(ModuleBinary) ->
    Module = binary_to_atom(ModuleBinary, utf8),
    try
        Lenses = Module:lenses(),
        {ok, Lenses}
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.

%% Call Module:extends() → List(String).
get_extends(ModuleBinary) ->
    Module = binary_to_atom(ModuleBinary, utf8),
    try
        Extends = Module:extends(),
        {ok, Extends}
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.

%% Call Module:requires() → List(String).
get_requires(ModuleBinary) ->
    Module = binary_to_atom(ModuleBinary, utf8),
    try
        Requires = Module:requires(),
        {ok, Requires}
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.

%% Call Module:invariants() → List(String).
get_invariants(ModuleBinary) ->
    Module = binary_to_atom(ModuleBinary, utf8),
    try
        Invariants = Module:invariants(),
        {ok, Invariants}
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.

%% Call Module:ensures() → List(String).
get_ensures(ModuleBinary) ->
    Module = binary_to_atom(ModuleBinary, utf8),
    try
        Ensures = Module:ensures(),
        {ok, Ensures}
    catch
        _:Reason -> {error, iolist_to_binary(io_lib:format("~p", [Reason]))}
    end.
