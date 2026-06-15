# Security Policy

## Supported versions

LimitBook is pre-1.0 and under active development. Security fixes are applied to the `main`
branch. There is no long-term-support branch yet.

| Version | Supported |
|---------|-----------|
| `main`  | ✅        |
| tagged pre-releases | ⚠️ best-effort |

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, use GitHub's private vulnerability reporting:

1. Go to the **Security** tab of this repository.
2. Click **Report a vulnerability**.
3. Provide a clear description, reproduction steps, and impact assessment.

You can expect an acknowledgement within **72 hours** and a status update within **7 days**.

## Scope

This project has a deliberately narrow trust boundary. When assessing security, note:

- **The engine-core crate trusts only validated input.** All validation happens at the gateway
  boundary (`POST /orders`, etc.). Reports about the engine panicking or misbehaving on
  *unvalidated* input passed directly to the library API are valid and welcome.
- **The gateway binds to localhost by default** and restricts CORS to localhost origins. It is
  **not hardened for direct public internet exposure** — there is no authentication, rate
  limiting beyond backpressure, or TLS termination built in. Deploying it publicly without a
  reverse proxy and auth layer is out of scope and not recommended.
- The synthetic-flow **simulator endpoint** generates load on demand; treat it as a development
  tool, not a production-safe feature.

## What we consider a vulnerability

- Memory-safety issues in any `unsafe` block.
- Panics / crashes reachable from validated gateway input.
- Integer overflow producing incorrect trades or quantities.
- Denial of service via a single well-formed request (unbounded allocation, unbounded loop).
- Secret/credential leakage (there should be none — none are stored).

## What we do not consider a vulnerability

- Resource exhaustion from the simulator or from deliberately abusive request volumes against an
  unprotected localhost binding.
- Issues requiring the operator to expose the gateway publicly without the recommended proxy/auth.
