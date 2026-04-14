-record(subscribe, {
    consumer_subject :: gleam@erlang@process:subject(stage@internal@message:consumer_message(any())),
    reply_to :: gleam@erlang@process:subject({ok,
            stage@internal@message:subscription_ref(any())} |
        {error, binary()}),
    partition :: gleam@option:option(integer())
}).
