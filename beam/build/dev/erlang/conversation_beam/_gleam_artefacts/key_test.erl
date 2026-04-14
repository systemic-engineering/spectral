-module(key_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/key_test.gleam").
-export([generate_produces_keypair_test/0, sign_verify_roundtrip_test/0, verify_wrong_message_fails_test/0, verify_wrong_key_fails_test/0, key_oid_deterministic_test/0, different_keys_different_oids_test/0, hierarchical_derivation_deterministic_test/0, hierarchical_differs_from_flat_test/0, different_names_different_derived_keys_test/0, derive_sign_verify_roundtrip_test/0]).

-file("test/key_test.gleam", 6).
-spec generate_produces_keypair_test() -> nil.
generate_produces_keypair_test() ->
    Kp = conversation@key:generate(),
    Pub_key = conversation@key:public_key(Kp),
    case Pub_key of
        {ed25519, _} ->
            gleeunit@should:be_true(true)
    end.

-file("test/key_test.gleam", 16).
-spec sign_verify_roundtrip_test() -> nil.
sign_verify_roundtrip_test() ->
    Kp = conversation@key:generate(),
    Pub_key = conversation@key:public_key(Kp),
    Message = <<"hello world"/utf8>>,
    Signature = conversation@key:sign(Kp, Message),
    _pipe = conversation@key:verify(Pub_key, Message, Signature),
    gleeunit@should:be_true(_pipe).

-file("test/key_test.gleam", 24).
-spec verify_wrong_message_fails_test() -> nil.
verify_wrong_message_fails_test() ->
    Kp = conversation@key:generate(),
    Pub_key = conversation@key:public_key(Kp),
    Signature = conversation@key:sign(Kp, <<"correct"/utf8>>),
    _pipe = conversation@key:verify(Pub_key, <<"wrong"/utf8>>, Signature),
    gleeunit@should:be_false(_pipe).

-file("test/key_test.gleam", 31).
-spec verify_wrong_key_fails_test() -> nil.
verify_wrong_key_fails_test() ->
    Kp1 = conversation@key:generate(),
    Kp2 = conversation@key:generate(),
    Pub_key2 = conversation@key:public_key(Kp2),
    Signature = conversation@key:sign(Kp1, <<"message"/utf8>>),
    _pipe = conversation@key:verify(Pub_key2, <<"message"/utf8>>, Signature),
    gleeunit@should:be_false(_pipe).

-file("test/key_test.gleam", 39).
-spec key_oid_deterministic_test() -> nil.
key_oid_deterministic_test() ->
    Kp = conversation@key:generate(),
    Pub_key = conversation@key:public_key(Kp),
    Oid1 = conversation@key:oid(Pub_key),
    Oid2 = conversation@key:oid(Pub_key),
    _pipe = conversation@oid:equals(
        conversation@ref:oid(Oid1),
        conversation@ref:oid(Oid2)
    ),
    gleeunit@should:be_true(_pipe).

-file("test/key_test.gleam", 48).
-spec different_keys_different_oids_test() -> nil.
different_keys_different_oids_test() ->
    Kp1 = conversation@key:generate(),
    Kp2 = conversation@key:generate(),
    Oid1 = conversation@key:oid(conversation@key:public_key(Kp1)),
    Oid2 = conversation@key:oid(conversation@key:public_key(Kp2)),
    _pipe = conversation@oid:equals(
        conversation@ref:oid(Oid1),
        conversation@ref:oid(Oid2)
    ),
    gleeunit@should:be_false(_pipe).

-file("test/key_test.gleam", 56).
-spec hierarchical_derivation_deterministic_test() -> nil.
hierarchical_derivation_deterministic_test() ->
    Root = conversation@key:generate(),
    Root_pub = conversation@key:public_key(Root),
    Kp1 = conversation@key:derive_child(Root_pub, <<"compiler"/utf8>>),
    Kp2 = conversation@key:derive_child(Root_pub, <<"compiler"/utf8>>),
    Oid1 = conversation@key:oid(conversation@key:public_key(Kp1)),
    Oid2 = conversation@key:oid(conversation@key:public_key(Kp2)),
    _pipe = conversation@oid:equals(
        conversation@ref:oid(Oid1),
        conversation@ref:oid(Oid2)
    ),
    gleeunit@should:be_true(_pipe).

-file("test/key_test.gleam", 67).
-spec hierarchical_differs_from_flat_test() -> nil.
hierarchical_differs_from_flat_test() ->
    Root = conversation@key:generate(),
    Root_pub = conversation@key:public_key(Root),
    Derived = conversation@key:derive_child(Root_pub, <<"compiler"/utf8>>),
    Flat = conversation@key:from_seed(<<0:256>>),
    Oid_derived = conversation@key:oid(conversation@key:public_key(Derived)),
    Oid_flat = conversation@key:oid(conversation@key:public_key(Flat)),
    _pipe = conversation@oid:equals(
        conversation@ref:oid(Oid_derived),
        conversation@ref:oid(Oid_flat)
    ),
    gleeunit@should:be_false(_pipe).

-file("test/key_test.gleam", 78).
-spec different_names_different_derived_keys_test() -> nil.
different_names_different_derived_keys_test() ->
    Root = conversation@key:generate(),
    Root_pub = conversation@key:public_key(Root),
    Compiler_kp = conversation@key:derive_child(Root_pub, <<"compiler"/utf8>>),
    Garden_kp = conversation@key:derive_child(Root_pub, <<"garden"/utf8>>),
    Oid1 = conversation@key:oid(conversation@key:public_key(Compiler_kp)),
    Oid2 = conversation@key:oid(conversation@key:public_key(Garden_kp)),
    _pipe = conversation@oid:equals(
        conversation@ref:oid(Oid1),
        conversation@ref:oid(Oid2)
    ),
    gleeunit@should:be_false(_pipe).

-file("test/key_test.gleam", 89).
-spec derive_sign_verify_roundtrip_test() -> nil.
derive_sign_verify_roundtrip_test() ->
    Root = conversation@key:generate(),
    Root_pub = conversation@key:public_key(Root),
    Derived = conversation@key:derive_child(Root_pub, <<"actor"/utf8>>),
    Pub_key = conversation@key:public_key(Derived),
    Message = <<"signed by derived key"/utf8>>,
    Signature = conversation@key:sign(Derived, Message),
    _pipe = conversation@key:verify(Pub_key, Message, Signature),
    gleeunit@should:be_true(_pipe).
