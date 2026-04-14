-record(producer_registry, {
    subscriptions :: gleam@dict:dict(gleam@erlang@process:subject(stage@internal@message:producer_message(any())), stage@subscription:producer_subscription(any()))
}).
