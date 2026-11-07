# Full porting plan

## Upstream baseline

Repository: `shader-slang/slang`, branch `master`, commit
`ae4e57957a0adad3061b069b0e0eec8217779338`.

| Area | Files | C/C++/Slang lines |
| --- | ---: | ---: |
| source/slang | 585 | 483772 |
| source/core | 137 | 39670 |
| source/compiler-core | 109 | 36001 |
| include | 7 | 11574 |
| tests | 6723 | 287740 |
| tools | 775 | 173027 |

These figures describe source size, not completion percentages. The generated
inventory records relative paths, SHA-256 digests, line counts, and areas for
core source files, headers, and tests. The original checkout is under
`.upstream/`, excluded from builds and version control. External fixtures and
upstream dependencies need separate inventory before porting their subsystems.

```text
python tools/upstream_inventory.py --source .upstream/slang --revision ae4e57957a0adad3061b069b0e0eec8217779338
```

## Stages

| Stage | Work | Verification |
| --- | --- | --- |
| 1. Frontend | Literal grammar, macro operators, grammar, diagnostics | Differential tests for each grammar category |
| 2. Semantic model | Complete types, conversions, scopes, overload ranking, constants | Positive and negative upstream semantic tests |
| 3. Modules | Imports, exports, access control, dependency graph, sessions | Multi-module builds and cyclic import diagnostics |
| 4. Generics and interfaces | Constraints, associated types, witnesses, specialization | Static/runtime dispatch and link-time specialization |
| 5. Core module | Types, resources, intrinsics, capabilities | Ported standard library with documented overloads |
| 6. SSA IR | CFG, SSA, verifier, use-def, dominance, canonicalization | IR invariants and pass equivalence |
| 7. GPU targets | HLSL, SPIR-V, DXIL, GLSL, MSL, WGSL, shader stages | Validators and hardware/CTS execution |
| 8. Layout and reflection | Register spaces, binding policy, buffer layout, API | Golden layouts for each target and profile |
| 9. Autodiff | Forward/reverse differentiation and contracts | Upstream derivatives and finite differences |
| 10. Host targets | CUDA, C++, CPU, libraries, runtime, ABI | Integration and execution tests |
| 11. Tooling and API | C ABI, CLI parity, LSP, repro, cache, serialization | Existing clients and complete option corpus |
| 12. Release parity | Full corpus, platform matrix, documentation, performance | All criteria in compatibility.md |

Develop each stage in small vertical changes. Every feature needs a source
fixture, expected upstream behavior, and an updated compatibility status.
Calling the upstream compiler does not implement a Rust backend.

## Responsibility mapping

| Upstream family | Rust responsibility |
| --- | --- |
| source/compiler-core/slang-lexer*, slang-preprocessor* | slang-lexer, slang-preprocessor |
| source/slang/slang-parser*, slang-syntax*, slang-ast-* | slang-syntax |
| source/slang/slang-parameter-binding*, slang-type-layout* | slang-layout, slang-sema, slang-compiler |
| Source module loading and lexical linking | slang-module |
| source/slang/slang-check-* | slang-sema |
| source/slang/slang-ir-* | slang-ir |
| source/slang/slang-emit-* | slang-codegen |
| slang-compiler*, slang-session*, slang-api*, slang-reflection* | slang-compiler and future ABI layer |
| tools/slangc and language-server tools | slang-cli and future LSP crate |

This maps responsibilities rather than individual files. Rust modules should
follow coherent data models and compilation phases instead of mechanically
renaming C++ files.
