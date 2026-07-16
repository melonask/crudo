---
layout: home
hero:
  name: crudo
  text: Your SQL, a production-minded JSON API
  tagline: Declare routes, authentication, protections, schema setup, and response shapes in TOML—without application-specific Rust.
  image: { src: '/logo.svg', alt: 'crudo logo' }
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: View repository
      link: https://github.com/melonask/crudo
features:
  - title: Configuration, not boilerplate
    details: Bind named request values to SQL and choose precise JSON result modes.
  - title: Defenses included
    details: Per-IP limits, body and concurrency ceilings, timeouts, CORS, ALTCHA, and Argon2.
  - title: Wallet-aware
    details: Atomically derive and persist EVM, Solana, and native SegWit public addresses.
---

## Quick start

Clone the repository, install `crudo`, and run the zero-environment starter from the repository root. The CLI resolves its default as `config/minimal.toml` from the current directory:

```sh
crudo
curl http://127.0.0.1:3000/v1/health
```

Read the [getting-started guide](/guide/getting-started), the complete [configuration reference](/reference/configuration), or browse the [repository](https://github.com/melonask/crudo).
