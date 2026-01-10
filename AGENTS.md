# Agent Instructions

Before submitting a Pull Request, you MUST perform the following validation steps. 
Do not create the PR if any of these fail. Fix the issues and retry.

1. **Formatting**: Run `cargo fmt --check`. If this fails, run `cargo fmt` to fix it.
2. **Testing**: Run `cargo test`. All tests must pass.
3. **Documentation**: Run `cargo doc`. This must run successfully with no warnings.
4. **Linting**: Run `cargo clippy -- -D warnings`. There must not be
   any warnings.
