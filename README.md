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

## Updated 2026-04-07

### What spectral is

The system orchestrator — build, runtime, deployment, collaboration —
that wraps the `mirror` compiler. Where mirror compiles a single `.mirror`
file into a `.shatter` artifact, spectral coordinates the many compilations,
runtime backends, and execution targets that turn a tree of `.shatter`
files into a running system.

### Layering: mirror is to spectral what gcc is to make

```
mirror      compiles    .mirror source → .shatter artifact
            via         MirrorRuntime (single file in, single artifact out)

spectral    consumes    many .shatter artifacts
            orchestrates build / runtime / deployment / collaboration
            emits       executable form via one of N runtime backends
```

mirror is a single-file compiler. It does one job and exits. spectral is
the substrate that makes a *project* out of mirror compilations: it owns
the dependency graph between `.shatter` files, the runtime backends that
load and execute them, the collaboration surface, and the trust and care
boundaries.

### Runtime backends

A runtime backend is a thing that consumes a `.shatter` file and executes
it. Multiple backends are planned. The first one being designed is
`gen_prism`:

- `docs/gen_prism.md` — the BEAM runtime backend. Generates BEAM bytecode
  (gen_servers, function exports, dispatch tables) from the typed prism
  trajectory captured in a `.shatter`. Currently a design document, not
  yet built. The pattern was preserved from the deleted EAF emission code
  in `mirror` so spectral can reconstruct it against the `.shatter`
  substrate. *Status: planned, not yet built.*

Other runtime backends (native execution, replay-only loads,
visualization-only loads) live alongside `gen_prism` as siblings. None of
them are special; `gen_prism` is the BEAM-shaped one.

### Trust and care surfaces

- `docs/threat-model.md` — what spectral is defended against and what it
  is not.
- `docs/care-model.md` — what spectral is responsible for caring about,
  and what it explicitly refuses to.

These are not afterthoughts. The orchestrator's job is not just to build
and run; it is to hold the trust boundary between `.shatter` artifacts,
between collaborators, and between the runtime and the substrate.

