-record(grammar, {
    domain :: binary(),
    types :: gleam@dict:dict(binary(), gleam@set:set(binary()))
}).
