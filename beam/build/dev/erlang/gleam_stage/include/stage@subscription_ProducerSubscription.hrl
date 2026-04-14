-record(producer_subscription, {
    consumer_subject :: gleam@erlang@process:subject(stage@internal@message:consumer_message(any())),
    demand_subject :: gleam@erlang@process:subject(stage@internal@message:producer_message(any()))
}).
