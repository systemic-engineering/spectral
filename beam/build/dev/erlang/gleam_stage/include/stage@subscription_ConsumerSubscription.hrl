-record(consumer_subscription, {
    demand_subject :: gleam@erlang@process:subject(stage@internal@message:producer_message(any())),
    events_subject :: gleam@erlang@process:subject(stage@internal@message:consumer_message(any())),
    opts :: stage@subscription:subscribe_opts(),
    pending_demand :: integer()
}).
