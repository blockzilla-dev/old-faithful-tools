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
machine records live in the private records repository. Source paths currently
use the local extraction layout; no packages or remote repository were published.
