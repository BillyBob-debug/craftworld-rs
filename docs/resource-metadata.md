# A2 metadata scanner and Toolkit oracle

Implemented and verified 2026-10-05. Original inputs remain immutable. A2 completes
the supported metadata scanner milestone; it is not a texture/mesh decoder or GUI.

## Scope and provenance

CONFIRMED_FROM_COMMUNITY_CODE: Toolkit MIT pin
d6a2ef80b5a09999f08f23a803b516096055e5c9, SerializedResource::process and
processDependencies; ResourceType, SerializationType, Branch and Revisions enums.
Rules are cited beside crates/lbp-resource/src/metadata.rs. Independent Rust layout
implementation; attribution retained in THIRD-PARTY-NOTICES. Full payload structures
and decompression remain deferred.

CONFIRMED_FROM_LOCAL_DATA: magic/header census of all 31,534 FARC entries. The old
sample maximum 940 did not cover this disc: additional clear heads 941–951 exist.
One representative of each additional head succeeded in unmodified cwlib; these
are now supported. Ordinary binary headers support heads through 951 and branch
0 or Leerdammer head 0x272, ID 0x4c44. Other heads/branches/custom resources are
unsupported. Older header boundaries and LD branch flag changes have synthetic
tests; those synthetic version combinations have not all been run through Java.

STATIC_MESH: common revision/dependency-offset/table parsed; its variable mesh
header is explicitly partial. Branch, compression flags, compressed and payload
offset remain null for these resources, rather than invented ordinary fields.

Texture/text/swizzled wrappers are magic-identified but payload-unparsed. Ordinary
opaque formats without a recognized wrapper remain unknown_magic, even if their
extension is familiar. A2 does not inspect GTF dimensions, DDS/PNG pixels, levels,
meshes, animations, shaders or audio payloads. Canonical uppercase magic is required;
Toolkit tolerates lowercase. This deliberate restriction avoids reading plain
lowercase text such as 'game' as an encrypted GAMe wrapper. Unknown signatures
receive only a four-byte probe.

Known resource 'e' markers, NPD, SCE and PS3 PKG signatures stop after four bytes.
No protected payload is passed to any oracle or decrypted. Zero protected markers
were found inside the selected clear FARC. This says nothing about DATA.SDAT/EBOOT,
which remain protected and outside these three prepared scanner files.

## Output and work budgets

Default scanner output is schema_version 2 / A2_resource_metadata. --tables-only
retains the A1 table-only mode, also using schema 2; legacy schema 1 artifacts remain
valid historical evidence. Per-location resource_inspections hold raw metadata.
Candidate inspection_index references avoid repeating dependency tables per alias.
Ambiguous locations are all retained; no precedence is selected. Only a unique
location supplies a row-level confirmed_type/header/dependency status.

All raw descriptor flag bytes, GUIDs, SHA-1s, type IDs, duplicate rows and null IDs
are retained. Unknown type IDs remain numeric, with known_type null. Unknown flag
bits produce unsupported without pretending a partial table is complete. Table
absence in older revisions differs from a parsed empty table.

dependency_links join raw references to original MAP row indices without recursion.
Zero GUID/empty hash is semantically null, matching Toolkit; raw values are still
retained. A GUID and hash disagreement is explicit, never silently substituted.
Catalog match is not proof that target bytes exist, match their hash, or implement
the specified type. Cycles cannot cause traversal loops because no recursive load
occurs. Reverse-reference UI is deferred to B.

Defaults: 1 MiB metadata per resource, 65,536 dependencies per resource, 256 MiB
total resource-metadata reads. --metadata-bytes, --dependency-limit and
--scan-read-budget control these independently of MAP/FARC/filesystem limits.
Readers seek only within the checked resource range; binary payloads are skipped.
The bounded worst-case dependency read may include uninterpreted bytes after the
table; custom trailing ALSR data is not processed. Header inspection coverage counts
distinct MAP hashes with at least four metadata bytes read, including unknown magic.
Status/type counts and raw dependency counts are per location, not per MAP alias.

Fatal table/source/publication failures return exit 1 without a published inventory.
Per-resource malformed, I/O or budget failures retain diagnostic JSON with
partial_metadata true and exit 2. Unknown/unsupported/protected/missing entries are
explicit outcomes and do not on their own mark the inventory partial. Exit 0 means
the catalog scan completed within the supported scope, not full asset compatibility.

## Verified local results

All 31,534 archive entries inspected; 1,462,918 metadata bytes read:

| Status | Locations |
|---|---:|
| metadata_parsed | 16,515 |
| metadata_partial_static_mesh | 21 |
| identified_unparsed | 13,862 |
| unknown_magic | 1,136 |

No malformed, unsupported, budget or I/O failures in the final selected scan.
116,739 raw dependency entries: 116,693 GUID catalog candidates, 46 unresolved in
this MAP. 4,333 distinct MAP hashes lack an inspected resource in this minimal input.
The catalog has 4,621 rows without any byte location; aliases make these different
counts. No additional extracted files are implied by these results.

Two root-form scans were byte-identical, 119,338,719 JSON bytes. Complete Toolkit
parity: all 36,854 MAP rows and 31,534 FARC entries; 24 clear binary samples across
22 actual heads (306–951), including static-mesh common fields, and 123 normalized
sample dependencies. This validates selected metadata, not every payload or type.

## Reproduce the oracle

Oracle library was built from the unmodified pinned source, not downloaded as an
unknown-provenance binary. Ignored local Temurin 21.0.12.1+1 and Maven 3.9.11 were
downloaded from official sources with SHA-256/SHA-512 checks. Package metadata/logs
remain local. Set an absolute Maven cache path; a first relative-path cache mistake
was corrected and the verified newly created misplaced cache removed.

```powershell
$env:JAVA_HOME = (Get-ChildItem local-analysis/toolchain/java -Directory | Select-Object -First 1).FullName
$env:PATH = "$env:JAVA_HOME/bin;$env:PATH"
$repoDir = Join-Path $PWD 'local-analysis/toolchain/maven/repository'
& ./local-analysis/toolchain/maven/apache-maven-3.9.11/bin/mvn.cmd "-Dmaven.repo.local=$repoDir" -f references/toolkit/pom.xml -pl lib/cwlib -am -DskipTests package
./scripts/run-toolkit-oracle.ps1 -Output local-analysis/oracle/toolkit-new.json
python scripts/verify_toolkit_oracle.py local-analysis/a2-inventory-verified.json local-analysis/oracle/toolkit-a2-expanded.json --repeat local-analysis/a2-inventory-repeat.json --output local-analysis/a2-comparison-new.json
```

The prepared sample manifest local-analysis/oracle/samples-a2.json combines the 19
original sampled hashes with one smallest archive candidate for each head 941–951;
only its 24 clear binary samples go through SerializedResource. Other samples are
not sent through that constructor. Harness validates the dependency offset/count,
bounds compressed sample input to 1 MiB and JVM heap to 512 MiB. Toolkit's plain
decompressor runs inside its normal constructor; no decrypted data, decoded pixels
or payload structure are exported. No save/import/game-execution APIs are called.

The oracle adapter checks source HEAD/clean tracked files. It exports selected
metadata to a new ignored filename. Comparison normalizes Toolkit's GUID-zero and
empty-hash null rules and HashSet deduplication. ResourceDescriptor equality ignores
type, so comparison retains first type per identity; Rust preserves every raw entry.
Unknown Toolkit type becomes INVALID=0, while Rust retains the actual raw number.
These normalization rules are explicit limitations, not permission to discard raw
inspector information. The harness reflects isCompressed because Toolkit has no
public getter for that wrapper field.

19 Rust tests pass (A1 plus six reader tests and two A2 CLI tests), including all
representative truncations, version boundaries, combined/null/unknown references,
encrypted stops, safe reads, mismatched hints, alias links, partial publication and
global budgets. fmt and Clippy pass. No cross-platform CI, fuzzing, full texture
oracle, pixel/mesh agreement or GUI validation is claimed.
