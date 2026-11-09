# Modules and access control

The native loader supports source modules within one compilation. It uses
the caller's source resolver and links their declarations before type checking.
Production builds do not invoke the upstream compiler.

## Primary files and imports

```slang
// math-utils.slang
module math_utils;

float helper(float value)
{
    return value * 2.0;
}

public float twice(float value)
{
    return helper(value);
}
```

```slang
// main.slang
import math_utils;

float run()
{
    return twice(2.0);
}
```

An import resolves a primary `.slang` file. Identifier requests translate
underscores to hyphens and dots to directory separators:
`import folder.math_utils;` requests `folder/math-utils.slang`.
Quoted requests preserve spelling and accept an optional `.slang` extension.
A `module` declaration accepts a single identifier, string, or omitted name;
it must precede imports and declarations. An omitted name uses the file name.

Filesystem requests search beside the requesting file, then the configured
`-I` paths. Virtual resolvers supply a stable file identity. Each primary file
loads once per compilation, including duplicate and diamond imports. Cyclic
imports fail with `M002`; missing sources fail with `M001`.

Imports expose public declarations. Own-module declarations take precedence.
Normal imports do not expose a dependency's imports to another importer;
`__exported import math_utils;` reexports its public declarations.

## Member files

```slang
// math-utils.slang
module math_utils;
__include "math-utils/scale";

public float twice(float value)
{
    return scale(value);
}
```

```slang
// math-utils/scale.slang
implementing math_utils;

float scale(float value)
{
    return value * 2.0;
}
```

`__include` adds a file to the same module. It must declare a matching
`implementing` name. Members can refer to all declarations in their module,
including later members. Repeated and circular member inclusions load a file
once. A primary file can include itself with no effect. A member cannot belong
to two modules or be imported as a primary file; these errors use `M003`.

Each primary and member file is preprocessed independently. Command-line
definitions apply to every file, while source-local macros and `#pragma once`
state do not propagate between them. Preprocessor `#include` remains textual
inclusion into the current file's preprocessing state.

See [the multi-file example](../examples/modules/main.slang).

## Visibility

`public` declarations are accessible across modules. `internal` declarations
are accessible throughout their own module. Modern source modules use internal
visibility by default, including fields. A public struct needs explicitly
public fields for callers to access them:

```slang
public struct Pair
{
    public float left;
    public float right;
    internal float cachedValue;
};
```

Public functions, type aliases, fields, and globals cannot expose internal struct types,
including through arrays and resources. A public field requires a public
struct. Conflicting access modifiers fail with `T018`. Visibility applies to
each overload: an internal overload is unavailable to importers even if a
public overload has the same name. Linking preserves parameter and local
shadowing, including loop and switch scopes.

An imported module cannot access the root source's declarations without an
import. This also applies during alias resolution and struct field checking.
Builtin calls remain distinct from same-name functions in other modules;
generated HLSL cannot accidentally select those user functions.

A legacy source with no module declaration, no `__include`, and no access
modifiers exports all its declarations and fields. Reflection records whether
this compatibility mode was used.

Struct and alias members use qualified names such as `Outer.Inner` and
`Outer.Number`. Importing `Outer` does not import its nested short names.
Each selected type requires public access from an importing module. Module
member files retain nested types and static storage when linked into their owner.

## Current limits

This is source linking rather than the upstream session/component API.
Separate compilation, serialized modules, incremental caches, namespaces,
private members, generic/extension methods, and imported shader entries remain
unported. Combining same-name overload sets from different modules currently
reports `M004`; overload ranking still uses exact matches followed by equal
conversion costs.

Language-version directives and version 2026's inherited aggregate-field
visibility are not implemented. Explicit public fields match the default
behavior observed with the `v2026.21` oracle. Member verification currently
compares normalized names; advanced module name/path alias rules remain absent.

Import depth and member depth are each limited to 32. A compilation accepts at
most 1024 modules and 1024 primary/member file identities. Excessive graphs
produce `M002`, including in the Windows debug CLI.

Reflection schema 4 introduced the graph and original global names. Imported
struct type strings still expose canonical linked names. Schema 7 records
explicit and automatic resource ranges, allocating root resources before
imported resources. Complete GPU layouts remain unimplemented.
