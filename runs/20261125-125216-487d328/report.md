# Mac verify 20261125-125216

- Commit: `487d328` on `main` — Fix final-review findings: Mac agent docs, verify.sh hardening, MSRV 1.95, probe zoom at creation
- macOS: 26.7.3 (25G342), x86_64
- Xcode CLT: /Applications/Xcode.app/Contents/Developer
- Rust: rustc 1.99.1 (58cf8f9c3 2026-10-13)
- Cargo: cargo 1.99.1 (5f94df478 2026-08-27)
- Targets: aarch64-apple-darwin x86_64-apple-darwin 

## Steps

| Step | Result | Time |
|---|---|---|
| Tools | ok | 1s |
| Tests (cargo test --workspace) | ok | 23s |
| Build app (universal) | ok | 247s |

## Tests

- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
- test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

## Warnings (0)

None.

## Failures

None.

## App

- lipo: x86_64 arm64 
- entitlements: com.apple.security.app-sandbox 
- signature: ok
- size:  24M

## Key probe (hands-on)

_Not yet recorded. Add it as described in MAC_AGENT.md, then run `scripts/mac/verify.sh --publish-run out/mac/20261125-125216-487d328`._
