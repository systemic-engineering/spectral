-record(consumer_registry, {
    subscriptions :: gleam@dict:dict(gleam@erlang@process:subject(stage@internal@message:producer_message(any())), stage@subscription:consumer_subscription(any()))
}).
