-module(conversation@key).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/key.gleam").
-export([public_key/1, oid/1, generate/0, from_seed/1, sign/2, verify/3, derive_child/2]).
-export_type([key_pair/0, key/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Key — actor cryptographic identity.\n"
    "\n"
    " Ed25519 keypairs for signing and verifying actor messages.\n"
).

-opaque key_pair() :: {key_pair, bitstring(), bitstring()}.

-type key() :: {ed25519, bitstring()}.

-file("src/conversation/key.gleam", 32).
?DOC(" Extract the public key from a keypair.\n").
-spec public_key(key_pair()) -> key().
public_key(Kp) ->
    {ed25519, erlang:element(2, Kp)}.

-file("src/conversation/key.gleam", 58).
?DOC(" Content address of a public key.\n").
-spec oid(key()) -> conversation@ref:scoped_oid(key()).
oid(Key) ->
    {ed25519, Public} = Key,
    conversation@ref:scope(conversation@oid:from_bytes(Public)).

-file("src/conversation/key.gleam", 19).
?DOC(" Generate a new Ed25519 keypair.\n").
-spec generate() -> key_pair().
generate() ->
    {Public, Private} = crypto_ffi:generate_ed25519(),
    {key_pair, Public, Private}.

-file("src/conversation/key.gleam", 26).
?DOC(
    " Generate a deterministic Ed25519 keypair from a 32-byte seed.\n"
    " Same seed = same keypair = same identity. This is the cairn pattern.\n"
).
-spec from_seed(bitstring()) -> key_pair().
from_seed(Seed) ->
    {Public, Private} = crypto_ffi:generate_ed25519_from_seed(Seed),
    {key_pair, Public, Private}.

-file("src/conversation/key.gleam", 37).
?DOC(" Sign data with a keypair.\n").
-spec sign(key_pair(), bitstring()) -> bitstring().
sign(Kp, Data) ->
    crypto_ffi:sign_ed25519(erlang:element(3, Kp), Data).

-file("src/conversation/key.gleam", 42).
?DOC(" Verify a signature against a public key and data.\n").
-spec verify(key(), bitstring(), bitstring()) -> boolean().
verify(Key, Data, Signature) ->
    {ed25519, Public} = Key,
    crypto_ffi:verify_ed25519(Public, Data, Signature).

-file("src/conversation/key.gleam", 50).
?DOC(
    " Derive a child keypair from a root public key and domain name.\n"
    " sha512(root_pub || name) → first 32 bytes → Ed25519 seed.\n"
    " Anyone with the root public key can derive any actor's public key.\n"
).
-spec derive_child(key(), binary()) -> key_pair().
derive_child(Root, Name) ->
    {ed25519, Public} = Root,
    Seed_input = <<Public/bitstring, Name/binary>>,
    Seed@1 = case crypto_ffi:sha512(Seed_input) of
        <<Seed:32/binary, _/binary>> -> Seed;
        _assert_fail ->
            erlang:error(#{gleam_error => let_assert,
                        message => <<"Pattern match failed, no pattern matched the value."/utf8>>,
                        file => <<?FILEPATH/utf8>>,
                        module => <<"conversation/key"/utf8>>,
                        function => <<"derive_child"/utf8>>,
                        line => 53,
                        value => _assert_fail,
                        start => 1609,
                        'end' => 1680,
                        pattern_start => 1620,
                        pattern_end => 1656})
    end,
    from_seed(Seed@1).
