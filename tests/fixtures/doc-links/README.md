# Fixture entry page

<a id="local-anchor"></a>

Valid local links: [guide](docs/guide.md), [section](docs/guide.md#second-section),
[repeated heading](docs/guide.md#repeat-1), [code heading](docs/guide.md#molten-cli-quick-start),
[explicit id](docs/guide.md#pinned), [line](docs/data.txt#L2), [range](docs/data.txt#L1-L3),
[self](#local-anchor), [heading](#fixture-entry-page), [root-relative](/docs/guide.md),
[directory](docs/) and [angle](<docs/guide.md>).

[reference]: docs/guide.md#first-section

External targets are skipped: [web](https://example.invalid/missing.md) and [mail](mailto:nobody@example.invalid).

Code is not a link: `[inline](missing-inline.md)`.

```md
[fenced](missing-fenced.md)
```
