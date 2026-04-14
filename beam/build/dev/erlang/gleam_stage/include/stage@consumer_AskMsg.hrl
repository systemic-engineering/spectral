-record(ask_msg, {
    producer_subject :: gleam@erlang@process:subject(stage@internal@message:producer_message(any())),
    count :: integer()
}).
