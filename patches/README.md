# LiteLLM compatibility patch

The portable build installs LiteLLM directly from the production commit in
[LiteLLM PR #32995](https://github.com/BerriAI/litellm/pull/32995), pinned to
upstream commit `dfe91303a72792bce0c790ab8615b779c1c4730a`.

The local regression test is `tests/test_tool_output_adjacency.py`. Installing
the pinned commit avoids applying a context-sensitive patch to a PyPI package
whose code may no longer match the pull request's original base.

Upstream commit patch SHA-256 at adoption time:
`729935A03BC50877BA96877945A3881C133F49D5117116AF8D28DFC33D5D3F63`.
