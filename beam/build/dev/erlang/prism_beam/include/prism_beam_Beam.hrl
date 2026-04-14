-record(beam, {
    result :: any(),
    path :: list(prism_beam:oid()),
    loss :: prism_beam:shannon_loss(),
    precision :: prism_beam:precision(),
    recovered :: gleam@option:option(prism_beam:recovery())
}).
