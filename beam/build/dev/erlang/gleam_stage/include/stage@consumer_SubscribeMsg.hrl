-record(subscribe_msg, {
    producer_subject :: gleam@erlang@process:subject(stage@internal@message:producer_message(any())),
    opts :: stage@subscription:subscribe_opts(),
    reply_to :: gleam@erlang@process:subject({ok, nil} |
        {error, stage@error:stage_error()})
}).
