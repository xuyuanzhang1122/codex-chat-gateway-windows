# Third-party notices

## LiteLLM

- Project: https://github.com/BerriAI/litellm
- Version used: upstream commit `dfe91303a72792bce0c790ab8615b779c1c4730a`
- Installation source: GitHub source archive for LiteLLM PR #32995
- License: repository content outside its separately licensed enterprise directory is MIT licensed; see the upstream `LICENSE` file for the complete and current terms.

The Windows distribution installs the pending upstream compatibility change directly from
LiteLLM PR #32995's pinned production commit. Provenance is documented in `patches/README.md`.

## Python embedded distribution

- Project: https://www.python.org/
- Version used by the Windows portable package: `3.11.9` x64 embedded distribution
- Source archive: `python-3.11.9-embed-amd64.zip`
- Archive SHA-256: `009d6bf7e3b2ddca3d784fa09f90fe54336d5b60f0e0f305c37f400bf83cfd3b`
- License: Python Software Foundation License; the complete `LICENSE.txt` is included in `runtime/`.

The portable package also contains LiteLLM's transitive Python dependencies and their installed package metadata.
