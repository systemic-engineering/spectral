-record(producer_config, {
    init_state :: any(),
    on_demand :: fun((integer(), any()) -> {list(any()), any()}),
    dispatcher :: stage@dispatcher:dispatcher_type(any()),
    buffer_size :: integer(),
    buffer_overflow :: stage@buffer:overflow_strategy()
}).
