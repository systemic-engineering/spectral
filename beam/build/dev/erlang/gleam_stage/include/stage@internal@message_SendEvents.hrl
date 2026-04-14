-record(send_events, {
    demand_subject :: gleam@erlang@process:subject(stage@internal@message:producer_message(any())),
    events :: list(any())
}).
