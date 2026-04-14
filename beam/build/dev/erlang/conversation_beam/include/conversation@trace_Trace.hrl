-record(trace, {
    oid :: conversation@oid:oid(),
    actor :: conversation@ref:scoped_oid(conversation@key:key()),
    parent :: gleam@option:option(conversation@oid:oid()),
    value :: any(),
    signature :: bitstring(),
    timestamp :: integer()
}).
