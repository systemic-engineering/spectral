import conversation/domain
import conversation/garden
import gleam/erlang/process
import gleeunit/should

/// Garden factory supervisor starts a domain server.
pub fn garden_starts_domain_test() {
  let name = process.new_name("garden_test_start")
  let assert Ok(_) = garden.start(name)

  let assert Ok(_) = garden.start_domain(name, "garden_alpha")
  should.be_true(garden.is_running("garden_alpha"))

  let _ = garden.stop_domain("garden_alpha")
}

/// Garden starts and stops multiple domains.
pub fn garden_multiple_domains_test() {
  let name = process.new_name("garden_test_multi")
  let assert Ok(_) = garden.start(name)

  let assert Ok(_) = garden.start_domain(name, "garden_one")
  let assert Ok(_) = garden.start_domain(name, "garden_two")

  should.be_true(garden.is_running("garden_one"))
  should.be_true(garden.is_running("garden_two"))

  // Stop one
  let assert Ok(_) = garden.stop_domain("garden_one")
  should.be_false(garden.is_running("garden_one"))
  should.be_true(garden.is_running("garden_two"))

  let _ = garden.stop_domain("garden_two")
}

/// Garden factory supervisor restarts crashed domain servers.
pub fn garden_restarts_on_kill_test() {
  let name = process.new_name("garden_test_restart")
  let assert Ok(_) = garden.start(name)

  let assert Ok(_) = garden.start_domain(name, "garden_phoenix")
  should.be_true(garden.is_running("garden_phoenix"))

  // Kill it — factory supervisor should restart
  domain.kill("garden_phoenix")
  process.sleep(100)
  should.be_true(garden.is_running("garden_phoenix"))

  let _ = garden.stop_domain("garden_phoenix")
}
