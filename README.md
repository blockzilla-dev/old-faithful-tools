# Old Faithful tools

CAR readers, lossless source metadata, import tools and fixed-block benchmarks
extracted from Blockzilla. Shared Solana wire codecs and Compact V2 formats are
provided by the sibling Blockzilla core.

- `crates/old-faithful/of-car-reader`: CAR reader and its two fixed benchmark fixtures.
- `crates/old-faithful/of-slot-ranges`: slot-range support.
- `tools/legacy-cli`: existing import/archive commands, with V3 commands removed.
- `tools/sample-block-importer`: reproducible CAR-to-Compact benchmark fixture preparation.
- `examples/read-car`: retained CAR reader examples.

Run `cargo bench -p of-car-reader --bench big_block` for the CAR sample. The
Compact V2 sample benchmark lives in Blockzilla. Full timing histories and
machine records live in the private records repository. Source dependencies use the sibling checkout layout below. No packages have been
published to a package registry.

## Checkout

Keep this repository beside the Blockzilla core and indexers checkouts. The
existing CAR examples use the shared application benchmark workloads in
`blockzilla-indexers/examples/workloads`. Until the cleanup is merged, select
the core candidate branch explicitly:

```sh
git clone --branch codex/1.0-core git@github.com:blockzilla-dev/blockzilla.git blockzilla
git clone git@github.com:blockzilla-dev/old-faithful-tools.git
git clone git@github.com:blockzilla-dev/blockzilla-indexers.git
```

The repository is private. Access to `blockzilla-dev` is required.
