Graph breaker
====

This service would create pull requests which disable or re-enable blocked paths in Cincinnati graphs 
on external request.

# Howto

* Create a target repo - a copy of https://github.com/openshift/cincinnati-graph-data repo. Avoid using 
the openshift repo, as this service would create pull requests to it. Optionally fork this repo to emulate 
fork process.

* Prepare a Github token. It needs to have write/commit/pull-request permissions.

* Create a copy of `./config/example.toml` and fill in the token and target/fork repo params.

* Run the service via `cargo build --release && ./target/release/graph-breaker -c path/to/your/config.toml -vv`

* `curl --header 'Authorization: Bearer foo' -kLvs http://localhost:8080/action\?type\=enable\&version\=4.3.12` 
  This will make the service create a new pull request to target repo, which removes 4.3.12 block.

* `curl --header 'Authorization: Bearer foo' -kLvs http://localhost:8080/action\?type\=disable\&version\=4.3.13`
  This will make the service create a pull request which blocks upgrades to 4.3.13 version

Branch name, pull request title and body are so far hardcoded
