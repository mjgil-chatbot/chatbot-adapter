# Testkit

The `agent-adapter-testkit` crate provides testing utilities for the adapter system.

## Components

### Fake Adapters

`FakeAdapter` simulates provider behavior for testing:

```rust
use agent_adapter_testkit::FakeAdapter;

let adapter = FakeAdapter::new(ProviderId::Claude)
    .simulate_text_response("Hello world");
```

### Real Provider Gates

`RealProviderGates` controls whether real provider integration tests run:

```rust
use agent_adapter_testkit::RealProviderGates;

let gates = RealProviderGates::from_env();
if gates.is_enabled("claude") {
    // Run real provider tests
}
```

Real provider tests are **disabled by default**. See
[testing-real-providers.md](testing-real-providers.md) for the current
provider gate list and example commands.

### Contract Tests

Contract tests verify provider adapter behavior against a specification.

### Fixtures

Golden fixtures for deterministic test comparisons.

### Docs Lint

Documentation linting for provider documentation.

### Temp Workspaces

`TempWorkspace` and `WorktreeWorkspace` for isolated testing.

## Usage

```rust
#[cfg(test)]
mod tests {
    use agent_adapter_testkit::{FakeAdapter, TempWorkspace};

    #[test]
    fn test_with_fake_adapter() {
        let adapter = FakeAdapter::new(ProviderId::Claude)
            .simulate_text_response("test");
        // test adapter behavior
    }

    #[test]
    fn test_with_temp_workspace() {
        let ws = TempWorkspace::new().unwrap();
        // test in isolated workspace
    }
}
```
