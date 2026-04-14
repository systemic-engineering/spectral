-module(conversation@oid).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/oid.gleam").
-export([from_string/1, to_string/1, equals/2, from_bytes/1]).
-export_type([oid/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Oid — content-addressed identity.\n"
    "\n"
    " Same content = same Oid. Value type wrapping a SHA-512 hex string.\n"
).

-opaque oid() :: {oid, binary()}.

-file("src/conversation/oid.gleam", 18).
?DOC(" Construct an Oid from its hex string representation.\n").
-spec from_string(binary()) -> oid().
from_string(Hex) ->
    {oid, Hex}.

-file("src/conversation/oid.gleam", 23).
?DOC(" The hex string representation of the Oid.\n").
-spec to_string(oid()) -> binary().
to_string(Oid) ->
    erlang:element(2, Oid).

-file("src/conversation/oid.gleam", 28).
?DOC(" Are two Oids equal?\n").
-spec equals(oid(), oid()) -> boolean().
equals(A, B) ->
    erlang:element(2, A) =:= erlang:element(2, B).

-file("src/conversation/oid.gleam", 50).
-spec hex_char(integer()) -> binary().
hex_char(Nibble) ->
    case Nibble of
        0 ->
            <<"0"/utf8>>;

        1 ->
            <<"1"/utf8>>;

        2 ->
            <<"2"/utf8>>;

        3 ->
            <<"3"/utf8>>;

        4 ->
            <<"4"/utf8>>;

        5 ->
            <<"5"/utf8>>;

        6 ->
            <<"6"/utf8>>;

        7 ->
            <<"7"/utf8>>;

        8 ->
            <<"8"/utf8>>;

        9 ->
            <<"9"/utf8>>;

        10 ->
            <<"a"/utf8>>;

        11 ->
            <<"b"/utf8>>;

        12 ->
            <<"c"/utf8>>;

        13 ->
            <<"d"/utf8>>;

        14 ->
            <<"e"/utf8>>;

        _ ->
            <<"f"/utf8>>
    end.

-file("src/conversation/oid.gleam", 39).
-spec do_hex_encode(bitstring(), binary()) -> binary().
do_hex_encode(Data, Acc) ->
    case Data of
        <<Byte/integer, Rest/bitstring>> ->
            Hi = hex_char(Byte div 16),
            Lo = hex_char(Byte rem 16),
            do_hex_encode(
                Rest,
                gleam@string:append(Acc, gleam@string:append(Hi, Lo))
            );

        _ ->
            Acc
    end.

-file("src/conversation/oid.gleam", 35).
-spec hex_encode(bitstring()) -> binary().
hex_encode(Data) ->
    do_hex_encode(Data, <<""/utf8>>).

-file("src/conversation/oid.gleam", 13).
?DOC(" Compute an Oid from raw bytes (SHA-512, hex-encoded).\n").
-spec from_bytes(bitstring()) -> oid().
from_bytes(Data) ->
    {oid, hex_encode(crypto_ffi:sha512(Data))}.
