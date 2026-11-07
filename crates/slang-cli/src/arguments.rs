use slang_compiler::CompileOptions;
use std::{ffi::OsString, path::PathBuf};

pub const HELP: &str = "Native Rust Slang compiler (partial language compatibility)\n\nUsage: slangc-rs <file.slang> [options]\n\n  -target <name>       Output target (hlsl implemented)\n  -entry <name>        Require a compute entry point\n  -o <path>           Write output atomically (default: stdout)\n  -I <directory>      Add include search path (also -Idirectory)\n  -D <NAME[=value]>   Define macro (also -DNAME=value)\n  --check             Check without generating code\n  --dump-ir           Emit typed structured IR as JSON\n  --reflection <path> Write declaration reflection as JSON\n  --diagnostics-json  Emit compilation errors as JSON\n  --help, -h          Display this help\n  --version           Display version\n\nTargets spirv, dxil, glsl, wgsl, metal, cuda, cpp are tracked but unimplemented.\nUnsupported language constructs fail explicitly. See docs/compatibility.md.\n";

pub enum Action {
    Help,
    Version,
    Compile(Box<Arguments>),
}

pub struct Arguments {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub reflection: Option<PathBuf>,
    pub options: CompileOptions,
    pub dump_ir: bool,
    pub json_diagnostics: bool,
}

impl Arguments {
    pub fn parse(values: impl Iterator<Item = OsString>) -> Result<Action, String> {
        let mut values = values.peekable();
        let mut input = None;
        let mut output = None;
        let mut reflection = None;
        let mut options = CompileOptions::default();
        let mut dump_ir = false;
        let mut json_diagnostics = false;
        let mut positional = false;
        while let Some(value) = values.next() {
            let text = value.to_str();
            if positional {
                set_input(&mut input, value)?;
                continue;
            }
            match text {
                Some("--") => positional = true,
                Some("--help" | "-h") => return Ok(Action::Help),
                Some("--version") => return Ok(Action::Version),
                Some("-o") => output = Some(PathBuf::from(next(&mut values, "-o")?)),
                Some("--reflection") => {
                    reflection = Some(PathBuf::from(next(&mut values, "--reflection")?))
                }
                Some("-target") => {
                    options.target = next(&mut values, "-target")?
                        .to_str()
                        .ok_or("target must be UTF-8")?
                        .parse()?
                }
                Some("-entry") => {
                    options.entry_point = Some(
                        next(&mut values, "-entry")?
                            .into_string()
                            .map_err(|_| "entry point must be UTF-8")?,
                    )
                }
                Some("--check") => options.check_only = true,
                Some("--dump-ir") => {
                    dump_ir = true;
                    options.check_only = true;
                }
                Some("--diagnostics-json") => json_diagnostics = true,
                Some("-I") => options
                    .include_paths
                    .push(PathBuf::from(next(&mut values, "-I")?)),
                Some("-D") => define(
                    &mut options,
                    &next(&mut values, "-D")?
                        .into_string()
                        .map_err(|_| "macro must be UTF-8")?,
                )?,
                Some(text) if text.starts_with("-I") => {
                    options.include_paths.push(PathBuf::from(&text[2..]))
                }
                Some(text) if text.starts_with("-D") => define(&mut options, &text[2..])?,
                Some(text) if text.starts_with('-') => {
                    return Err(format!("unknown option '{text}'"));
                }
                _ => set_input(&mut input, value)?,
            }
        }
        let input = input.ok_or("input file is required")?;
        if options.check_only && !dump_ir && output.is_some() {
            return Err("--check cannot be combined with -o".into());
        }
        if output.as_ref().is_some_and(|p| p == &input)
            || reflection.as_ref().is_some_and(|p| p == &input)
        {
            return Err("output path must differ from input path".into());
        }
        if output.is_some() && output == reflection {
            return Err("code and reflection output paths must differ".into());
        }
        Ok(Action::Compile(Box::new(Self {
            input,
            output,
            reflection,
            options,
            dump_ir,
            json_diagnostics,
        })))
    }
}

fn next(values: &mut impl Iterator<Item = OsString>, option: &str) -> Result<OsString, String> {
    values
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{option} requires a value"))
}
fn set_input(input: &mut Option<PathBuf>, value: OsString) -> Result<(), String> {
    if input.replace(PathBuf::from(value)).is_some() {
        return Err("only one input file is supported".into());
    }
    Ok(())
}
fn define(options: &mut CompileOptions, text: &str) -> Result<(), String> {
    let (name, value) = text.split_once('=').unwrap_or((text, "1"));
    if name.is_empty()
        || !name
            .chars()
            .enumerate()
            .all(|(i, ch)| ch == '_' || ch.is_ascii_alphabetic() || i > 0 && ch.is_ascii_digit())
    {
        return Err("invalid macro name".into());
    }
    options.defines.insert(name.into(), value.into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Result<Action, String> {
        Arguments::parse(values.iter().map(OsString::from))
    }
    #[test]
    fn options_and_invalid_combinations() {
        assert!(matches!(args(&["--help"]), Ok(Action::Help)));
        let Ok(Action::Compile(a)) = args(&["a.slang", "-DN=4", "-I", "include", "-entry", "main"])
        else {
            panic!()
        };
        assert_eq!(a.options.defines["N"], "4");
        for values in [
            &["a", "-o"][..],
            &["a", "b"],
            &["a", "-what"],
            &["a", "--check", "-o", "out"],
            &["a", "-o", "a"],
            &["a", "-D9X"],
        ] {
            assert!(args(values).is_err(), "{values:?}");
        }
    }
}
