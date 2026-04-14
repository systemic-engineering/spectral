-record(subscription_ref, {
    events_subject :: gleam@erlang@process:subject(stage@internal@message:consumer_message(any())),
    demand_subject :: gleam@erlang@process:subject(stage@internal@message:producer_message(any())),
    producer_pid :: gleam@erlang@process:pid_()
}).
