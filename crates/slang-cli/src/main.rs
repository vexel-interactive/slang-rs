mod arguments;
mod output;

use arguments::{Action, Arguments};
use slang_compiler::compile_file;
use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

fn main() -> ExitCode {
    let arguments = match Arguments::parse(env::args_os().skip(1)) {
        Ok(Action::Help) => {
            print!("{}", arguments::HELP);
            return ExitCode::SUCCESS;
        }
        Ok(Action::Version) => {
            println!(
                "slangc-rs {} (native Rust; partial Slang compatibility)",
                env!("CARGO_PKG_VERSION")
            );
            return ExitCode::SUCCESS;
        }
        Ok(Action::Compile(arguments)) => arguments,
        Err(message) => {
            eprintln!("{message}\nRun slangc-rs --help for usage.");
            return ExitCode::from(2);
        }
    };
    match run(*arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Arguments) -> io::Result<()> {
    let compilation = match compile_file(&arguments.input, &arguments.options) {
        Ok(compilation) => compilation,
        Err(error) => {
            if arguments.json_diagnostics {
                let json =
                    serde_json::to_string_pretty(&error.diagnostics).map_err(io::Error::other)?;
                return Err(io::Error::other(json));
            }
            return Err(io::Error::other(error.to_string()));
        }
    };
    for diagnostic in &compilation.diagnostics {
        eprintln!("{}", diagnostic.render(&compilation.sources));
    }
    for path in [&arguments.output, &arguments.reflection]
        .into_iter()
        .flatten()
    {
        output::validate_destination(path, &compilation.sources)?;
    }
    if let (Some(code), Some(reflection)) = (&arguments.output, &arguments.reflection)
        && output::resolved_path(code)? == output::resolved_path(reflection)?
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "code and reflection output paths must differ",
        ));
    }
    let content = if arguments.dump_ir {
        Some(serde_json::to_string_pretty(&compilation.ir).map_err(io::Error::other)?)
    } else {
        compilation.artifact
    };
    let reflection = arguments
        .reflection
        .as_ref()
        .map(|_| serde_json::to_vec_pretty(&compilation.reflection))
        .transpose()
        .map_err(io::Error::other)?;
    if let Some(content) = content {
        if let Some(path) = &arguments.output {
            output::atomic_write(path, content.as_bytes())?;
        } else {
            io::stdout().lock().write_all(content.as_bytes())?;
        }
    }
    if let (Some(path), Some(bytes)) = (&arguments.reflection, reflection) {
        output::atomic_write(path, &bytes)?;
    }
    Ok(())
}
