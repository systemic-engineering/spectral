-record(consumer_config, {
    init_state :: any(),
    on_events :: fun((list(any()), any()) -> any())
}).
