# spectral

jq for reality. One binary. Five operations. Everything settles.

## Install

```
cargo install spectral
```

## Use

```
spectral fold .              observe any structure
spectral prism .             filter by what matters
spectral traversal .         explore what's connected
spectral lens .              transform one thing
spectral iso .               settle. done. crystal.
```

## Agent Memory

```
spectral memory store fact "the tests pass on the refactor branch"
spectral memory recall <oid>
spectral memory crystallize <oid>
spectral memory status
```

## Claude Code

Add to `.claude/settings.json`:

```json
{
  "mcpServers": {
    "spectral": {
      "command": "spectral",
      "args": ["serve", "--project", "."]
    }
  }
}
```

## Tools

```
spectral mirror <cmd>        compiler
spectral conversation <cmd>  runtime
spectral db <cmd>            graph database
spectral memory <cmd>        agent memory
spectral serve               MCP server
```
