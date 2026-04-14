-module(crypto_ffi).
-export([sha256/1, sha512/1, term_to_binary/1,
         generate_ed25519/0, generate_ed25519_from_seed/1,
         sign_ed25519/2, verify_ed25519/3,
         system_time_ms/0]).

sha256(Data) ->
    crypto:hash(sha256, Data).

sha512(Data) ->
    crypto:hash(sha512, Data).

term_to_binary(Term) ->
    erlang:term_to_binary(Term).

generate_ed25519() ->
    {Public, Private} = crypto:generate_key(eddsa, ed25519),
    {Public, Private}.

generate_ed25519_from_seed(Seed) ->
    {Public, Private} = crypto:generate_key(eddsa, ed25519, Seed),
    {Public, Private}.

sign_ed25519(Private, Message) ->
    crypto:sign(eddsa, sha512, Message, [Private, ed25519]).

verify_ed25519(Public, Message, Signature) ->
    crypto:verify(eddsa, sha512, Message, Signature, [Public, ed25519]).

system_time_ms() ->
    erlang:system_time(millisecond).
