import conversation/compiler
import conversation/runtime
import conversation/protocol.{Arm, Case, Cmp, DesiredState, Gt, Pass, Wildcard}

pub type CompiledDomain =
  compiler.CompiledDomain

pub type CompilerMessage =
  compiler.Message

pub fn start_compiler() {
  compiler.start()
}

pub fn main() {
  // Example: case dispatch with wildcard fallback
  let spec =
    Case("error.rate", [
      Arm(Cmp(Gt, "0.1"), DesiredState("health_monitor", "critical")),
      Arm(Wildcard, Pass),
    ])

  let deltas = runtime.converge(spec)
  deltas
}
