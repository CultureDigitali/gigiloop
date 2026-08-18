# GigiLoop branding

## Canonical approved assets

Use these files for repository presentation and public launch material:

- [`gigiloop-logo.jpg`](gigiloop-logo.jpg) — primary rendered logo and wordmark.
- [`gigiloop-superbanner.jpg`](gigiloop-superbanner.jpg) — approved repository hero with headline, workflow, compatibility, and install hook.
- [`gigiloop-compatibility.jpg`](gigiloop-compatibility.jpg) — approved multi-host compatibility artwork.
- [`visual-manifest.json`](visual-manifest.json) — locked hashes, dimensions, provenance, and intended roles.
- [`loop.svg`](loop.svg) — protocol diagram for technical documentation.

The canonical identity is the neon infinity / iteration mark with the lightning + verification motif. The rendered JPEGs are the visuals approved by the repository owner; CI verifies their exact hashes and dimensions.

## Legacy assets

`logo-v2.svg` and `banner-v2.svg` were simplified vector interpretations created during an earlier iteration. They are not the canonical approved visuals and should not be used for the README hero, social preview, launch imagery, or product identity.

They may remain temporarily for historical compatibility, but new references must use the canonical rendered assets above.

## Third-party compatibility marks

The compatibility artwork identifies products that can consume GigiLoop. It does not imply endorsement, sponsorship, partnership, or certification.

For standalone third-party marks, use vendor-controlled downloads and current brand guidance without modifying the logo:

- **OpenCode:** https://opencode.ai/brand
- **Claude / Anthropic:** https://www.anthropic.com/press-kit
- **OpenAI / Codex:** https://openai.com/brand/
- **Cursor:** https://cursor.com/brand
- **Gemini:** https://about.google/brand-resource-center/logos-list/

OpenCode, Claude, Anthropic, OpenAI, Codex, Cursor, Gemini, GitHub Copilot, Cline, OpenHands, Amp, and other third-party names and marks belong to their respective owners.

Use a third-party mark only to describe genuine compatibility. Do not merge it into the GigiLoop logo, imply endorsement, or make it more prominent than GigiLoop.

## Visual change control

Do not replace a canonical rendered asset because another file has the same filename. A visual update requires:

1. explicit approval of the rendered result;
2. updated SHA-256 and dimensions in `visual-manifest.json`;
3. passing asset-integrity CI;
4. README and skill metadata review;
5. a changelog entry.
