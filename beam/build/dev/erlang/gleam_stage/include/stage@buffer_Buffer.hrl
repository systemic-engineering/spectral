-record(buffer, {
    events :: list(any()),
    size :: integer(),
    max_size :: integer(),
    overflow :: stage@buffer:overflow_strategy()
}).
