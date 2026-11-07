# Compatibility matrix

Statuses describe implemented behavior. **Partial** means working code with
explicit limits and regression tests. **Not ported** means the equivalent
subsystem is absent. No subsystem currently claims complete upstream parity.

| Slang subsystem | Rust crate | Status | Evidence or remaining work |
| --- | --- | --- | --- |
| Source manager and diagnostics | slang-source | Partial | UTF-8, CRLF, spans; upstream diagnostic IDs absent |
| Lexer | slang-lexer | Partial | Lossless tokens; incomplete literal grammar |
| Preprocessor | slang-preprocessor | Partial | Macros, #, ##, variadics, includes, conditionals; no predefined macros or #line |
| Parser and AST | slang-syntax | Partial | Functions, globals, structs, expressions, casts, loops, switches |
| Name and type checking | slang-sema | Partial | Scalars, vectors, matrices, scopes, overloads, writable locations |
| Core module | slang-sema | Partial | Small built-in signature set |
| Texture and sampler operations | slang-sema, slang-codegen | Partial | 1D/2D/3D/cube, indexed access, load/sample/comparison/dimensions; advanced resources absent |
| Modules and imports | ? | Not ported | Loader, visibility, dependency graph required |
| Interfaces and generics | ? | Not ported | Constraints, witnesses, specialization required |
| Capability system | ? | Not ported | Profiles and target requirements required |
| Automatic differentiation | ? | Not ported | AD passes, JVP/VJP, runtime contracts required |
| Structured IR | slang-ir | Partial | Typed tree lowering |
| SSA, linking, optimization | ? | Not ported | CFG, use-def, verifier, passes required |
| HLSL emitter | slang-codegen | Partial | Compute, vertex, fragment subset; examples validated by DXC |
| SPIR-V emitter | ? | Not ported | Native binary emission and spirv-val required |
| DXIL emitter | ? | Not ported | Textual HLSL output does not implement a native DXIL backend |
| GLSL, WGSL, Metal | ? | Not ported | Target-specific lowering and validators required |
| CUDA, C++, CPU targets | ? | Not ported | ABI lowering and runtime required |
| Resource binding and layout | ? | Not ported | Target API layout rules required |
| Declaration reflection | slang-compiler | Partial | Names, types, stages, group sizes |
| Complete reflection API | ? | Not ported | Bindings, layouts, specialization absent |
| Native Rust API | slang-compiler | Partial | Own API for the implemented pipeline |
| Upstream C ABI and API | ? | Not ported | API/ABI compatibility suite required |
| slangc CLI | slang-cli | Partial | Limited option set |
| Language server | ? | Not ported | Incremental model and LSP transport required |
| Repro, serialization, caches | ? | Not ported | Stable formats and versioning required |
| Upstream test coverage | tools, tests/conformance | Partial | Source inventory and a small separate differential corpus |

## Full rewrite completion criteria

1. Port all compiler-facing declarations and subsystems in the pinned baseline.
2. Classify and run the full upstream corpus, with documented platform skips.
   A test is not passed merely because its source parses.
3. Validate every supported target and compare behavior: shader execution,
   layout, reflection, diagnostics, and specialization.
4. Pass API/ABI tests for applications that consume `slang.h`.
5. Build and test on target operating systems without an upstream compiler in
   the production pipeline.
6. Document verified behavior for all implemented features.

These criteria are not yet met. Crate counts, renamed files, and source
inventories are not evidence of language or API parity.
