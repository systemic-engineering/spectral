-record(partition_dispatcher, {
    partition_fn :: fun((any()) -> integer()),
    partitions :: integer()
}).
