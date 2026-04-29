# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- Simplified the public release docs and CI guidance
- Removed tracked internal task-pass history from the release surface
- Switched CI to a single workspace verification flow

### Fixed

- Replaced owner-specific and local-path examples in docs and test fixtures
- Pointed doctor remediation output at the repository docs instead of a hardcoded external URL

## [0.1.0] - 2026-05-11

### Added

- Initial implementation of agent-adapter universal adapter
- Core types: ChatRequest, AgentEvent, ApprovalPolicy, AdapterError
- AgentAdapter trait and AgentRouter for provider routing
- ConfigLoader with built-in defaults for 8 providers
- FakeProvider test harness with EventBuilder and EventStreamAssertions
- Provider contract tests for all built-in providers
- RealProviderGates for integration test control via env vars
- Docs linting module for documentation consistency
- JSONL, one-shot, ACP stdio, and PTY fallback transports
- CLI with run, healthcheck, doctor, and providers commands

### Providers

- Codex: JSONL command transport
- Claude: JSONL command transport
- Gemini: JSONL command transport
- Kimi: ACP and JSONL transports
- OpenCode: custom command support
- Hermes: ACP and one-shot transports
- Forge: custom command support
- Claw: custom command support

### Documentation

- docs/design.md - Architecture and design overview
- docs/policy.md - Approval policy documentation
- docs/config.md - Configuration schema
- docs/ci.md - CI pipeline documentation
- docs/build.md - Build instructions
- docs/contributing.md - Contribution guidelines
- docs/testing.md - Testing documentation
- Provider docs in docs/providers/
