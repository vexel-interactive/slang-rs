use crate::{
    Program,
    graph::{ModuleId, ModuleInfo, Node},
    link,
};
use slang_lexer::TokenKind;
use slang_preprocessor::{IncludeResolver, Options};
use slang_source::{Diagnostic, Severity, SourceId, SourceMap, Span};
use slang_syntax::{FileInclude, Module};
use std::collections::{BTreeMap, BTreeSet};

/// Resolve and link a root source already owned by `sources`.
/// Imports use the same resolver's quoted-path search, with isolated macro state.
pub fn load(
    root: SourceId,
    sources: &mut SourceMap,
    resolver: &dyn IncludeResolver,
    options: &Options,
) -> Result<Program, Vec<Diagnostic>> {
    let mut loader = Loader {
        sources,
        resolver,
        options,
        nodes: vec![],
        paths: BTreeMap::new(),
        loading: BTreeSet::new(),
        members: BTreeMap::new(),
        order: vec![],
        identifiers: BTreeSet::new(),
        diagnostics: vec![],
    };
    loader.visit(root, Span::new(root, 0, 0), 0)?;
    let mut program = link::link(loader.nodes, &loader.order, &loader.identifiers)?;
    program.diagnostics = loader.diagnostics;
    Ok(program)
}

struct Loader<'a> {
    sources: &'a mut SourceMap,
    resolver: &'a dyn IncludeResolver,
    options: &'a Options,
    nodes: Vec<Node>,
    paths: BTreeMap<String, ModuleId>,
    loading: BTreeSet<ModuleId>,
    members: BTreeMap<String, ModuleId>,
    order: Vec<ModuleId>,
    identifiers: BTreeSet<String>,
    diagnostics: Vec<Diagnostic>,
}

impl Loader<'_> {
    fn parse(&mut self, source: SourceId) -> Result<Module, Vec<Diagnostic>> {
        let processed =
            slang_preprocessor::preprocess(self.sources, source, self.resolver, self.options);
        if processed
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            return Err(processed.diagnostics);
        }
        self.diagnostics.extend(processed.diagnostics);
        self.identifiers.extend(
            processed
                .tokens
                .iter()
                .filter(|t| t.kind == TokenKind::Identifier)
                .map(|t| t.text.clone()),
        );
        slang_syntax::parse(&processed.tokens)
    }

    fn fragment(
        &mut self,
        module: ModuleId,
        include: FileInclude,
        depth: usize,
    ) -> Result<(), Vec<Diagnostic>> {
        let from = &self.sources.get(include.span.source).unwrap().name;
        let (path, text) = self
            .resolver
            .resolve(from, &include.path, false)
            .map_err(|error| {
                vec![Diagnostic::error(
                    "M001",
                    include.span,
                    format!("cannot load module member '{}': {error}", include.path),
                )]
            })?;
        if let Some(owner) = self.members.get(&path) {
            if *owner == module {
                return Ok(());
            }
            return Err(vec![Diagnostic::error(
                "M003",
                include.span,
                "source file belongs to a different module",
            )]);
        }
        if depth >= 32 || self.members.len() >= 1024 {
            return Err(vec![Diagnostic::error(
                "M002",
                include.span,
                "module member graph limit exceeded",
            )]);
        }
        let source = self.sources.add(path.clone(), text);
        let mut fragment = self.parse(source)?;
        let name = &self.nodes[module.0].info.name;
        if fragment.implementation.as_deref() != Some(name.replace('_', "-").as_str()) {
            return Err(vec![Diagnostic::error(
                "M003",
                include.span,
                format!("module member must declare 'implementing {name};'"),
            )]);
        }
        self.members.insert(path, module);
        self.nodes[module.0].info.files.push(source);
        for include in fragment.file_includes {
            self.fragment(module, include, depth + 1)?;
        }
        self.nodes[module.0]
            .syntax
            .imports
            .append(&mut fragment.imports);
        self.nodes[module.0]
            .syntax
            .declarations
            .append(&mut fragment.declarations);
        self.nodes[module.0]
            .syntax
            .constant_buffers
            .append(&mut fragment.constant_buffers);
        self.nodes[module.0]
            .syntax
            .static_members
            .append(&mut fragment.static_members);
        self.nodes[module.0]
            .syntax
            .nested_types
            .append(&mut fragment.nested_types);
        Ok(())
    }

    fn visit(
        &mut self,
        source: SourceId,
        import_span: Span,
        depth: usize,
    ) -> Result<ModuleId, Vec<Diagnostic>> {
        let path = self.sources.get(source).unwrap().name.clone();
        if let Some(id) = self.paths.get(&path).copied() {
            if self.loading.contains(&id) {
                return Err(vec![Diagnostic::error(
                    "M002",
                    import_span,
                    format!("cyclic import of '{path}'"),
                )]);
            }
            return Ok(id);
        }
        if depth >= 32 || self.nodes.len() >= 1024 || self.members.len() >= 1024 {
            return Err(vec![Diagnostic::error(
                "M002",
                import_span,
                "module graph limit exceeded",
            )]);
        }
        if self.members.contains_key(&path) {
            return Err(vec![Diagnostic::error(
                "M003",
                import_span,
                "a module member cannot be imported as a primary module",
            )]);
        }
        let mut syntax = self.parse(source)?;
        if syntax.implementation.is_some() {
            return Err(vec![Diagnostic::error(
                "M003",
                import_span,
                "an implementing file cannot be imported as a primary module",
            )]);
        }
        let legacy_visibility = syntax.uses_legacy_visibility();
        syntax.apply_legacy_visibility();
        let includes = std::mem::take(&mut syntax.file_includes);
        let id = ModuleId(self.nodes.len());
        let name = syntax.name.clone().unwrap_or_else(|| {
            path.rsplit(['/', '\\'])
                .next()
                .unwrap_or(&path)
                .trim_end_matches(".slang")
                .into()
        });
        self.nodes.push(Node {
            info: ModuleInfo {
                id,
                name,
                path: path.clone(),
                source,
                legacy_visibility,
                files: vec![source],
                imports: vec![],
            },
            syntax,
            dependencies: vec![],
        });
        self.paths.insert(path.clone(), id);
        self.members.insert(path, id);
        self.loading.insert(id);
        for include in includes {
            self.fragment(id, include, 0)?;
        }
        let imports = self.nodes[id.0].syntax.imports.clone();
        for import in imports {
            let from = &self.sources.get(import.span.source).unwrap().name;
            let (path, text) =
                self.resolver
                    .resolve(from, &import.path, false)
                    .map_err(|error| {
                        vec![Diagnostic::error(
                            "M001",
                            import.span,
                            format!("cannot load module '{}': {error}", import.path),
                        )]
                    })?;
            let dependency = if let Some(dependency) = self.paths.get(&path).copied() {
                if self.loading.contains(&dependency) {
                    return Err(vec![Diagnostic::error(
                        "M002",
                        import.span,
                        format!("cyclic import of '{path}'"),
                    )]);
                }
                dependency
            } else {
                let source = self.sources.add(path, text);
                self.visit(source, import.span, depth + 1)?
            };
            self.nodes[id.0]
                .dependencies
                .push((dependency, import.exported));
            if !self.nodes[id.0].info.imports.contains(&dependency) {
                self.nodes[id.0].info.imports.push(dependency);
            }
        }
        self.loading.remove(&id);
        self.order.push(id);
        Ok(id)
    }
}
