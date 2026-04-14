-record(subscribe_opts, {
    demand :: stage@subscription:demand_mode(),
    cancel :: stage@subscription:cancel(),
    partition :: gleam@option:option(integer())
}).
