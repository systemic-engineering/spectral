-record(producer_consumer_config, {
    init_state :: any(),
    on_events :: fun((list(any()), any()) -> {list(any()), any()}),
    dispatcher :: stage@dispatcher:dispatcher_type(any()),
    buffer_size :: integer(),
    buffer_overflow :: stage@buffer:overflow_strategy()
}).
